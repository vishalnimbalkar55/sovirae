//! Native audio output (spec §5.1, §10.5).
//!
//! The realtime callback only pops from a lock-free SPSC ring and touches
//! atomics: no allocation, no locks, no UI. A feeder thread owns the
//! segment queue and the time stretcher and keeps the ring topped up.

pub mod stretch;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use thiserror::Error;

use stretch::Stretcher;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("No audio output device is available.")]
    NoDevice,
    #[error("The audio device could not be opened: {0}")]
    Open(String),
}

/// Ring capacity: short, so speed changes take effect quickly.
const RING_MS: u64 = 200;
/// Waveform envelope resolution.
pub const ENVELOPE_HZ: u32 = 50;

struct Shared {
    paused: AtomicBool,
    volume: AtomicU32,
    /// Output samples actually played from the ring.
    played: AtomicU64,
    underruns: AtomicU64,
    flush_req: AtomicU64,
    flush_ack: AtomicU64,
    /// Last output peak for a live level meter.
    level: AtomicU32,
    device_lost: AtomicBool,
}

/// One synthesized segment queued for playback.
pub struct QueuedSegment {
    pub id: usize,
    pub samples: Arc<[f32]>,
    /// Source sample where playback of this segment starts (for seeks).
    pub start: usize,
}

/// Maps an output sample index to a source position.
#[derive(Clone, Copy)]
struct Marker {
    out_index: u64,
    segment: usize,
    /// Source sample offset within the segment at `out_index`.
    offset: usize,
    /// Rate in effect after this marker.
    rate: f32,
}

struct Feeder {
    queue: VecDeque<QueuedSegment>,
    /// Segment IDs whose samples have entered the stretcher, with the
    /// stretcher's absolute input position where each begins.
    fed: VecDeque<(usize, u64, usize)>,
    stretcher: Stretcher,
    fed_total: u64,
    out_total: u64,
    markers: VecDeque<Marker>,
    rate: f32,
    end_of_stream: bool,
    scratch: Vec<f32>,
}

impl Feeder {
    fn reset(&mut self) {
        self.queue.clear();
        self.fed.clear();
        self.stretcher.reset();
        self.fed_total = 0;
        self.markers.clear();
        self.end_of_stream = false;
        self.scratch.clear();
    }

