//! Kokoro-82M engine: text → phonemes (espeak-ng) → isolated ONNX worker →
//! 24 kHz PCM → device rate. The worker is started on first use, killed on
//! cancellation or crash, and unloaded when idle (spec §8.4).

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::espeak::Phonemizer;
use crate::resample::resample;
use crate::{CancelToken, Engine, Pcm, TtsError, VoiceInfo};

const LOAD_TIMEOUT: Duration = Duration::from_secs(60);
const SYNTH_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct KokoroVoice {
    pub id: String,
    pub label: String,
    pub language: String,
    pub gender: Option<String>,
    /// espeak-ng voice used to phonemize text for this voice.
    pub g2p: String,
    pub approximate_pronunciation: bool,
    pub file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct KokoroConfig {
    /// Catalog model ID and display name.
    pub model_id: String,
    pub model_name: String,
    /// Voice IDs are `<voice_prefix>:<voice>`.
    pub voice_prefix: String,
    pub model_file: PathBuf,
    pub worker_bin: PathBuf,
    pub voices: Vec<KokoroVoice>,
    pub threads: usize,
    pub model_rate: u32,
}

struct Frame {
    id: u64,
    status: u32,
    payload: Vec<u8>,
}

struct WorkerProc {
    child: Child,
    stdin: ChildStdin,
    frames: Receiver<Frame>,
}

impl Drop for WorkerProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct KokoroEngine {
    config: Mutex<KokoroConfig>,
    phonemizer: Option<Phonemizer>,
    worker: Mutex<Option<WorkerProc>>,
    next_id: AtomicU64,
}

impl KokoroEngine {
    pub fn new(config: KokoroConfig) -> Self {
        Self {
            config: Mutex::new(config),
            phonemizer: Phonemizer::find(),
            worker: Mutex::new(None),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn has_phonemizer(&self) -> bool {
        self.phonemizer.is_some()
    }

    fn spawn(&self, cancel: &CancelToken) -> Result<WorkerProc, TtsError> {
        let c = self.config.lock().unwrap().clone();
        let mut child = Command::new(&c.worker_bin)
            .arg("--model")
            .arg(&c.model_file)
            .arg("--threads")
            .arg(c.threads.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| TtsError::Engine(format!("the voice worker could not start: {e}")))?;
        let stdin = child.stdin.take().expect("piped stdin");
        let mut stdout = child.stdout.take().expect("piped stdout");
        let (tx, frames) = mpsc::channel();
        std::thread::Builder::new()
            .name("kokoro-reader".into())
            .spawn(move || loop {
                let mut header = [0u8; 16];
                if stdout.read_exact(&mut header).is_err() {
                    break;
                }
                let id = u64::from_le_bytes(header[0..8].try_into().unwrap());
                let status = u32::from_le_bytes(header[8..12].try_into().unwrap());
                let len = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
                let bytes = if status == 0 { len * 4 } else { len };
                let mut payload = vec![0u8; bytes];
                if stdout.read_exact(&mut payload).is_err() {
                    break;
                }
                if tx.send(Frame { id, status, payload }).is_err() {
                    break;
                }
            })
            .map_err(|e| TtsError::Engine(e.to_string()))?;

        let mut proc = WorkerProc { child, stdin, frames };
        let ready = wait_frame(&mut proc, 0, cancel, LOAD_TIMEOUT)?;
        match ready.status {
            2 => Ok(proc),
            _ => Err(TtsError::Engine(String::from_utf8_lossy(&ready.payload).into_owned())),
        }
    }
}

fn wait_frame(proc: &mut WorkerProc, id: u64, cancel: &CancelToken, timeout: Duration) -> Result<Frame, TtsError> {
    let start = Instant::now();
    loop {
        if cancel.is_cancelled() {
            return Err(TtsError::Cancelled);
        }
        match proc.frames.recv_timeout(Duration::from_millis(15)) {
            Ok(f) if f.id == id || f.status == 2 || (f.id == 0 && f.status == 1) => return Ok(f),
            Ok(_) => continue, // stale reply to a cancelled request
            Err(RecvTimeoutError::Timeout) => {
                if start.elapsed() > timeout {
                    return Err(TtsError::Engine("the voice worker stopped responding".into()));
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(TtsError::Engine("the voice worker stopped unexpectedly".into()));
            }
        }
    }
}

impl Engine for KokoroEngine {
    fn id(&self) -> &'static str {
        "kokoro"
    }

    fn device(&self) -> &'static str {
        "CPU"
    }

    fn voices(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        let c = self.config.lock().unwrap();
        Ok(c.voices
            .iter()
            .filter(|v| v.file.is_file())
            .map(|v| VoiceInfo {
                id: format!("{}:{}", c.voice_prefix, v.id),
                name: v.label.clone(),
                language: v.language.clone(),
                gender: v.gender.clone(),
                model: c.model_id.clone(),
                engine: c.model_name.clone(),
                recommended: false,
                approximate_pronunciation: v.approximate_pronunciation,
            })
            .collect())
    }

    fn max_segment_chars(&self) -> usize {
        300
    }

    fn synthesize(&self, text: &str, voice: &str, sample_rate: u32, cancel: &CancelToken) -> Result<Pcm, TtsError> {
        let phonemizer = self
            .phonemizer
            .as_ref()
            .ok_or_else(|| TtsError::Engine("espeak-ng is required for Kokoro pronunciation".into()))?;
        let (file, g2p, model_rate) = {
            let c = self.config.lock().unwrap();
            let id = voice.split_once(':').map_or(voice, |(_, v)| v);
            let v = c.voices.iter().find(|v| v.id == id).ok_or_else(|| TtsError::UnknownVoice(voice.into()))?;
            (v.file.clone(), v.g2p.clone(), c.model_rate)
        };
        let phonemes = phonemizer.phonemize(text, &g2p)?;
        if phonemes.is_empty() {
            return Ok(Pcm { samples: Vec::new(), sample_rate });
        }

        let mut guard = self.worker.lock().unwrap();
        if guard.is_none() {
            *guard = Some(self.spawn(cancel)?);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let req = serde_json::json!({
            "id": id,
            "phonemes": phonemes,
            "voice": file.to_string_lossy(),
            "speed": 1.0,
        });
        let proc = guard.as_mut().unwrap();
        let sent = writeln!(proc.stdin, "{req}").and_then(|_| proc.stdin.flush());
        if sent.is_err() {
            *guard = None;
            return Err(TtsError::Engine("the voice worker stopped unexpectedly".into()));
        }
        let frame = match wait_frame(proc, id, cancel, SYNTH_TIMEOUT) {
            Ok(f) => f,
            Err(e) => {
                // Cancelled or failed: native inference may not stop on its
                // own, so replace the worker (spec §8.4).
                *guard = None;
                return Err(e);
            }
        };
        drop(guard);
        if frame.status != 0 {
            return Err(TtsError::Engine(String::from_utf8_lossy(&frame.payload).into_owned()));
        }
        let samples: Vec<f32> = frame
            .payload
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        Ok(Pcm { samples: resample(&samples, model_rate, sample_rate), sample_rate })
    }

    fn unload(&self) {
        *self.worker.lock().unwrap() = None;
    }

    /// Takes effect on the next worker start.
    fn set_threads(&self, threads: usize) {
        let mut c = self.config.lock().unwrap();
        if c.threads != threads {
            c.threads = threads;
            drop(c);
            self.unload();
        }
    }
}
