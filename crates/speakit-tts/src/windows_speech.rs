//! Windows system voices through WinRT `Windows.Media.SpeechSynthesis`, the
//! offline voices listed under Settings → Time & language → Speech. Each
//! segment renders to an in-memory WAV that is resampled to the output rate.

use std::sync::OnceLock;
use std::time::Duration;

use windows::core::HSTRING;
use windows::Media::SpeechSynthesis::{SpeechSynthesizer, VoiceGender, VoiceInformation};
use windows::Storage::Streams::DataReader;
use windows_future::AsyncStatus;

use crate::{resample::resample, CancelToken, Engine, Pcm, TtsError, VoiceInfo};

/// English voices that read long passages comfortably.
const RECOMMENDED: &[&str] = &["Aria", "Jenny", "Guy", "Zira", "David", "Mark", "Hazel", "George", "Susan"];

pub struct WindowsEngine;

impl WindowsEngine {
    pub fn new() -> Self {
        ensure_mta();
        Self
    }
}

impl Default for WindowsEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// WinRT calls need a COM apartment. Keeping the process MTA alive lets any
/// thread (synthesis, commands) use the speech APIs without its own setup.
fn ensure_mta() {
    static MTA: OnceLock<()> = OnceLock::new();
    MTA.get_or_init(|| {
        // SAFETY: plain COM call; the cookie is intentionally never released.
        if let Err(e) = unsafe { windows::Win32::System::Com::CoIncrementMTAUsage() } {
            log::warn!("could not start the COM apartment for speech: {e}");
        }
    });
}

fn engine_err(e: windows::core::Error) -> TtsError {
    TtsError::Engine(e.message().to_string())
}

/// "Microsoft David" → "David".
fn short_name(display: &str) -> &str {
    display.strip_prefix("Microsoft ").unwrap_or(display)
}

fn all_voices() -> Result<Vec<VoiceInformation>, TtsError> {
    ensure_mta();
    let list = SpeechSynthesizer::AllVoices().map_err(engine_err)?;
    Ok(list.into_iter().collect())
}

/// Waits for a WinRT operation, cancelling it if the token fires.
macro_rules! wait {
    ($op:expr, $cancel:expr) => {{
        let op = $op;
        loop {
            if $cancel.is_cancelled() {
                let _ = op.Cancel();
                return Err(TtsError::Cancelled);
            }
            match op.Status().map_err(engine_err)? {
                AsyncStatus::Completed => break op.GetResults().map_err(engine_err)?,
                AsyncStatus::Started => std::thread::sleep(Duration::from_millis(5)),
                AsyncStatus::Canceled => return Err(TtsError::Cancelled),
                _ => {
                    let code = op.ErrorCode().map(|c| c.message()).unwrap_or_default();
                    return Err(TtsError::Engine(format!("Windows speech failed: {code}")));
                }
            }
        }
    }};
}

impl Engine for WindowsEngine {
    fn id(&self) -> &'static str {
        "windows-system"
    }

    fn device(&self) -> &'static str {
        "CPU"
    }

    fn voices(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        let mut voices: Vec<VoiceInfo> = all_voices()?
            .iter()
            .filter_map(|v| {
                let display = v.DisplayName().ok()?.to_string();
                let language = v.Language().ok()?.to_string();
                let gender = match v.Gender().ok()? {
                    VoiceGender::Female => Some("female".to_string()),
                    VoiceGender::Male => Some("male".to_string()),
                    _ => None,
                };
                let name = short_name(&display).to_string();
                Some(VoiceInfo {
                    recommended: language.starts_with("en") && RECOMMENDED.contains(&name.as_str()),
                    gender,
                    id: display,
                    name,
                    language,
                    model: "system".into(),
                    engine: "System voice".into(),
                    approximate_pronunciation: false,
                })
            })
            .collect();
        voices.sort_by(|a, b| b.recommended.cmp(&a.recommended).then(a.name.cmp(&b.name)));
        Ok(voices)
    }

    fn max_segment_chars(&self) -> usize {
        400
    }

    fn synthesize(&self, text: &str, voice: &str, sample_rate: u32, cancel: &CancelToken) -> Result<Pcm, TtsError> {
        if cancel.is_cancelled() {
            return Err(TtsError::Cancelled);
        }
        let info = all_voices()?
            .into_iter()
            .find(|v| v.DisplayName().is_ok_and(|n| n == voice))
            .ok_or_else(|| TtsError::UnknownVoice(voice.to_string()))?;
        let synth = SpeechSynthesizer::new().map_err(engine_err)?;
        synth.SetVoice(&info).map_err(engine_err)?;

        // Plain-text synthesis: page text is never interpreted as SSML.
        let stream = wait!(synth.SynthesizeTextToStreamAsync(&HSTRING::from(text)).map_err(engine_err)?, cancel);
        let size = u32::try_from(stream.Size().map_err(engine_err)?)
            .map_err(|_| TtsError::Engine("The rendered speech was too large.".into()))?;
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0).map_err(engine_err)?).map_err(engine_err)?;
        let loaded = wait!(reader.LoadAsync(size).map_err(engine_err)?, cancel);
        let mut wav = vec![0u8; loaded as usize];
        reader.ReadBytes(&mut wav).map_err(engine_err)?;

        let pcm = crate::pcm_from_wav(std::io::Cursor::new(wav))?;
        Ok(Pcm { samples: resample(&pcm.samples, pcm.sample_rate, sample_rate), sample_rate })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortens_display_names() {
        assert_eq!(short_name("Microsoft David"), "David");
        assert_eq!(short_name("Contoso Voice"), "Contoso Voice");
    }

    #[test]
    #[ignore = "invokes the real system voice"]
    fn synthesizes_offline() {
        let e = WindowsEngine::new();
        let voice = e.voices().unwrap().into_iter().next().expect("Windows has at least one voice");
        let pcm = e.synthesize("Hello world.", &voice.id, 48_000, &CancelToken::default()).unwrap();
        assert_eq!(pcm.sample_rate, 48_000);
        assert!(pcm.duration_ms() > 300);
    }

    #[test]
    #[ignore = "invokes the real system voice"]
    fn cancels() {
        let e = WindowsEngine::new();
        let voice = e.voices().unwrap().into_iter().next().unwrap();
        let c = CancelToken::default();
        c.cancel();
        assert!(matches!(e.synthesize("Hi.", &voice.id, 48_000, &c), Err(TtsError::Cancelled)));
    }
}