    /// Source position for absolute stretcher input position `pos`.
    fn locate(&self, pos: u64) -> Option<(usize, usize)> {
        let mut found = None;
        for &(id, begin, start) in &self.fed {
            if begin <= pos {
                found = Some((id, start + (pos - begin) as usize));
            } else {
                break;
            }
        }
        found
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub segment: usize,
    /// Source samples into the segment.
    pub offset: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerStatus {
    pub position: Option<Position>,
    /// Every queued segment has been played and the stream was ended.
    pub finished: bool,
    /// Waiting for more synthesized audio.
    pub starved: bool,
    pub underruns: u64,
    pub device_lost: bool,
    pub level: f32,
}

pub struct Player {
    shared: Arc<Shared>,
    feeder: Arc<Mutex<Feeder>>,
    sample_rate: u32,
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl Player {
    /// Opens the default output device. The stream follows OS default-device
    /// changes where the backend supports it.
    pub fn new() -> Result<Self, AudioError> {
        let (ready_tx, ready_rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            paused: AtomicBool::new(true),
            volume: AtomicU32::new(0.8f32.to_bits()),
            played: AtomicU64::new(0),
            underruns: AtomicU64::new(0),
            flush_req: AtomicU64::new(0),
            flush_ack: AtomicU64::new(0),
            level: AtomicU32::new(0),
            device_lost: AtomicBool::new(false),
        });
        let stop = Arc::new(AtomicBool::new(false));

        // cpal streams are not Send; the stream lives on its own thread.
        let (s, st) = (shared.clone(), stop.clone());
        let stream_thread = std::thread::Builder::new()
            .name("speakit-audio".into())
            .spawn(move || run_stream(s, st, ready_tx))
            .map_err(|e| AudioError::Open(e.to_string()))?;
        let (sample_rate, producer) = ready_rx
            .recv()
            .map_err(|_| AudioError::Open("audio thread exited".into()))??;

        let feeder = Arc::new(Mutex::new(Feeder {
            queue: VecDeque::new(),
            fed: VecDeque::new(),
            stretcher: Stretcher::new(sample_rate),
            fed_total: 0,
            out_total: 0,
            markers: VecDeque::new(),
            rate: 1.0,
            end_of_stream: false,
            scratch: Vec::with_capacity(4096),
        }));
        let (f, s, st) = (feeder.clone(), shared.clone(), stop.clone());
        let feed_thread = std::thread::Builder::new()
            .name("speakit-feeder".into())
            .spawn(move || run_feeder(f, s, st, producer))
            .map_err(|e| AudioError::Open(e.to_string()))?;

        Ok(Self { shared, feeder, sample_rate, stop, threads: vec![stream_thread, feed_thread] })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn append(&self, segment: QueuedSegment) {
        self.feeder.lock().unwrap().queue.push_back(segment);
    }

    /// No more segments will be appended for this reading.
    pub fn end_stream(&self) {
        self.feeder.lock().unwrap().end_of_stream = true;
    }

    /// Silences output immediately and discards everything queued.
    pub fn clear(&self) {
        let mut f = self.feeder.lock().unwrap();
        f.reset();
        let req = self.shared.flush_req.fetch_add(1, Ordering::AcqRel) + 1;
        // The callback drains the ring and acknowledges; bounded wait in
        // case the device has stopped calling back.
        for _ in 0..40 {
            if self.shared.flush_ack.load(Ordering::Acquire) >= req {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        f.out_total = self.shared.played.load(Ordering::Acquire);
    }

    pub fn set_paused(&self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Release);
    }

    pub fn set_volume(&self, volume: f32) {
        self.shared.volume.store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Release);
    }

    pub fn set_rate(&self, rate: f32) {
        let mut f = self.feeder.lock().unwrap();
        f.rate = rate;
        f.stretcher.set_rate(rate);
    }

    pub fn status(&self) -> PlayerStatus {
        let played = self.shared.played.load(Ordering::Acquire);
        let f = self.feeder.lock().unwrap();
        let position = f
            .markers
            .iter()
            .rev()
            .find(|m| m.out_index <= played)
            .or_else(|| f.markers.front())
            .map(|m| {
                let elapsed_out = played.saturating_sub(m.out_index) as f64;
                let adv = (elapsed_out * m.rate as f64) as usize;
                Position { segment: m.segment, offset: m.offset + adv }
            });
        let drained = played >= f.out_total;
        let queued = !f.queue.is_empty() || !f.scratch.is_empty();
        let input_left = queued || f.stretcher.pending_input() > 0;
        PlayerStatus {
            position,
            finished: f.end_of_stream && !input_left && drained,
            // The stretcher's sub-frame remainder between segments cannot
            // play until more audio arrives, so it does not count: otherwise
            // the reading shows as playing while the output is silent.
            starved: !f.end_of_stream && !queued && !f.stretcher.can_process() && drained,
            underruns: self.shared.underruns.load(Ordering::Relaxed),
            device_lost: self.shared.device_lost.load(Ordering::Relaxed),
            level: f32::from_bits(self.shared.level.load(Ordering::Relaxed)),
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

type Ready = Result<(u32, rtrb::Producer<f32>), AudioError>;

fn run_stream(shared: Arc<Shared>, stop: Arc<AtomicBool>, ready: mpsc::Sender<Ready>) {
    let host = cpal::default_host();
    let Some(device) = host.default_output_device() else {
        let _ = ready.send(Err(AudioError::NoDevice));
        return;
    };
    let supported = match device.default_output_config() {
        Ok(c) => c,
        Err(e) => {
            let _ = ready.send(Err(AudioError::Open(e.to_string())));
            return;
        }
    };
    let config: cpal::StreamConfig = supported.config();
    let sample_rate = config.sample_rate;
    let channels = config.channels.max(1) as usize;
    let capacity = (sample_rate as u64 * RING_MS / 1000) as usize;
    let (producer, mut consumer) = rtrb::RingBuffer::<f32>::new(capacity);

    let cb = shared.clone();
    let mut last_flush = 0u64;
    let data = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
        let req = cb.flush_req.load(Ordering::Acquire);
        if req != last_flush {
            let n = consumer.slots();
            if let Ok(chunk) = consumer.read_chunk(n) {
                chunk.commit_all();
            }
            last_flush = req;
            cb.flush_ack.store(req, Ordering::Release);
        }
        let volume = f32::from_bits(cb.volume.load(Ordering::Relaxed));
        let mut peak = 0.0f32;
        if cb.paused.load(Ordering::Acquire) {
            out.fill(0.0);
        } else {
            let mut played = 0u64;
            let mut starved = false;
            for frame in out.chunks_mut(channels) {
                let s = match consumer.pop() {
                    Ok(s) => {
                        played += 1;
                        s * volume
                    }
                    Err(_) => {
                        starved = true;
                        0.0
                    }
                };
                peak = peak.max(s.abs());
                frame.fill(s);
            }
            cb.played.fetch_add(played, Ordering::AcqRel);
            if starved && played > 0 {
                cb.underruns.fetch_add(1, Ordering::Relaxed);
            }
        }
        cb.level.store(peak.to_bits(), Ordering::Relaxed);
    };
    let err_shared = shared.clone();
    let on_error = move |e: cpal::Error| {
        log::warn!("audio stream error: {e}");
        if matches!(e.kind(), cpal::ErrorKind::DeviceNotAvailable) {
            err_shared.device_lost.store(true, Ordering::Release);
        }
    };

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream::<f32, _, _>(config, data, on_error, None),
        other => {
            let _ = ready.send(Err(AudioError::Open(format!("unsupported sample format {other}"))));
            return;
        }
    };
    let stream = match stream {
        Ok(s) => s,
        Err(e) => {
            let _ = ready.send(Err(AudioError::Open(e.to_string())));
            return;
        }
    };
    if let Err(e) = stream.play() {
        let _ = ready.send(Err(AudioError::Open(e.to_string())));
        return;
    }
    let _ = ready.send(Ok((sample_rate, producer)));
    while !stop.load(Ordering::Acquire) {
        std::thread::sleep(Duration::from_millis(50));
    }
    drop(stream);
}

fn run_feeder(
    feeder: Arc<Mutex<Feeder>>,
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
    mut producer: rtrb::Producer<f32>,
) {
    let mut seen_flush = 0u64;
    while !stop.load(Ordering::Acquire) {
        {
            let mut f = feeder.lock().unwrap();
            let req = shared.flush_req.load(Ordering::Acquire);
            if req != seen_flush {
                // Write nothing until the callback has drained the ring:
                // the drain would discard new audio as well. `clear` stops
                // waiting after 200 ms, and the first callback of a new
                // stream can come later than that (seen on Windows).
                if shared.flush_ack.load(Ordering::Acquire) < req {
                    drop(f);
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                seen_flush = req;
                // Drop stretched samples from before the flush; the ring is
                // empty, so everything written so far has been played.
                f.scratch.clear();
                f.out_total = shared.played.load(Ordering::Acquire);
            }
            loop {
                // Move stretched samples into the ring as space allows.
                if !f.scratch.is_empty() {
                    let n = producer.slots().min(f.scratch.len());
                    if n == 0 {
                        break;
                    }
                    if let Ok(mut chunk) = producer.write_chunk_uninit(n) {
                        let (a, b) = chunk.as_mut_slices();
                        for (dst, src) in a.iter_mut().chain(b.iter_mut()).zip(f.scratch.iter()) {
                            dst.write(*src);
                        }
                        unsafe { chunk.commit_all() };
                    }
                    f.scratch.drain(..n);
                    f.out_total += n as u64;
                    continue;
                }
                // Keep the stretcher supplied with input.
                if f.stretcher.pending_input() < 8192 {
                    if let Some(seg) = f.queue.pop_front() {
                        let begin = f.fed_total;
                        let data = &seg.samples[seg.start.min(seg.samples.len())..];
                        f.stretcher.push(data);
                        f.fed_total += data.len() as u64;
                        f.fed.push_back((seg.id, begin, seg.start));
                        while f.fed.len() > 64 {
                            f.fed.pop_front();
                        }
                    } else if f.end_of_stream {
                        f.stretcher.finish();
                    }
                }
                let at = f.stretcher.input_position();
                let out_index = f.out_total;
                let mut scratch = std::mem::take(&mut f.scratch);
                let produced = f.stretcher.process_hop(&mut scratch);
                f.scratch = scratch;
                if produced == 0 {
                    break;
                }
                if let Some((segment, offset)) = f.locate(at) {
                    let rate = f.rate;
                    f.markers.push_back(Marker { out_index, segment, offset, rate });
                    let played = shared.played.load(Ordering::Acquire);
                    while f.markers.len() > 2 && f.markers[1].out_index <= played {
                        f.markers.pop_front();
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(8));
    }
}
