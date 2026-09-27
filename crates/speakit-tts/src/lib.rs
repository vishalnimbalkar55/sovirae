//! Speech engines. Every engine turns one segment of text into mono PCM at
//! the requested sample rate, in a separate process that can be killed on
//! cancellation (spec §5.1, §8.4).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use serde::Serialize;
use thiserror::Error;

pub mod espeak;
pub mod kokoro;
#[cfg(target_os = "macos")]
pub mod macos_say;
pub mod pocket;
pub mod resample;
#[cfg(windows)]
pub mod windows_speech;
mod worker;

#[derive(Debug, Error)]
pub enum TtsError {
    #[error("Synthesis was cancelled.")]
    Cancelled,
    #[error("The voice \"{0}\" is not available.")]
    UnknownVoice(String),
    #[error("The speech engine failed: {0}")]
    Engine(String),
    #[error("No speech engine is available on this platform.")]
    Unsupported,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceInfo {
    /// Stable identifier passed back to `synthesize`.
    pub id: String,
    pub name: String,
    /// BCP-47 style tag, e.g. `en-US`.
    pub language: String,
    /// `female`, `male`, or `None` when the provider does not say.
    pub gender: Option<String>,
    /// Model that owns the voice: a catalog model ID, or `system`.
    pub model: String,
    /// Display name of the engine or model.
    pub engine: String,
    /// Evaluated as suitable for long reading; not a quality grade.
    pub recommended: bool,
    /// Pronunciation uses a stand-in phonemizer for this language.
    pub approximate_pronunciation: bool,
}

#[derive(Debug, Clone)]
pub struct Pcm {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl Pcm {
    pub fn duration_ms(&self) -> u64 {
        self.samples.len() as u64 * 1000 / self.sample_rate.max(1) as u64
    }
}

/// Shared cancellation flag checked by long-running engine calls.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub trait Engine: Send + Sync {
    fn id(&self) -> &'static str;
    /// "CPU", or the verified accelerator actually in use.
    fn device(&self) -> &'static str;
    fn voices(&self) -> Result<Vec<VoiceInfo>, TtsError>;
    /// Longest segment, in chars, this engine should be given.
    fn max_segment_chars(&self) -> usize;
    fn synthesize(
        &self,
        text: &str,
        voice: &str,
        sample_rate: u32,
        cancel: &CancelToken,
    ) -> Result<Pcm, TtsError>;
    /// Releases loaded models and worker processes; the next call reloads.
    fn unload(&self) {}
    /// Applies an inference thread budget (spec §8.2), where supported.
    fn set_threads(&self, _threads: usize) {}
    /// Device used for `voice`; engines that own one device ignore it.
    fn device_for(&self, _voice: &str) -> &'static str {
        self.device()
    }
    /// Requests the GPU for later work, where supported (spec §7.3).
    fn set_gpu(&self, _gpu: bool) {}
    /// Why a requested GPU is not in use, if it failed.
    fn gpu_fallback(&self) -> Option<String> {
        None
    }
}

/// Routes each voice to its engine. Voices of downloaded models are
/// `<prefix>:<voice>`; anything else belongs to the zero-download system
/// engine. Any number of models can be registered at once.
pub struct Registry {
    system: Box<dyn Engine>,
    models: RwLock<Vec<(String, Arc<dyn Engine>)>>,
}

impl Registry {
    pub fn new(system: Box<dyn Engine>) -> Self {
        Self { system, models: RwLock::new(Vec::new()) }
    }

    /// Registers, replaces, or (with `None`) removes the engine for a prefix.
    pub fn set_model(&self, prefix: &str, engine: Option<Arc<dyn Engine>>) {
        let old = {
            let mut models = self.models.write().unwrap();
            let old = models.iter().position(|(p, _)| p == prefix).map(|i| models.remove(i).1);
            if let Some(e) = engine {
                models.push((prefix.to_string(), e));
            }
            old
        };
        if let Some(old) = old {
            old.unload();
        }
    }

    pub fn has_model(&self, prefix: &str) -> bool {
        self.models.read().unwrap().iter().any(|(p, _)| p == prefix)
    }

    pub fn system_id(&self) -> &'static str {
        self.system.id()
    }

    fn route(&self, voice: &str) -> Option<Arc<dyn Engine>> {
        let (prefix, _) = voice.split_once(':')?;
        self.models.read().unwrap().iter().find(|(p, _)| p == prefix).map(|(_, e)| e.clone())
    }

    fn all_models(&self) -> Vec<Arc<dyn Engine>> {
        self.models.read().unwrap().iter().map(|(_, e)| e.clone()).collect()
    }
}

