//! Kokoro-82M engine: text → phonemes (espeak-ng) → isolated ONNX worker →
//! 24 kHz PCM → device rate. The worker is started on first use, killed on
//! cancellation or crash, and unloaded when idle (spec §8.4).
//!
//! With `gpu` set the worker is asked for the GPU provider. If it cannot load
//! or fails while reading, the engine records why and continues on the CPU
//! until the setting changes (spec §7.3).

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use crate::espeak::Phonemizer;
use crate::resample::resample;
use crate::worker::{self, WorkerProc};
use crate::{CancelToken, Engine, Pcm, TtsError, VoiceInfo};

const LOAD_TIMEOUT: Duration = Duration::from_secs(60);
const SYNTH_TIMEOUT: Duration = Duration::from_secs(60);

/// Whether this build's worker has a GPU provider: WebGPU on Metal on Apple
/// Silicon, DirectML on Windows. Whether the GPU actually loads is only known
/// once the worker starts, which falls back to the CPU and reports why.
pub const GPU_AVAILABLE: bool = cfg!(any(all(target_os = "macos", target_arch = "aarch64"), windows));

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
    /// Workers that may generate segments at once, each with `threads`
    /// threads. On hybrid laptop CPUs several small workers outrun one wide
    /// one, since each wide step waits for the slowest core.
    pub parallel: usize,
    /// Ask the worker for the GPU provider (WebGPU on Apple Silicon, CUDA on Windows).
    pub gpu: bool,
    /// Run workers at reduced CPU priority (Eco and Balanced, spec §8.2).
    pub below_normal: bool,
    pub model_rate: u32,
}

pub struct KokoroEngine {
    config: Mutex<KokoroConfig>,
    phonemizer: Option<Phonemizer>,
    /// Loaded workers not running a request. A busy worker is owned by the
    /// call using it and returns here afterwards.
    idle: Mutex<Vec<WorkerProc>>,
    /// Bumped by `unload`; a worker started before it is not returned.
    epoch: AtomicU64,
    next_id: AtomicU64,
    /// Device reported by the most recent worker; "CPU" until one loads.
    device: Mutex<&'static str>,
    /// Why the GPU could not be used; cleared when the setting changes.
    gpu_error: Mutex<Option<String>>,
}

impl KokoroEngine {
    pub fn new(config: KokoroConfig) -> Self {
        Self {
            config: Mutex::new(config),
            phonemizer: Phonemizer::find(),
            idle: Mutex::new(Vec::new()),
            epoch: AtomicU64::new(0),
            next_id: AtomicU64::new(1),
            device: Mutex::new("CPU"),
            gpu_error: Mutex::new(None),
        }
    }

    pub fn has_phonemizer(&self) -> bool {
        self.phonemizer.is_some()
    }

    /// Starts a worker on the GPU when requested and still usable, else on
    /// the CPU.
    fn start(&self, cancel: &CancelToken) -> Result<WorkerProc, TtsError> {
        let gpu = self.config.lock().unwrap().gpu && self.gpu_error.lock().unwrap().is_none();
        if gpu {
            match self.spawn(true, cancel) {
                Ok(proc) => return Ok(proc),
                Err(TtsError::Cancelled) => return Err(TtsError::Cancelled),
                Err(TtsError::Engine(reason)) => self.gpu_failed(reason),
                Err(e) => self.gpu_failed(e.to_string()),
            }
        }
        self.spawn(false, cancel)
    }

    fn gpu_failed(&self, reason: String) {
        log::warn!("Kokoro GPU unavailable, using CPU: {reason}");
        *self.gpu_error.lock().unwrap() = Some(reason);
    }

    fn spawn(&self, gpu: bool, cancel: &CancelToken) -> Result<WorkerProc, TtsError> {
        let c = self.config.lock().unwrap().clone();
        let mut cmd = Command::new(&c.worker_bin);
        cmd.arg("--model")
            .arg(&c.model_file)
            .arg("--threads")
            .arg(c.threads.to_string())
            .arg("--device")
            .arg(if gpu { "gpu" } else { "cpu" });
        let proc = WorkerProc::spawn(cmd, "kokoro", cancel, LOAD_TIMEOUT, c.below_normal)?;
        *self.device.lock().unwrap() = proc.device;
        Ok(proc)
    }

