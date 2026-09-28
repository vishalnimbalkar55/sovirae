//! The single playback-session owner (spec §5.1, §8.4, §10).
//!
//! Every entry path sends commands here. A dedicated thread owns the
//! document index, the synthesis schedule, and the native player; the UI only
//! receives snapshots.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use speakit_audio::{Player, QueuedSegment};
use speakit_core::text::truncate_to_limit;
use speakit_core::{
    validate, PlaybackSnapshot, PlaybackStatus, SourceKind, SourceReference, SpeakRequest,
    SpeechDocument, TextError, MAX_TEXT_UTF16,
};
use speakit_tts::{CancelToken, Engine, Pcm, TtsError, VoiceInfo};

use crate::settings::ResourceProfile;

/// Used for duration estimates until real segment lengths are known.
const DEFAULT_MS_PER_CHAR: f64 = 62.0;
/// Steady-state snapshot rate while playing (≤ 4 Hz, spec §13.3).
const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(250);
/// A finished reading stays visible briefly before the player hides.
const COMPLETED_LINGER: Duration = Duration::from_secs(4);

#[derive(Debug)]
pub enum Command {
    /// `reply`, when present, receives the outcome instead of a player
    /// notice: the new session ID, or why the request was rejected.
    Speak { request: SpeakRequest, truncate: bool, reply: Option<Sender<SpeakOutcome>> },
    TogglePause,
    Play,
    Pause,
    Stop,
    Skip { delta_ms: i64, snap: bool },
    SeekSegment(usize),
    SeekFraction(f64),
    SetRate(f32),
    SetVolume(f32),
    SetVoice(Option<String>),
    /// Prefer a voice in the page's language when one exists (Chrome).
    SetMatchLanguage(bool),
    SetProfile(ResourceProfile),
    Preview(String),
    Shutdown,
}

pub type SpeakOutcome = Result<u64, Notice>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub code: &'static str,
    pub message: String,
    /// Optional recovery action the UI can offer, e.g. `readFirstPart`.
    pub action: Option<&'static str>,
}

impl Notice {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), action: None }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentView {
    pub session_id: u64,
    pub text: String,
    /// (segment id, sentence id, normalized byte start, end, paragraph start)
    pub segments: Vec<(usize, usize, usize, usize, bool)>,
}

/// Downsampled amplitude of one synthesized segment for the waveform
/// (spec §10.5), computed off the audio thread.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    pub session_id: u64,
    pub segment: usize,
    /// Source-time duration of the segment.
    pub duration_ms: u64,
    /// Peak levels, 0–255, at `ENVELOPE_HZ` points per second of source audio.
    pub levels: Vec<u8>,
}

fn envelope_levels(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let hop = (sample_rate / speakit_audio::ENVELOPE_HZ).max(1) as usize;
    samples
        .chunks(hop)
        .map(|c| {
            let rms = (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt();
            // Perceptual-ish scaling so quiet speech is still visible.
            ((rms * 4.0).sqrt().min(1.0) * 255.0) as u8
        })
        .collect()
}

/// Callbacks from the controller into the desktop shell.
pub trait Host: Send + 'static {
    fn snapshot(&self, snapshot: &PlaybackSnapshot);
    fn notice(&self, notice: &Notice);
    fn document(&self, document: &DocumentView);
    fn envelope(&self, envelope: &Envelope);
    fn show_player(&self);
    fn hide_player(&self);
    fn playback_active(&self, active: bool);
}

#[derive(Clone)]
pub struct Controller {
    tx: Sender<Msg>,
}

impl Controller {
    pub fn send(&self, command: Command) {
        let _ = self.tx.send(Msg::Cmd(command));
    }
}

enum Msg {
    Cmd(Command),
    Synth(SynthDone),
}

/// Most synthesis jobs run at once, whatever the engine allows.
const MAX_PARALLEL_JOBS: usize = 3;

struct SynthJob {
    job: u64,
    generation: u64,
    segment: usize,
    text: String,
    voice: String,
    sample_rate: u32,
    cancel: CancelToken,
}

struct SynthDone {
    job: u64,
    generation: u64,
    segment: usize,
    result: Result<Pcm, TtsError>,
    elapsed: Duration,
}

enum Seg {
    Pending,
    Ready(Arc<[f32]>),
    /// PCM released under memory pressure; length kept for timing.
    Evicted(u64),
    Failed,
}

impl Seg {
    fn samples(&self) -> Option<u64> {
        match self {
            Seg::Ready(s) => Some(s.len() as u64),
            Seg::Evicted(n) => Some(*n),
            Seg::Failed => Some(0),
            Seg::Pending => None,
        }
    }
}

struct Session {
    id: u64,
    doc: SpeechDocument,
    source: SourceReference,
    segs: Vec<Seg>,
    chars: Vec<usize>,
    voice: VoiceInfo,
    queued_upto: usize,
    start_offset: usize,
    end_sent: bool,
    status: PlaybackStatus,
    user_paused: bool,
    is_preview: bool,
    pos: (usize, usize),
    message: Option<String>,
    completed_at: Option<Instant>,
}

pub struct Config {
    pub match_language: bool,
    pub rate: f32,
    pub volume: f32,
    pub voice: Option<String>,
    pub profile: ResourceProfile,
}

pub fn spawn<H: Host>(engine: Arc<dyn Engine>, config: Config, host: H) -> Controller {
    let (tx, rx) = mpsc::channel();
    let (job_tx, job_rx) = mpsc::channel::<SynthJob>();

    // One thread per job the engine may run at once; `schedule` never has
    // more jobs in flight than the engine allows.
    let job_rx = Arc::new(Mutex::new(job_rx));
    for n in 0..MAX_PARALLEL_JOBS {
        let synth_engine = engine.clone();
        let synth_tx = tx.clone();
        let job_rx = job_rx.clone();
        std::thread::Builder::new()
            .name(format!("sovirae-synth-{n}"))
            .spawn(move || loop {
                let Ok(job) = job_rx.lock().unwrap().recv() else { break };
                let started = Instant::now();
                let result =
                    synth_engine.synthesize(&job.text, &job.voice, job.sample_rate, &job.cancel);
                let done = SynthDone {
                    job: job.job,
                    generation: job.generation,
                    segment: job.segment,
                    result,
                    elapsed: started.elapsed(),
                };
                if synth_tx.send(Msg::Synth(done)).is_err() {
                    break;
                }
            })
            .expect("spawn synthesis thread");
    }

    let worker = Worker {
        engine,
        job_tx,
        host,
        player: None,
        idle_since: None,
        session: None,
        stash: None,
        next_id: 1,
        generation: 0,
        in_flight: Vec::new(),
        next_job: 1,
        rate: config.rate,
        volume: config.volume,
        voice: config.voice,
        match_language: config.match_language,
        profile: config.profile,
        sample_rate: 48_000,
        active: false,
        last_emit: Instant::now(),
        last_status: PlaybackStatus::Idle,
        slow_segments: 0,
    };
    std::thread::Builder::new()
        .name("sovirae-session".into())
        .spawn(move || worker.run(rx))
        .expect("spawn session thread");
    Controller { tx }
}