impl Engine for Registry {
    fn id(&self) -> &'static str {
        "registry"
    }

    fn device(&self) -> &'static str {
        "CPU"
    }

    fn voices(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        let mut all = self.system.voices()?;
        for m in self.all_models() {
            all.extend(m.voices()?);
        }
        Ok(all)
    }

    fn max_segment_chars(&self) -> usize {
        self.all_models()
            .iter()
            .map(|m| m.max_segment_chars())
            .fold(self.system.max_segment_chars(), usize::min)
    }

    fn synthesize(&self, text: &str, voice: &str, sample_rate: u32, cancel: &CancelToken) -> Result<Pcm, TtsError> {
        if voice.contains(':') {
            let engine = self.route(voice).ok_or_else(|| TtsError::UnknownVoice(voice.into()))?;
            engine.synthesize(text, voice, sample_rate, cancel)
        } else {
            self.system.synthesize(text, voice, sample_rate, cancel)
        }
    }

    fn unload(&self) {
        self.system.unload();
        for m in self.all_models() {
            m.unload();
        }
    }

    fn set_threads(&self, threads: usize) {
        for m in self.all_models() {
            m.set_threads(threads);
        }
    }

    fn device_for(&self, voice: &str) -> &'static str {
        match self.route(voice) {
            Some(engine) => engine.device(),
            None => self.system.device(),
        }
    }

    fn set_gpu(&self, gpu: bool) {
        for m in self.all_models() {
            m.set_gpu(gpu);
        }
    }

    fn gpu_fallback(&self) -> Option<String> {
        self.all_models().iter().find_map(|m| m.gpu_fallback())
    }
}

/// The zero-download starter engine for the current OS. Platforms without a
/// verified engine get one with no voices, so the UI shows "Choose or
/// download a voice" instead of failing.
pub fn system_engine() -> Box<dyn Engine> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos_say::SayEngine::new())
    }
    #[cfg(windows)]
    {
        Box::new(windows_speech::WindowsEngine::new())
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        Box::new(NoEngine)
    }
}

/// Keeps a helper process (worker, espeak-ng) from opening a console
/// window when started by the windowed app on Windows.
pub(crate) fn no_console(cmd: &mut std::process::Command) -> &mut std::process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Decodes a WAV stream to mono float PCM at its own sample rate.
#[allow(dead_code)] // used by the platform system engines
pub(crate) fn pcm_from_wav<R: std::io::Read>(source: R) -> Result<Pcm, TtsError> {
    let reader = hound::WavReader::new(source).map_err(|e| TtsError::Engine(e.to_string()))?;
    let spec = reader.spec();
    let channels = spec.channels.max(1) as usize;
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.into_samples::<f32>().filter_map(Result::ok).collect(),
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .into_samples::<i32>()
                .filter_map(Result::ok)
                .map(|s| s as f32 * scale)
                .collect()
        }
    };
    let samples = if channels == 1 {
        interleaved
    } else {
        interleaved.chunks(channels).map(|f| f.iter().sum::<f32>() / channels as f32).collect()
    };
    Ok(Pcm { samples, sample_rate: spec.sample_rate })
}

pub struct NoEngine;

impl Engine for NoEngine {
    fn id(&self) -> &'static str {
        "none"
    }
    fn device(&self) -> &'static str {
        "CPU"
    }
    fn voices(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        Ok(Vec::new())
    }
    fn max_segment_chars(&self) -> usize {
        400
    }
    fn synthesize(&self, _: &str, _: &str, _: u32, _: &CancelToken) -> Result<Pcm, TtsError> {
        Err(TtsError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(&'static str, &'static str);

    impl Engine for Fake {
        fn id(&self) -> &'static str {
            self.0
        }
        fn device(&self) -> &'static str {
            "CPU"
        }
        fn voices(&self) -> Result<Vec<VoiceInfo>, TtsError> {
            Ok(vec![VoiceInfo {
                id: format!("{}{}", self.1, self.0),
                name: self.0.into(),
                language: "en-US".into(),
                gender: None,
                model: self.0.into(),
                engine: self.0.into(),
                recommended: false,
                approximate_pronunciation: false,
            }])
        }
        fn max_segment_chars(&self) -> usize {
            if self.0 == "b" { 200 } else { 400 }
        }
        fn synthesize(&self, _: &str, _: &str, sample_rate: u32, _: &CancelToken) -> Result<Pcm, TtsError> {
            Ok(Pcm { samples: vec![self.0.len() as f32; 1], sample_rate })
        }
    }

    #[test]
    fn routes_voices_across_several_models() {
        let r = Registry::new(Box::new(Fake("sys", "")));
        r.set_model("a", Some(Arc::new(Fake("a", "a:"))));
        r.set_model("b", Some(Arc::new(Fake("b", "b:"))));
        let ids: Vec<String> = r.voices().unwrap().into_iter().map(|v| v.id).collect();
        assert_eq!(ids, ["sys", "a:a", "b:b"]);
        assert_eq!(r.max_segment_chars(), 200);
        let c = CancelToken::default();
        assert!(r.synthesize("x", "a:a", 48_000, &c).is_ok());
        assert!(r.synthesize("x", "sys", 48_000, &c).is_ok());
        assert!(matches!(r.synthesize("x", "missing:v", 48_000, &c), Err(TtsError::UnknownVoice(_))));

        r.set_model("a", None);
        assert!(!r.has_model("a") && r.has_model("b"));
        assert!(matches!(r.synthesize("x", "a:a", 48_000, &c), Err(TtsError::UnknownVoice(_))));
    }
}