    /// Runs one request on an idle worker, starting one if none is free, and
    /// returns model-rate samples.
    fn run(&self, phonemes: &str, voice_file: &str, cancel: &CancelToken) -> Result<Vec<f32>, TtsError> {
        let epoch = self.epoch.load(Ordering::Acquire);
        let idle = self.idle.lock().unwrap().pop();
        let mut proc = match idle {
            Some(proc) => proc,
            None => self.start(cancel)?,
        };
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let req = serde_json::json!({
            "id": id,
            "phonemes": phonemes,
            "voice": voice_file,
            "speed": 1.0,
        });
        // On a send or wait error the worker is dropped, which kills it:
        // cancelled native inference may not stop on its own (spec §8.4).
        proc.send(&req)?;
        let frame = proc.wait(id, cancel, SYNTH_TIMEOUT)?;
        if frame.status != 0 {
            let error = TtsError::Engine(String::from_utf8_lossy(&frame.payload).into_owned());
            if proc.device != "GPU" {
                self.release(proc, epoch);
            }
            return Err(error);
        }
        self.release(proc, epoch);
        Ok(worker::samples(&frame.payload))
    }

    /// Returns a worker to the idle pool unless it is outdated or surplus.
    fn release(&self, proc: WorkerProc, epoch: u64) {
        let limit = self.config.lock().unwrap().parallel.max(1);
        let mut idle = self.idle.lock().unwrap();
        if self.epoch.load(Ordering::Acquire) == epoch && idle.len() < limit {
            idle.push(proc);
        }
    }
}