struct Worker<H: Host> {
    engine: Arc<dyn Engine>,
    job_tx: Sender<SynthJob>,
    host: H,
    player: Option<Player>,
    idle_since: Option<Instant>,
    session: Option<Session>,
    /// A reading paused while a voice preview plays.
    stash: Option<Session>,
    next_id: u64,
    generation: u64,
    /// (job, segment, cancel) for each synthesis job of this generation
    /// still running; cleared whenever the generation changes.
    in_flight: Vec<(u64, usize, CancelToken)>,
    next_job: u64,
    rate: f32,
    volume: f32,
    voice: Option<String>,
    match_language: bool,
    profile: ResourceProfile,
    sample_rate: u32,
    active: bool,
    last_emit: Instant,
    last_status: PlaybackStatus,
    slow_segments: u32,
}

impl<H: Host> Worker<H> {
    fn run(mut self, rx: Receiver<Msg>) {
        loop {
            let timeout = if self.session.is_some() { 40 } else { 500 };
            match rx.recv_timeout(Duration::from_millis(timeout)) {
                Ok(Msg::Cmd(Command::Shutdown)) => {
                    self.stop_all();
                    self.player = None;
                    return;
                }
                Ok(Msg::Cmd(c)) => self.handle(c),
                Ok(Msg::Synth(d)) => self.on_synth(d),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.tick();
        }
    }

    fn handle(&mut self, c: Command) {
        match c {
            Command::Speak { request, truncate, reply } => self.speak(request, truncate, reply),
            Command::Preview(voice) => self.preview(&voice),
            Command::TogglePause => {
                let paused = self.session.as_ref().map(|s| s.user_paused);
                match paused {
                    Some(true) => self.play(),
                    Some(false) => {
                        if matches!(self.session.as_ref().map(|s| s.status), Some(PlaybackStatus::Completed)) {
                            self.play()
                        } else {
                            self.pause()
                        }
                    }
                    None => {}
                }
            }
            Command::Play => self.play(),
            Command::Pause => self.pause(),
            Command::Stop => self.stop(),
            Command::Skip { delta_ms, snap } => self.skip(delta_ms, snap),
            Command::SeekSegment(i) => {
                if self.session.as_ref().is_some_and(|s| i < s.segs.len()) {
                    self.seek(i, 0);
                }
            }
            Command::SeekFraction(f) => {
                if let Some(s) = &self.session {
                    let total = self.duration_samples(s).0 as f64;
                    let target = (f.clamp(0.0, 1.0) * total) as u64;
                    let (seg, off) = self.locate_samples(s, target);
                    self.seek(seg, off);
                }
            }
            Command::SetRate(r) => {
                self.rate = r;
                if let Some(p) = &self.player {
                    p.set_rate(r);
                }
                self.slow_segments = 0;
                self.emit(true);
            }
            Command::SetVolume(v) => {
                self.volume = v;
                if let Some(p) = &self.player {
                    p.set_volume(v);
                }
                self.emit(true);
            }
            // Applies to the next reading; the current one keeps its voice.
            Command::SetVoice(v) => self.voice = v,
            Command::SetMatchLanguage(m) => self.match_language = m,
            Command::SetProfile(p) => self.profile = p,
            Command::Shutdown => {}
        }
    }

    fn resolve_voice(&self, wanted: Option<&str>) -> Option<VoiceInfo> {
        let voices = self.engine.voices().ok()?;
        wanted
            .and_then(|w| voices.iter().find(|v| v.id == w).cloned())
            .or_else(|| voices.iter().find(|v| v.recommended).cloned())
            .or_else(|| voices.into_iter().next())
    }

    fn ensure_player(&mut self) -> bool {
        if self.player.is_none() {
            match Player::new() {
                Ok(p) => {
                    p.set_rate(self.rate);
                    p.set_volume(self.volume);
                    self.sample_rate = p.sample_rate();
                    self.player = Some(p);
                }
                Err(e) => {
                    log::error!("audio output failed: {e}");
                    self.host.notice(&Notice::new(
                        "AUDIO_DEVICE",
                        "Audio device disconnected. Choose an output device.",
                    ));
                    return false;
                }
            }
        }
        self.idle_since = None;
        true
    }

    fn speak(&mut self, request: SpeakRequest, truncate: bool, reply: Option<Sender<SpeakOutcome>>) {
        // Callers with a reply channel (the Chrome bridge) show their own
        // errors; everyone else gets a player notice.
        let reject = |host: &H, notice: Notice| match &reply {
            Some(tx) => {
                let _ = tx.send(Err(notice));
            }
            None => host.notice(&notice),
        };
        let mut text = request.text;
        // Rejected input never stops a working reading (spec §10.1).
        match validate(&text) {
            Ok(()) => {}
            Err(TextError::NoText) => {
                let message = match request.source.kind {
                    SourceKind::Clipboard => "Clipboard is empty. Copy some text and try again.",
                    SourceKind::Selection => "Select text first, or copy it and use Speak clipboard.",
                    _ => "There is no text to read.",
                };
                reject(&self.host, Notice::new("NO_TEXT", message));
                return;
            }
            Err(TextError::TextTooLong { .. }) if truncate => {
                text = truncate_to_limit(&text, MAX_TEXT_UTF16).to_string();
            }
            Err(TextError::TextTooLong { .. }) => {
                let mut n = Notice::new(
                    "TEXT_TOO_LONG",
                    format!(
                        "This text is longer than {} characters. Read the first part?",
                        group(MAX_TEXT_UTF16)
                    ),
                );
                if request.source.kind == SourceKind::Clipboard {
                    n.action = Some("readFirstPart");
                }
                reject(&self.host, n);
                return;
            }
        }
        let Some(mut voice) = self.resolve_voice(self.voice.as_deref()) else {
            reject(&self.host, Notice::new("NO_VOICE", "Choose or download a voice to start reading."));
            return;
        };
        if self.match_language {
            if let Some(hint) = request.language_hint.as_deref() {
                if let Some(better) = self.voice_for_language(&voice, hint) {
                    voice = better;
                }
            }
        }
        let doc = SpeechDocument::new(text, self.engine.max_segment_chars());
        if doc.segments.is_empty() {
            reject(&self.host, Notice::new("NO_TEXT", "There is nothing readable in this text."));
            return;
        }
        if !self.ensure_player() {
            if let Some(tx) = &reply {
                let _ = tx.send(Err(Notice::new("AUDIO_DEVICE", "No audio output device is available.")));
            }
            return;
        }
        self.stash = None;
        self.start_session(doc, request.source, voice, false);
        if let (Some(tx), Some(s)) = (&reply, &self.session) {
            let _ = tx.send(Ok(s.id));
        }
    }

    /// A voice of the same model in the page's language, keeping the
    /// gender when possible. `None` when the chosen voice already fits or no
    /// voice speaks that language.
    fn voice_for_language(&self, chosen: &VoiceInfo, hint: &str) -> Option<VoiceInfo> {
        let primary = |tag: &str| tag.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
        let want = primary(hint);
        if want.is_empty() || primary(&chosen.language) == want {
            return None;
        }
        let voices = self.engine.voices().ok()?;
        let same_model: Vec<&VoiceInfo> =
            voices.iter().filter(|v| v.model == chosen.model && primary(&v.language) == want).collect();
        let exact = |v: &&&VoiceInfo| v.language.eq_ignore_ascii_case(hint);
        same_model
            .iter()
            .filter(exact)
            .find(|v| v.gender == chosen.gender)
            .or_else(|| same_model.iter().find(|v| v.gender == chosen.gender))
            .or_else(|| same_model.iter().filter(exact).next())
            .or_else(|| same_model.first())
            .map(|v| (*v).clone())
    }

    fn preview(&mut self, voice_id: &str) {
        let Some(voice) = self.resolve_voice(Some(voice_id)).filter(|v| v.id == voice_id) else {
            self.host.notice(&Notice::new("NO_VOICE", "That voice is no longer available."));
            return;
        };
        if !self.ensure_player() {
            return;
        }
        // Pause the current reading and keep it to restore afterwards.
        if let Some(mut current) = self.session.take() {
            if current.is_preview {
                current = match self.stash.take() {
                    Some(s) => s,
                    None => current,
                };
            }
            if !current.is_preview {
                self.refresh_position(&mut current);
                current.user_paused = true;
                self.stash = Some(current);
            }
        }
        let base = voice.name.split(" (").next().unwrap_or(&voice.name).to_string();
        let text = format!("Hello, I'm {base}. This is how I sound when reading your text aloud.");
        let doc = SpeechDocument::new(text, self.engine.max_segment_chars());
        let source = SourceReference { kind: SourceKind::Manual, display_name: Some("Voice preview".into()) };
        self.start_session(doc, source, voice, true);
    }

    fn start_session(&mut self, doc: SpeechDocument, source: SourceReference, voice: VoiceInfo, preview: bool) {
        self.cancel_in_flight();
        self.generation += 1;
        let player = self.player.as_ref().expect("player");
        player.clear();
        player.set_paused(false);
        let n = doc.segments.len();
        let chars = (0..n).map(|i| doc.segment_text(i).chars().count()).collect();
        let id = self.next_id;
        self.next_id += 1;
        self.slow_segments = 0;
        self.session = Some(Session {
            id,
            segs: (0..n).map(|_| Seg::Pending).collect(),
            chars,
            doc,
            source,
            voice,
            queued_upto: 0,
            start_offset: 0,
            end_sent: false,
            status: PlaybackStatus::Buffering,
            user_paused: false,
            is_preview: preview,
            pos: (0, 0),
            message: None,
            completed_at: None,
        });
        self.publish_document();
        self.host.show_player();
        self.schedule();
        self.emit(true);
    }

    fn publish_document(&self) {
        if let Some(s) = &self.session {
            let text = s.doc.normalized_text().to_string();
            let segments = s
                .doc
                .segments
                .iter()
                .map(|g| (g.id, g.sentence_id, g.normalized.start, g.normalized.end, g.paragraph_start))
                .collect();
            self.host.document(&DocumentView { session_id: s.id, text, segments });
        }
    }

    fn play(&mut self) {
        let Some(s) = self.session.as_mut() else { return };
        if s.status == PlaybackStatus::Completed {
            s.user_paused = false;
            s.completed_at = None;
            self.seek(0, 0);
        } else {
            s.user_paused = false;
        }
        if let Some(p) = &self.player {
            p.set_paused(false);
        }
        self.tick();
        self.emit(true);
    }

    fn pause(&mut self) {
        let Some(s) = self.session.as_mut() else { return };
        s.user_paused = true;
        if let Some(p) = &self.player {
            p.set_paused(true);
        }
        self.tick();
        self.emit(true);
    }

    /// Stop from the user: a preview returns to the stashed reading.
    fn stop(&mut self) {
        if self.session.as_ref().is_some_and(|s| s.is_preview) && self.stash.is_some() {
            self.restore_stash();
            return;
        }
        self.stop_all();
    }

    fn stop_all(&mut self) {
        self.cancel_in_flight();
        self.generation += 1;
        if let Some(p) = &self.player {
            p.clear();
            p.set_paused(true);
        }
        self.session = None;
        self.stash = None;
        self.idle_since = Some(Instant::now());
        self.host.hide_player();
        self.emit(true);
    }

    fn restore_stash(&mut self) {
        let Some(mut s) = self.stash.take() else { return };
        self.cancel_in_flight();
        self.generation += 1;
        s.status = PlaybackStatus::Paused;
        s.user_paused = true;
        s.completed_at = None;
        let (seg, off) = s.pos;
        self.session = Some(s);
        if let Some(p) = &self.player {
            p.set_paused(true);
        }
        self.publish_document();
        self.seek(seg, off);
        self.emit(true);
    }

    fn cancel_in_flight(&mut self) {
        for (_, _, token) in self.in_flight.drain(..) {
            token.cancel();
        }
    }

    fn seek(&mut self, seg: usize, offset: usize) {
        let Some(player) = &self.player else { return };
        let Some(s) = self.session.as_mut() else { return };
        player.clear();
        s.queued_upto = seg;
        s.start_offset = offset;
        s.end_sent = false;
        s.pos = (seg, offset);
        s.completed_at = None;
        if s.status == PlaybackStatus::Completed {
            s.status = PlaybackStatus::Buffering;
        }
        // A seek outranks speculative lookahead (spec §8.4): keep only jobs
        // for the target and the segments right after it.
        if !matches!(s.segs[seg], Seg::Ready(_)) && !self.in_flight.iter().any(|(_, busy, _)| *busy == seg) {
            let window = seg..seg + self.parallel_jobs();
            self.in_flight.retain(|(_, busy, token)| {
                let keep = window.contains(busy);
                if !keep {
                    token.cancel();
                }
                keep
            });
            // Every job left is just ahead of the target; free one for it.
            if self.in_flight.len() >= self.parallel_jobs() {
                if let Some(i) = (0..self.in_flight.len()).max_by_key(|&i| self.in_flight[i].1) {
                    self.in_flight.remove(i).2.cancel();
                }
            }
        }
        self.feed_player();
        self.schedule();
        self.emit(true);
    }

    fn skip(&mut self, delta_ms: i64, snap: bool) {
        let Some(s) = &self.session else { return };
        let sr = self.sample_rate as i64;
        let (total, _) = self.duration_samples(s);
        let now = self.position_samples(s) as i64;
        let target = (now + delta_ms * sr / 1000).clamp(0, (total as i64 - 1).max(0)) as u64;
        let (seg, off) = self.locate_samples(s, target);
        self.seek(seg, if snap { 0 } else { off });
    }

    fn est_samples(&self, s: &Session, i: usize) -> u64 {
        s.segs[i].samples().unwrap_or_else(|| {
            let (known_samples, known_chars) = s
                .segs
                .iter()
                .zip(&s.chars)
                .filter_map(|(g, c)| g.samples().filter(|n| *n > 0).map(|n| (n, *c as u64)))
                .fold((0u64, 0u64), |a, b| (a.0 + b.0, a.1 + b.1));
            let per_char = if known_chars >= 80 {
                known_samples as f64 / known_chars as f64
            } else {
                DEFAULT_MS_PER_CHAR * self.sample_rate as f64 / 1000.0
            };
            (s.chars[i] as f64 * per_char) as u64
        })
    }

    /// (total samples, whether every segment length is known)
    fn duration_samples(&self, s: &Session) -> (u64, bool) {
        let total = (0..s.segs.len()).map(|i| self.est_samples(s, i)).sum();
        (total, s.segs.iter().all(|g| g.samples().is_some()))
    }

    fn position_samples(&self, s: &Session) -> u64 {
        let before: u64 = (0..s.pos.0.min(s.segs.len())).map(|i| self.est_samples(s, i)).sum();
        before + s.pos.1 as u64
    }

    fn locate_samples(&self, s: &Session, target: u64) -> (usize, usize) {
        let mut acc = 0u64;
        for i in 0..s.segs.len() {
            let len = self.est_samples(s, i);
            if target < acc + len {
                let off = if s.segs[i].samples().is_some() { (target - acc) as usize } else { 0 };
                return (i, off);
            }
            acc += len;
        }
        (s.segs.len().saturating_sub(1), 0)
    }

    fn feed_player(&mut self) {
        let Some(player) = &self.player else { return };
        let Some(s) = self.session.as_mut() else { return };
        while s.queued_upto < s.segs.len() {
            match &s.segs[s.queued_upto] {
                Seg::Ready(samples) => {
                    player.append(QueuedSegment {
                        id: s.queued_upto,
                        samples: samples.clone(),
                        start: std::mem::take(&mut s.start_offset),
                    });
                    s.queued_upto += 1;
                }
                Seg::Failed => {
                    s.start_offset = 0;
                    s.queued_upto += 1;
                }
                _ => break,
            }
        }
        if s.queued_upto == s.segs.len() && !s.end_sent {
            player.end_stream();
            s.end_sent = true;
        }
    }

    /// Jobs the current voice's engine may run at once.
    fn parallel_jobs(&self) -> usize {
        self.session
            .as_ref()
            .map_or(1, |s| self.engine.max_parallel(&s.voice.id))
            .clamp(1, MAX_PARALLEL_JOBS)
    }

    /// Bounded lookahead: up to `parallel_jobs` synthesis jobs at once, in
    /// reading order, stopping at the duration or byte budget of the active
    /// resource profile.
    fn schedule(&mut self) {
        while self.in_flight.len() < self.parallel_jobs() && self.schedule_one() {}
    }

    /// Starts a job for the next segment that needs one; false when none may.
    fn schedule_one(&mut self) -> bool {
        let sr = self.sample_rate as u64;
        let budget = self.profile.pcm_budget_bytes() as u64;
        let lookahead = (self.profile.lookahead_ms() as f64 * self.rate as f64 / 1000.0 * sr as f64) as u64;
        let in_flight = &self.in_flight;
        let Some(s) = self.session.as_mut() else { return false };
        let cur = s.pos.0.min(s.segs.len().saturating_sub(1));

        // Release PCM behind the listener first when over budget.
        let mut bytes: u64 = s.segs.iter().filter_map(|g| match g {
            Seg::Ready(v) => Some(v.len() as u64 * 4),
            _ => None,
        }).sum();
        let mut i = 0;
        while bytes > budget && i < cur {
            if let Seg::Ready(v) = &s.segs[i] {
                let n = v.len() as u64;
                s.segs[i] = Seg::Evicted(n);
                bytes -= n * 4;
            }
            i += 1;
        }
        if bytes > budget {
            return false;
        }

        let mut ahead = 0u64;
        let mut target = None;
        for (i, g) in s.segs.iter().enumerate().skip(cur) {
            match g {
                Seg::Ready(v) => {
                    let skip = if i == cur { s.pos.1 as u64 } else { 0 };
                    ahead += (v.len() as u64).saturating_sub(skip);
                }
                Seg::Failed => {}
                _ if in_flight.iter().any(|(_, seg, _)| *seg == i) => {}
                Seg::Pending | Seg::Evicted(_) => {
                    target = Some(i);
                    break;
                }
            }
        }
        // The segment the player needs next is always allowed.
        let Some(target) = target else { return false };
        if ahead >= lookahead && target != s.queued_upto {
            return false;
        }
        let token = CancelToken::default();
        let job_id = self.next_job;
        self.next_job += 1;
        let job = SynthJob {
            job: job_id,
            generation: self.generation,
            segment: target,
            text: s.doc.segment_text(target).to_string(),
            voice: s.voice.id.clone(),
            sample_rate: self.sample_rate,
            cancel: token.clone(),
        };
        if self.job_tx.send(job).is_err() {
            return false;
        }
        self.in_flight.push((job_id, target, token));
        true
    }

    fn on_synth(&mut self, d: SynthDone) {
        // Jobs that ran alongside this one, which shared the processor.
        let concurrent = self.in_flight.len().max(1);
        self.in_flight.retain(|(job, _, _)| *job != d.job);
        // Stale results from a replaced or restored session are discarded.
        if d.generation != self.generation {
            return;
        }
        let rate = self.rate;
        let Some(s) = self.session.as_mut() else { return };
        match d.result {
            Ok(pcm) => {
                let audio_ms = pcm.duration_ms().max(1);
                // Real-time factor at the current speed (spec §8.3), per
                // job slot when several segments are generated at once.
                let rtf = d.elapsed.as_millis() as f64 / audio_ms as f64 / concurrent as f64;
                if rtf * rate as f64 > 1.0 {
                    self.slow_segments += 1;
                } else {
                    self.slow_segments = self.slow_segments.saturating_sub(1);
                }
                s.message = (self.slow_segments >= 3)
                    .then(|| "This voice cannot keep up at this speed.".to_string());
                if d.segment < s.segs.len() {
                    self.host.envelope(&Envelope {
                        session_id: s.id,
                        segment: d.segment,
                        duration_ms: pcm.duration_ms(),
                        levels: envelope_levels(&pcm.samples, pcm.sample_rate),
                    });
                    s.segs[d.segment] = Seg::Ready(pcm.samples.into());
                }
            }
            Err(TtsError::Cancelled) => {}
            Err(e) => {
                log::error!("synthesis failed for segment {}: {e}", d.segment);
                if d.segment < s.segs.len() {
                    s.segs[d.segment] = Seg::Failed;
                }
                self.host.notice(&Notice {
                    code: "SYNTHESIS_FAILED",
                    message: "Part of this text could not be spoken. Retry or switch voice.".into(),
                    action: None,
                });
            }
        }
        self.feed_player();
        self.schedule();
    }

    fn refresh_position(&self, s: &mut Session) {
        if let Some(p) = &self.player {
            if let Some(pos) = p.status().position {
                s.pos = (pos.segment, pos.offset);
            }
        }
    }

    fn tick(&mut self) {
        let mut hide = false;
        let mut restore = false;
        if let (Some(player), Some(s)) = (&self.player, self.session.as_mut()) {
            let st = player.status();
            if let Some(pos) = st.position {
                let len = s.segs.get(pos.segment).and_then(Seg::samples).unwrap_or(u64::MAX);
                s.pos = (pos.segment, (pos.offset as u64).min(len) as usize);
            }
            let next = if st.device_lost {
                PlaybackStatus::Recovering
            } else if s.user_paused {
                PlaybackStatus::Paused
            } else if st.finished {
                PlaybackStatus::Completed
            } else if st.starved || st.position.is_none() {
                PlaybackStatus::Buffering
            } else {
                PlaybackStatus::Playing
            };
            if next == PlaybackStatus::Completed && s.status != PlaybackStatus::Completed {
                s.completed_at = Some(Instant::now());
                if let Some(last) = s.segs.len().checked_sub(1) {
                    s.pos = (last, s.segs[last].samples().unwrap_or(0) as usize);
                }
                if s.is_preview && self.stash.is_some() {
                    restore = true;
                }
            }
            s.status = next;
            if s.completed_at.is_some_and(|t| t.elapsed() > COMPLETED_LINGER) && !s.is_preview {
                hide = true;
            }
            if st.device_lost {
                s.message = Some("Audio device disconnected. Choose an output device.".into());
            }
        }
        if restore {
            self.restore_stash();
        } else if hide {
            self.session = None;
            self.idle_since = Some(Instant::now());
            self.host.hide_player();
            self.emit(true);
        } else if self.session.as_ref().is_some_and(|s| s.is_preview && s.status == PlaybackStatus::Completed)
            && self.session.as_ref().and_then(|s| s.completed_at).is_some_and(|t| t.elapsed() > Duration::from_millis(600))
        {
            // A preview with nothing to restore simply ends.
            self.stop_all();
        }

        self.schedule();

        let active = self.session.as_ref().is_some_and(|s| s.status.is_active());
        if active != self.active {
            self.active = active;
            self.host.playback_active(active);
        }

        let status = self.session.as_ref().map_or(PlaybackStatus::Idle, |s| s.status);
        let changed = status != self.last_status;
        self.emit(changed);

        // Release the audio device when idle (spec §8.2 idle unload).
        if self.session.is_none() && self.player.is_some() {
            let since = *self.idle_since.get_or_insert_with(Instant::now);
            if since.elapsed() > Duration::from_secs(self.profile.idle_release_secs()) {
                self.player = None;
                // Idle model unload (spec §8.2): stops any inference worker.
                self.engine.unload();
            }
        }
    }

    fn emit(&mut self, force: bool) {
        let playing = self.session.as_ref().is_some_and(|s| s.status == PlaybackStatus::Playing);
        if !force && !(playing && self.last_emit.elapsed() >= SNAPSHOT_INTERVAL) {
            return;
        }
        self.last_emit = Instant::now();
        let snap = self.snapshot();
        self.last_status = snap.status;
        self.host.snapshot(&snap);
    }

    fn snapshot(&self) -> PlaybackSnapshot {
        let mut snap = PlaybackSnapshot::idle(self.rate, self.volume);
        snap.device = self.engine.device();
        let Some(s) = &self.session else { return snap };
        let sr = self.sample_rate as u64;
        let (total, known) = self.duration_samples(s);
        let pos = self.position_samples(s).min(total);
        snap.session_id = s.id;
        snap.status = s.status;
        snap.source = Some(s.source.clone());
        snap.segment_id = Some(s.pos.0);
        snap.sentence_id = s.doc.segments.get(s.pos.0).map(|g| g.sentence_id);
        snap.position_ms = pos * 1000 / sr;
        snap.duration_ms = total * 1000 / sr;
        snap.duration_is_final = known;
        snap.voice = Some(s.voice.name.clone());
        snap.device = self.engine.device_for(&s.voice.id);
        snap.message = s.message.clone();
        snap
    }
}

fn group(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Log {
        snapshots: Vec<(Instant, PlaybackSnapshot)>,
        notices: Vec<Notice>,
        envelopes: usize,
        shown: u32,
        hidden: u32,
        active: Vec<bool>,
    }

    #[derive(Clone, Default)]
    struct TestHost(Arc<Mutex<Log>>);

    impl Host for TestHost {
        fn snapshot(&self, s: &PlaybackSnapshot) {
            self.0.lock().unwrap().snapshots.push((Instant::now(), s.clone()));
        }
        fn notice(&self, n: &Notice) {
            self.0.lock().unwrap().notices.push(n.clone());
        }
        fn document(&self, _: &DocumentView) {}
        fn envelope(&self, _: &Envelope) {
            self.0.lock().unwrap().envelopes += 1;
        }
        fn show_player(&self) {
            self.0.lock().unwrap().shown += 1;
        }
        fn hide_player(&self) {
            self.0.lock().unwrap().hidden += 1;
        }
        fn playback_active(&self, a: bool) {
            self.0.lock().unwrap().active.push(a);
        }
    }

    impl TestHost {
        fn last(&self) -> PlaybackSnapshot {
            self.0.lock().unwrap().snapshots.last().unwrap().1.clone()
        }
        fn wait_for(&self, timeout: Duration, f: impl Fn(&PlaybackSnapshot) -> bool) -> Option<Duration> {
            let start = Instant::now();
            while start.elapsed() < timeout {
                if self.0.lock().unwrap().snapshots.iter().any(|(t, s)| *t >= start - Duration::from_millis(1) && f(s)) {
                    return Some(start.elapsed());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            None
        }
    }

    fn request(text: &str) -> Command {
        Command::Speak {
            request: SpeakRequest {
                text: text.into(),
                source: SourceReference { kind: SourceKind::Manual, display_name: None },
                language_hint: None,
            },
            truncate: false,
            reply: None,
        }
    }

    const TEXT: &str = "SpeakIt reads your text aloud using a voice on this computer. \
        Nothing is sent to a server. The first sentence starts quickly, while later sentences \
        are prepared in the background. You can pause, skip, and change speed at any time. \
        Speed changes keep the pitch natural. This paragraph is long enough to test seeking.";

    #[test]
    fn rejected_input_does_not_start_a_session() {
        let host = TestHost::default();
        let engine: Arc<dyn Engine> = Arc::from(speakit_tts::system_engine());
        let c = spawn(engine, Config { match_language: true, rate: 1.0, volume: 0.0, voice: None, profile: ResourceProfile::Balanced }, host.clone());
        c.send(request("   \n "));
        std::thread::sleep(Duration::from_millis(200));
        let log = host.0.lock().unwrap();
        assert_eq!(log.notices.first().map(|n| n.code), Some("NO_TEXT"));
        assert_eq!(log.shown, 0);
        drop(log);
        c.send(Command::Shutdown);
    }

    /// Speaks every sentence as a one-second tone; sentences after the first
    /// take longer to synthesize than the audio before them lasts.
    struct SlowEngine(std::sync::atomic::AtomicUsize);

    impl Engine for SlowEngine {
        fn id(&self) -> &'static str { "slow" }
        fn device(&self) -> &'static str { "CPU" }
        fn voices(&self) -> Result<Vec<VoiceInfo>, TtsError> {
            Ok(vec![VoiceInfo {
                id: "tone".into(), name: "Tone".into(), language: "en-US".into(), gender: None, model: "system".into(),
                engine: "Test".into(), recommended: true, approximate_pronunciation: false,
            }])
        }
        fn max_segment_chars(&self) -> usize { 40 }
        fn synthesize(&self, _: &str, _: &str, sample_rate: u32, cancel: &CancelToken) -> Result<Pcm, TtsError> {
            if self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed) > 0 {
                let start = Instant::now();
                while start.elapsed() < Duration::from_millis(2500) {
                    if cancel.is_cancelled() { return Err(TtsError::Cancelled); }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
            let samples = (0..sample_rate).map(|i| 0.2 * (i as f32 * 440.0 * std::f32::consts::TAU / sample_rate as f32).sin()).collect();
            Ok(Pcm { samples, sample_rate })
        }
    }

    #[test]
    #[ignore = "opens the default output device (silently)"]
    fn waiting_for_audio_shows_buffering_and_holds_the_position() {
        let host = TestHost::default();
        let engine: Arc<dyn Engine> = Arc::new(SlowEngine(Default::default()));
        let c = spawn(engine, Config { match_language: false, rate: 1.3, volume: 0.0, voice: Some("tone".into()), profile: ResourceProfile::Balanced }, host.clone());
        c.send(request("The first sentence is ready. The second one is slow. So is the third."));
        host.wait_for(Duration::from_secs(3), |s| s.status == PlaybackStatus::Playing).expect("plays");

        // One second of audio at 1.3× runs out well before the next sentence.
        host.wait_for(Duration::from_secs(2), |s| s.status == PlaybackStatus::Buffering)
            .expect("shows buffering while the next sentence is synthesized");
        let held = host.last().position_ms;
        assert!(held <= 1000, "position stops at the end of the audio: {held}");
        std::thread::sleep(Duration::from_millis(600));
        let last = host.last();
        assert_eq!(last.status, PlaybackStatus::Buffering);
        assert_eq!(last.position_ms, held, "position holds while nothing plays");

        host.wait_for(Duration::from_secs(3), |s| s.status == PlaybackStatus::Playing && s.position_ms > held)
            .expect("resumes when the sentence arrives");
        c.send(Command::Shutdown);
    }

    #[test]
    #[ignore = "plays audio through the default output device"]
    fn end_to_end_reading() {
        let host = TestHost::default();
        let engine: Arc<dyn Engine> = Arc::from(speakit_tts::system_engine());
        let c = spawn(engine, Config { match_language: true, rate: 1.0, volume: 0.15, voice: None, profile: ResourceProfile::Balanced }, host.clone());

        c.send(request(TEXT));
        let first = host.wait_for(Duration::from_secs(5), |s| s.status == PlaybackStatus::Playing)
            .expect("reading should start playing");
        println!("first audible output after {first:?}");
        assert!(first < Duration::from_millis(1500), "warm first audio target is 1.5 s");
        assert!(host.0.lock().unwrap().active.contains(&true), "playback shortcuts activate");

        std::thread::sleep(Duration::from_millis(1500));
        let before = host.last().position_ms;
        assert!(before > 800, "position advances while playing: {before}");

        c.send(Command::Pause);
        let paused = host.wait_for(Duration::from_secs(1), |s| s.status == PlaybackStatus::Paused).expect("pauses");
        println!("pause acknowledged after {paused:?}");
        assert!(paused < Duration::from_millis(150));
        let held = host.last().position_ms;
        std::thread::sleep(Duration::from_millis(400));
        assert!(host.last().position_ms.abs_diff(held) < 100, "position holds while paused");

        c.send(Command::Skip { delta_ms: 5_000, snap: false });
        std::thread::sleep(Duration::from_millis(300));
        let skipped = host.last().position_ms;
        assert!(skipped >= held + 4_000, "skip forward moves position: {held} -> {skipped}");

        c.send(Command::SetRate(2.0));
        c.send(Command::Play);
        host.wait_for(Duration::from_secs(3), |s| s.status == PlaybackStatus::Playing).expect("resumes");
        // Let audio buffered at the old speed drain before measuring.
        std::thread::sleep(Duration::from_millis(400));
        let (t0, p0) = { let l = host.0.lock().unwrap(); let (t, s) = l.snapshots.last().unwrap(); (*t, s.position_ms) };
        std::thread::sleep(Duration::from_millis(2000));
        let (t1, p1) = { let l = host.0.lock().unwrap(); let (t, s) = l.snapshots.last().unwrap(); (*t, s.position_ms) };
        let speed = (p1 - p0) as f64 / (t1 - t0).as_millis() as f64;
        println!("measured playback speed {speed:.2}x at a 2.0x setting");
        assert!((1.7..2.3).contains(&speed), "2x speed advances ~2 s of source per second");

        let log = host.0.lock().unwrap();
        assert!(log.envelopes >= 2, "waveform envelopes emitted");
        let durations: Vec<bool> = log.snapshots.iter().map(|(_, s)| s.duration_is_final).collect();
        assert!(!durations[0], "duration starts as an estimate");
        drop(log);

        // A new reading replaces the current one with a new session ID, and
        // rejected input does not interrupt it.
        let first_id = host.last().session_id;
        c.send(request("A second reading replaces the first one."));
        host.wait_for(Duration::from_secs(3), |s| s.session_id > first_id && s.status == PlaybackStatus::Playing)
            .expect("replacement session plays");
        let second_id = host.last().session_id;
        c.send(request("  "));
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(host.last().session_id, second_id, "empty input keeps the current reading");
        assert_eq!(host.last().status, PlaybackStatus::Playing);

        c.send(Command::Stop);
        host.wait_for(Duration::from_secs(1), |s| s.status == PlaybackStatus::Idle).expect("stops");
        let log = host.0.lock().unwrap();
        assert!(log.hidden >= 1);
        assert_eq!(log.active.last(), Some(&false), "playback shortcuts released after stop");
        drop(log);
        c.send(Command::Shutdown);
    }

    #[test]
    #[ignore = "needs the installed Kokoro model and plays audio"]
    fn end_to_end_kokoro() {
        kokoro_reading(false);
    }

    #[test]
    #[ignore = "needs the installed Kokoro model, an Apple Silicon or NVIDIA GPU (with scripts/fetch-cuda.mjs on Windows), and plays audio"]
    fn end_to_end_kokoro_gpu() {
        kokoro_reading(true);
    }

    fn kokoro_reading(gpu: bool) {
        use speakit_models::Store;
        use speakit_tts::kokoro::{KokoroConfig, KokoroEngine, KokoroVoice};
        let model = speakit_models::find("kokoro-82m-v1.0").unwrap();
        let root = if cfg!(windows) {
            std::path::PathBuf::from(std::env::var("APPDATA").unwrap()).join("com.sovirae.desktop/models")
        } else {
            std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("Library/Application Support/com.sovirae.desktop/models")
        };
        let installed = Store::new(root).installed(model).expect("Kokoro installed");
        let worker_bin = std::env::current_dir().unwrap().join("../target/release/sovirae-kokoro-worker");
        let voices = model.voices.iter().map(|v| KokoroVoice {
            id: v.id.clone(), label: v.label.clone(), language: v.language.clone(), gender: v.gender.clone(), g2p: v.g2p.clone(), approximate_pronunciation: v.pronunciation.is_some(), file: installed.voice_file(&v.id),
        }).collect();
        let registry = Arc::new(speakit_tts::Registry::new(speakit_tts::system_engine()));
        registry.set_model(&model.voice_prefix, Some(Arc::new(KokoroEngine::new(KokoroConfig {
            model_id: model.id.clone(), model_name: model.name.clone(), voice_prefix: model.voice_prefix.clone(),
            model_file: installed.model_file.clone(), worker_bin, voices, threads: 4, parallel: 1, gpu, model_rate: model.sample_rate,
        }))));
        let host = TestHost::default();
        let c = spawn(registry.clone(), Config { match_language: true, rate: 1.0, volume: 0.15, voice: Some("kokoro:af_heart".into()), profile: ResourceProfile::Balanced }, host.clone());

        c.send(request(TEXT));
        let first = host.wait_for(Duration::from_secs(15), |s| s.status == PlaybackStatus::Playing).expect("plays");
        println!("Kokoro first audible output (cold worker) after {first:?}");
        assert_eq!(host.last().voice.as_deref(), Some("Heart"));
        assert_eq!(host.last().device, if gpu { "GPU" } else { "CPU" }, "GPU fallback: {:?}", registry.gpu_fallback());
        std::thread::sleep(Duration::from_millis(2500));
        assert!(host.last().position_ms > 1500, "position advances");
        assert!(host.0.lock().unwrap().notices.is_empty(), "no errors");

        c.send(request("A second, warm reading starts faster."));
        let warm = host.wait_for(Duration::from_secs(5), |s| s.status == PlaybackStatus::Playing && s.session_id >= 2).expect("replacement plays");
        println!("Kokoro first audible output (warm) after {warm:?}");
        std::thread::sleep(Duration::from_millis(1500));
        c.send(Command::Stop);
        host.wait_for(Duration::from_secs(1), |s| s.status == PlaybackStatus::Idle).expect("stops");
        c.send(Command::Shutdown);
    }

    #[test]
    #[ignore = "needs the installed Pocket TTS English model and plays audio"]
    fn end_to_end_pocket() {
        use speakit_models::Store;
        use speakit_tts::pocket::{PocketConfig, PocketEngine, PocketVoice};
        let model = speakit_models::find("pocket-tts-en").unwrap();
        let root = if cfg!(windows) {
            std::path::PathBuf::from(std::env::var("APPDATA").unwrap()).join("com.sovirae.desktop/models")
        } else {
            std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("Library/Application Support/com.sovirae.desktop/models")
        };
        let installed = Store::new(root).installed(model).expect("Pocket TTS English installed");
        let worker_bin = std::env::current_dir().unwrap().join("../target/release/sovirae-pocket-worker");
        let voices = model.voices.iter().map(|v| PocketVoice {
            id: v.id.clone(), label: v.label.clone(), language: v.language.clone(), gender: v.gender.clone(), file: installed.voice_file(&v.id),
        }).collect();
        let registry = Arc::new(speakit_tts::Registry::new(speakit_tts::system_engine()));
        registry.set_model(&model.voice_prefix, Some(Arc::new(PocketEngine::new(PocketConfig {
            model_id: model.id.clone(), model_name: model.name.clone(), voice_prefix: model.voice_prefix.clone(),
            model_file: installed.model_file.clone(), tokenizer_file: installed.support_file(&model.files[0]), worker_bin,
            voices, threads: 4, options: model.options.clone(), model_rate: model.sample_rate,
        }))));
        let host = TestHost::default();
        let c = spawn(registry.clone(), Config { match_language: true, rate: 1.0, volume: 0.15, voice: Some("pocket-en:alba".into()), profile: ResourceProfile::Balanced }, host.clone());

        c.send(request(TEXT));
        let first = host.wait_for(Duration::from_secs(15), |s| s.status == PlaybackStatus::Playing).expect("plays");
        println!("Pocket TTS first audible output (cold worker) after {first:?}");
        assert_eq!(host.last().voice.as_deref(), Some("Alba"));
        assert_eq!(host.last().device, "CPU");
        std::thread::sleep(Duration::from_millis(2500));
        assert!(host.last().position_ms > 1500, "position advances");
        assert!(host.0.lock().unwrap().notices.is_empty(), "no errors");

        c.send(request("A second, warm reading starts faster."));
        let warm = host.wait_for(Duration::from_secs(5), |s| s.status == PlaybackStatus::Playing && s.session_id >= 2).expect("replacement plays");
        println!("Pocket TTS first audible output (warm) after {warm:?}");
        std::thread::sleep(Duration::from_millis(1500));
        c.send(Command::Stop);
        host.wait_for(Duration::from_secs(1), |s| s.status == PlaybackStatus::Idle).expect("stops");
        c.send(Command::Shutdown);
    }

    #[test]
    #[ignore = "needs the installed Kokoro model"]
    fn installed_kokoro_lists_every_catalog_voice() {
        use speakit_models::Store;
        use speakit_tts::kokoro::{KokoroConfig, KokoroEngine, KokoroVoice};
        let model = speakit_models::find("kokoro-82m-v1.0").unwrap();
        let root = if cfg!(windows) {
            std::path::PathBuf::from(std::env::var("APPDATA").unwrap()).join("com.sovirae.desktop/models")
        } else {
            std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("Library/Application Support/com.sovirae.desktop/models")
        };
        let installed = Store::new(root).installed(model).expect("Kokoro installed");
        let voices = model.voices.iter().map(|v| KokoroVoice {
            id: v.id.clone(), label: v.label.clone(), language: v.language.clone(), gender: v.gender.clone(), g2p: v.g2p.clone(), approximate_pronunciation: v.pronunciation.is_some(), file: installed.voice_file(&v.id),
        }).collect();
        let registry = speakit_tts::Registry::new(speakit_tts::system_engine());
        registry.set_model(&model.voice_prefix, Some(Arc::new(KokoroEngine::new(KokoroConfig {
            model_id: model.id.clone(), model_name: model.name.clone(), voice_prefix: model.voice_prefix.clone(),
            model_file: installed.model_file.clone(), worker_bin: "unused".into(), voices, threads: 1, parallel: 1, gpu: false, model_rate: model.sample_rate,
        }))));
        let listed: Vec<String> = registry.voices().unwrap().into_iter()
            .filter(|v| v.model == model.id).map(|v| format!("{} ({}, {})", v.name, v.language, v.gender.unwrap_or_default())).collect();
        println!("{} Kokoro voices: {}", listed.len(), listed.join(", "));
        assert_eq!(listed.len(), model.voices.len());
        assert!(listed.iter().any(|v| v.starts_with("Echo")) && listed.iter().any(|v| v.starts_with("Eric")));
    }
}
