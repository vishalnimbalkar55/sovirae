//! Pocket TTS engine (Kyutai): text → isolated Candle worker → 24 kHz PCM →
//! device rate. The model reads raw text (no phonemizer); each voice is a
//! precomputed state file. The worker is started on first use, killed on
//! cancellation or crash, and unloaded when idle (spec §8.4).

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use crate::resample::resample;
use crate::worker::{self, WorkerProc};
use crate::{CancelToken, Engine, Pcm, TtsError, VoiceInfo};

const LOAD_TIMEOUT: Duration = Duration::from_secs(60);
const SYNTH_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct PocketVoice {
    pub id: String,
    pub label: String,
    pub language: String,
    pub gender: Option<String>,
    pub file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PocketConfig {
    /// Catalog model ID and display name.
    pub model_id: String,
    pub model_name: String,
    /// Voice IDs are `<voice_prefix>:<voice>`.
    pub voice_prefix: String,
    pub model_file: PathBuf,
    pub tokenizer_file: PathBuf,
    pub worker_bin: PathBuf,
    pub voices: Vec<PocketVoice>,
    pub threads: usize,
    /// The catalog's per-language text rules and sampling settings.
    pub options: serde_json::Map<String, serde_json::Value>,
    pub model_rate: u32,
}

pub struct PocketEngine {
    config: Mutex<PocketConfig>,
    worker: Mutex<Option<WorkerProc>>,
    next_id: AtomicU64,
}

impl PocketEngine {
    pub fn new(config: PocketConfig) -> Self {
        Self { config: Mutex::new(config), worker: Mutex::new(None), next_id: AtomicU64::new(1) }
    }

    fn spawn(&self, cancel: &CancelToken) -> Result<WorkerProc, TtsError> {
        let c = self.config.lock().unwrap().clone();
        let mut cmd = Command::new(&c.worker_bin);
        cmd.arg("--model")
            .arg(&c.model_file)
            .arg("--tokenizer")
            .arg(&c.tokenizer_file)
            .arg("--threads")
            .arg(c.threads.to_string())
            .arg("--options")
            .arg(serde_json::Value::Object(c.options).to_string());
        WorkerProc::spawn(cmd, "pocket", cancel, LOAD_TIMEOUT)
    }

    /// Runs one request through the worker, starting it if needed, and
    /// returns model-rate samples.
    fn run(&self, text: &str, voice_file: &str, cancel: &CancelToken) -> Result<Vec<f32>, TtsError> {
        let mut guard = self.worker.lock().unwrap();
        if guard.is_none() {
            *guard = Some(self.spawn(cancel)?);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let proc = guard.as_mut().unwrap();
        if let Err(e) = proc.send(&serde_json::json!({ "id": id, "text": text, "voice": voice_file })) {
            *guard = None;
            return Err(e);
        }
        let frame = match proc.wait(id, cancel, SYNTH_TIMEOUT) {
            Ok(f) => f,
            Err(e) => {
                // Cancelled or failed: generation does not stop on its own,
                // so replace the worker (spec §8.4).
                *guard = None;
                return Err(e);
            }
        };
        drop(guard);
        if frame.status != 0 {
            return Err(TtsError::Engine(String::from_utf8_lossy(&frame.payload).into_owned()));
        }
        Ok(worker::samples(&frame.payload))
    }
}

impl Engine for PocketEngine {
    fn id(&self) -> &'static str {
        "pocket"
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
                approximate_pronunciation: false,
            })
            .collect())
    }

    fn max_segment_chars(&self) -> usize {
        // Segments are sentences; the worker splits longer ones into the
        // model's 50-token chunks.
        300
    }

    fn synthesize(&self, text: &str, voice: &str, sample_rate: u32, cancel: &CancelToken) -> Result<Pcm, TtsError> {
        if !text.chars().any(char::is_alphanumeric) {
            return Ok(Pcm { samples: Vec::new(), sample_rate });
        }
        let (file, model_rate) = {
            let c = self.config.lock().unwrap();
            let id = voice.split_once(':').map_or(voice, |(_, v)| v);
            let v = c.voices.iter().find(|v| v.id == id).ok_or_else(|| TtsError::UnknownVoice(voice.into()))?;
            (v.file.clone(), c.model_rate)
        };
        let samples = self.run(text, &file.to_string_lossy(), cancel)?;
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// A stand-in worker speaking the frame protocol. It echoes the
    /// request back as audio: one sample per character of text, and
    /// fails for a voice file named `bad`.
    fn fake_worker() -> Option<PathBuf> {
        std::process::Command::new("python3").arg("--version").output().ok()?;
        let path = std::env::temp_dir().join(format!("speakit-fake-pocket-{}.py", std::process::id()));
        let script = r#"#!/usr/bin/env python3
import json, struct, sys
out = sys.stdout.buffer
def frame(i, status, payload, n):
    out.write(struct.pack('<QII', i, status, n) + payload); out.flush()
args = sys.argv
opts = json.loads(args[args.index('--options') + 1])
assert args[args.index('--tokenizer') + 1].endswith('tokenizer.json')
frame(0, 2, b'CPU', 3)
for line in sys.stdin:
    req = json.loads(line)
    if req['voice'].endswith('bad'):
        msg = b'voice file unreadable'; frame(req['id'], 1, msg, len(msg)); continue
    n = len(req['text']) + (1 if opts.get('removeSemicolons') else 0)
    frame(req['id'], 0, struct.pack('<%df' % n, *([0.5] * n)), n)
"#;
        std::fs::write(&path, script).ok()?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).ok()?;
        Some(path)
    }

    fn engine() -> Option<(PocketEngine, PathBuf)> {
        let dir = std::env::temp_dir().join(format!("speakit-pocket-voices-{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok()?;
        let alba = dir.join("alba.bin");
        std::fs::write(&alba, b"state").ok()?;
        let voice = |id: &str, file: PathBuf| PocketVoice {
            id: id.into(),
            label: id.into(),
            language: "fr".into(),
            gender: None,
            file,
        };
        let mut options = serde_json::Map::new();
        options.insert("removeSemicolons".into(), true.into());
        let engine = PocketEngine::new(PocketConfig {
            model_id: "pocket-tts-fr".into(),
            model_name: "Pocket TTS French".into(),
            voice_prefix: "pocket-fr".into(),
            model_file: "model.safetensors".into(),
            tokenizer_file: "tokenizer.json".into(),
            worker_bin: fake_worker()?,
            voices: vec![voice("alba", alba), voice("bad", dir.join("bad")), voice("gone", dir.join("gone.bin"))],
            threads: 1,
            options,
            model_rate: 24_000,
        });
        Some((engine, dir))
    }

    #[test]
    fn sends_text_and_options_to_the_worker() {
        let Some((e, _)) = engine() else { return };
        let c = CancelToken::default();
        let pcm = e.synthesize("Bonjour.", "pocket-fr:alba", 24_000, &c).unwrap();
        // 8 characters, plus one because the options reached the worker.
        assert_eq!(pcm.samples.len(), 9);
        assert_eq!(e.synthesize("Encore.", "pocket-fr:alba", 48_000, &c).unwrap().samples.len(), 16);
    }

    #[test]
    fn lists_only_voices_on_disk() {
        let Some((e, dir)) = engine() else { return };
        std::fs::write(dir.join("bad"), b"x").unwrap();
        let ids: Vec<String> = e.voices().unwrap().into_iter().map(|v| v.id).collect();
        assert_eq!(ids, ["pocket-fr:alba", "pocket-fr:bad"]);
    }

    #[test]
    fn reports_worker_errors_and_unknown_voices() {
        let Some((e, _)) = engine() else { return };
        let c = CancelToken::default();
        assert!(matches!(e.synthesize("Salut.", "pocket-fr:nobody", 24_000, &c), Err(TtsError::UnknownVoice(_))));
        match e.synthesize("Salut.", "pocket-fr:bad", 24_000, &c) {
            Err(TtsError::Engine(m)) => assert!(m.contains("unreadable"), "{m}"),
            other => panic!("{other:?}"),
        }
        // The worker survives a failed request.
        assert!(e.synthesize("Salut.", "pocket-fr:alba", 24_000, &c).is_ok());
        // Nothing to say: no worker round trip.
        assert!(e.synthesize(" … ", "pocket-fr:alba", 24_000, &c).unwrap().samples.is_empty());
    }

    #[test]
    fn cancellation_is_reported() {
        let Some((e, _)) = engine() else { return };
        let c = CancelToken::default();
        c.cancel();
        assert!(matches!(e.synthesize("Salut.", "pocket-fr:alba", 24_000, &c), Err(TtsError::Cancelled)));
    }
}