impl Engine for KokoroEngine {
    fn id(&self) -> &'static str {
        "kokoro"
    }

    fn device(&self) -> &'static str {
        *self.device.lock().unwrap()
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

        let voice_file = file.to_string_lossy();
        let samples = match self.run(&phonemes, &voice_file, cancel) {
            // A GPU failure mid-reading retries this segment on the CPU. The
            // GPU stays off only if the CPU succeeds where it failed.
            Err(TtsError::Engine(e)) if *self.device.lock().unwrap() == "GPU" => {
                *self.gpu_error.lock().unwrap() = Some(e.clone());
                match self.run(&phonemes, &voice_file, cancel) {
                    Ok(samples) => {
                        self.gpu_failed(e);
                        samples
                    }
                    Err(retry) => {
                        *self.gpu_error.lock().unwrap() = None;
                        return Err(retry);
                    }
                }
            }
            r => r?,
        };
        Ok(Pcm { samples: resample(&samples, model_rate, sample_rate), sample_rate })
    }

    fn unload(&self) {
        let mut idle = self.idle.lock().unwrap();
        self.epoch.fetch_add(1, Ordering::AcqRel);
        idle.clear();
    }

    /// One GPU worker at a time: parallel CPU workers only help on the CPU.
    fn max_parallel(&self, _voice: &str) -> usize {
        let c = self.config.lock().unwrap();
        if c.gpu && self.gpu_error.lock().unwrap().is_none() {
            1
        } else {
            c.parallel.max(1)
        }
    }

    fn set_parallel(&self, jobs: usize) {
        self.config.lock().unwrap().parallel = jobs.max(1);
        self.idle.lock().unwrap().truncate(jobs.max(1));
    }

    fn gpu_fallback(&self) -> Option<String> {
        self.gpu_error.lock().unwrap().clone()
    }

    /// Takes effect on the next worker start and gives the GPU another try.
    fn set_gpu(&self, gpu: bool) {
        let mut c = self.config.lock().unwrap();
        if c.gpu != gpu {
            c.gpu = gpu;
            drop(c);
            *self.gpu_error.lock().unwrap() = None;
            self.unload();
        }
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

    /// Takes effect on the next worker start.
    fn set_below_normal(&self, below_normal: bool) {
        let mut c = self.config.lock().unwrap();
        if c.below_normal != below_normal {
            c.below_normal = below_normal;
            drop(c);
            self.unload();
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// A stand-in worker speaking the frame protocol. `gpu` decides what a
    /// GPU start does: `ok`, fail to `load`, or load and fail every `run`.
    fn fake_worker(gpu: &str) -> Option<PathBuf> {
        std::process::Command::new("python3").arg("--version").output().ok()?;
        let path = std::env::temp_dir().join(format!("speakit-fake-kokoro-{}-{gpu}.py", std::process::id()));
        let script = format!(
            r#"#!/usr/bin/env python3
import json, struct, sys
out = sys.stdout.buffer
def frame(i, status, payload, n):
    out.write(struct.pack('<QII', i, status, n) + payload); out.flush()
gpu = sys.argv[sys.argv.index('--device') + 1] == 'gpu'
if gpu and '{gpu}' == 'load':
    frame(0, 1, b'no adapter', 10); sys.exit(1)
frame(0, 2, b'GPU' if gpu else b'CPU', 3)
for line in sys.stdin:
    req = json.loads(line)
    if gpu and '{gpu}' == 'run':
        frame(req['id'], 1, b'kernel failed', 13)
    else:
        frame(req['id'], 0, struct.pack('<4f', 0.1, 0.2, 0.3, 0.4), 4)
"#
        );
        std::fs::write(&path, script).ok()?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).ok()?;
        Some(path)
    }

    fn engine(gpu_mode: &str) -> Option<KokoroEngine> {
        let engine = KokoroEngine::new(KokoroConfig {
            model_id: "kokoro-test".into(),
            model_name: "Kokoro".into(),
            voice_prefix: "kokoro".into(),
            model_file: "unused.onnx".into(),
            worker_bin: fake_worker(gpu_mode)?,
            voices: vec![KokoroVoice {
                id: "af_heart".into(),
                label: "Heart".into(),
                language: "en-US".into(),
                gender: None,
                g2p: "en-us".into(),
                approximate_pronunciation: false,
                file: "af_heart.bin".into(),
            }],
            threads: 1,
            parallel: 1,
            gpu: true,
            below_normal: true,
            model_rate: 24_000,
        });
        // Phonemes come from espeak-ng before the worker is involved.
        engine.has_phonemizer().then_some(engine)
    }

    fn speak(e: &KokoroEngine) -> Result<Pcm, TtsError> {
        e.synthesize("Hello there.", "kokoro:af_heart", 24_000, &CancelToken::default())
    }

    #[test]
    fn reports_the_gpu_only_once_the_worker_loads_on_it() {
        let Some(e) = engine("ok") else { return };
        assert_eq!(e.device(), "CPU");
        assert_eq!(speak(&e).unwrap().samples.len(), 4);
        assert_eq!(e.device(), "GPU");
        assert_eq!(e.gpu_fallback(), None);

        e.set_gpu(false);
        speak(&e).unwrap();
        assert_eq!(e.device(), "CPU");
    }

    #[test]
    fn falls_back_to_the_cpu_when_the_gpu_cannot_load() {
        let Some(e) = engine("load") else { return };
        assert_eq!(speak(&e).unwrap().samples.len(), 4);
        assert_eq!(e.device(), "CPU");
        assert_eq!(e.gpu_fallback().as_deref(), Some("no adapter"));

        // Changing the setting clears the reason and tries the GPU again.
        e.set_gpu(false);
        assert_eq!(e.gpu_fallback(), None);
        e.set_gpu(true);
        speak(&e).unwrap();
        assert_eq!(e.gpu_fallback().as_deref(), Some("no adapter"));
    }

    #[test]
    fn retries_on_the_cpu_when_the_gpu_fails_mid_reading() {
        let Some(e) = engine("run") else { return };
        assert_eq!(speak(&e).unwrap().samples.len(), 4);
        assert_eq!(e.device(), "CPU");
        assert_eq!(e.gpu_fallback().as_deref(), Some("kernel failed"));
        // Later segments stay on the CPU.
        speak(&e).unwrap();
        assert_eq!(e.device(), "CPU");
    }
}
