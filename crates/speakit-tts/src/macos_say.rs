//! macOS system voices through `/usr/bin/say`, which renders offline to a
//! float WAV at the output device's sample rate, so no resampling is needed.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::{CancelToken, Engine, Pcm, TtsError, VoiceInfo};

const SAY: &str = "/usr/bin/say";

/// Built-in novelty voices: listed, but never chosen as the default.
const NOVELTY: &[&str] = &[
    "Albert", "Bad News", "Bahh", "Bells", "Boing", "Bubbles", "Cellos", "Wobble", "Good News",
    "Jester", "Organ", "Superstar", "Trinoids", "Whisper", "Zarvox", "Fred", "Junior", "Kathy",
    "Ralph",
];

/// Voices that read long passages comfortably in informal listening.
const RECOMMENDED: &[&str] = &["Samantha", "Daniel", "Moira", "Karen", "Tessa", "Rishi"];

pub struct SayEngine {
    counter: AtomicU64,
}

impl SayEngine {
    pub fn new() -> Self {
        Self { counter: AtomicU64::new(0) }
    }

    fn temp_path(&self) -> PathBuf {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("speakit-{}-{n}.wav", std::process::id()))
    }
}

impl Default for SayEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Gender as reported by macOS for (voice name, language), cached once.
fn system_gender(name: &str, language: &str) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static GENDERS: OnceLock<HashMap<(String, String), &'static str>> = OnceLock::new();
    let map = GENDERS.get_or_init(|| {
        use objc2_avf_audio::{AVSpeechSynthesisVoice, AVSpeechSynthesisVoiceGender};
        let mut map = HashMap::new();
        // SAFETY: class method returning an autoreleased array of voices.
        let voices = unsafe { AVSpeechSynthesisVoice::speechVoices() };
        for v in voices.iter() {
            // SAFETY: plain property reads on a live voice object.
            let (name, lang, gender) = unsafe { (v.name().to_string(), v.language().to_string(), v.gender()) };
            let g = match gender {
                AVSpeechSynthesisVoiceGender::Female => "female",
                AVSpeechSynthesisVoiceGender::Male => "male",
                _ => continue,
            };
            map.insert((name, lang), g);
        }
        map
    });
    if let Some(g) = map.get(&(name.to_string(), language.to_string())) {
        return Some(g.to_string());
    }
    // Apple's Eloquence voices report "unspecified"; their names identify them.
    const ELOQUENCE: &[(&str, &str)] = &[
        ("Eddy", "male"), ("Flo", "female"), ("Grandma", "female"), ("Grandpa", "male"),
        ("Reed", "male"), ("Rocko", "male"), ("Sandy", "female"), ("Shelley", "female"),
    ];
    ELOQUENCE.iter().find(|(n, _)| *n == name).map(|(_, g)| g.to_string())
}

/// Parses one line of `say -v '?'`, e.g.
/// `Eddy (English (US)) en_US    # Hello! My name is Eddy.`
pub fn parse_voice_line(line: &str) -> Option<(String, String)> {
    let head = line.split('#').next()?.trim_end();
    let (name, locale) = head.rsplit_once(char::is_whitespace)?;
    let name = name.trim();
    if name.is_empty() || !locale.contains('_') {
        return None;
    }
    Some((name.to_string(), locale.replace('_', "-")))
}

/// `say` treats `[[ … ]]` as embedded commands; page text must never control
/// the engine, so break those delimiters apart.
fn neutralize_commands(text: &str) -> String {
    text.replace("[[", "[ [").replace("]]", "] ]")
}

impl Engine for SayEngine {
    fn id(&self) -> &'static str {
        "macos-system"
    }

    fn device(&self) -> &'static str {
        "CPU"
    }

    fn voices(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        let out = Command::new(SAY)
            .args(["-v", "?"])
            .output()
            .map_err(|e| TtsError::Engine(e.to_string()))?;
        let list = String::from_utf8_lossy(&out.stdout);
        let mut voices: Vec<VoiceInfo> = list
            .lines()
            .filter_map(parse_voice_line)
            .map(|(name, language)| {
                let base = name.split(" (").next().unwrap_or(&name);
                VoiceInfo {
                    recommended: !NOVELTY.contains(&base)
                        && (RECOMMENDED.contains(&base) || name.contains("Enhanced") || name.contains("Premium")),
                    gender: system_gender(base, &language),
                    id: name.clone(),
                    name,
                    language,
                    model: "system".into(),
                    engine: "System voice".into(),
                    approximate_pronunciation: false,
                }
            })
            .collect();
        voices.sort_by(|a, b| b.recommended.cmp(&a.recommended).then(a.name.cmp(&b.name)));
        Ok(voices)
    }

    fn max_segment_chars(&self) -> usize {
        400
    }

    fn synthesize(
        &self,
        text: &str,
        voice: &str,
        sample_rate: u32,
        cancel: &CancelToken,
    ) -> Result<Pcm, TtsError> {
        if cancel.is_cancelled() {
            return Err(TtsError::Cancelled);
        }
        let path = self.temp_path();
        let mut child = Command::new(SAY)
            .arg("-v")
            .arg(voice)
            .arg("-o")
            .arg(&path)
            .arg("--file-format=WAVE")
            .arg(format!("--data-format=LEF32@{sample_rate}"))
            .args(["-f", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TtsError::Engine(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(neutralize_commands(text).as_bytes());
        }

        let status = loop {
            if cancel.is_cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&path);
                return Err(TtsError::Cancelled);
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(e) => return Err(TtsError::Engine(e.to_string())),
            }
        };

        if !status.success() {
            let mut err = String::new();
            if let Some(mut stderr) = child.stderr.take() {
                use std::io::Read;
                let _ = stderr.read_to_string(&mut err);
            }
            let _ = std::fs::remove_file(&path);
            if err.contains("Voice") || err.contains("voice") {
                return Err(TtsError::UnknownVoice(voice.to_string()));
            }
            return Err(TtsError::Engine(err.trim().to_string()));
        }

        let result = read_wav(&path);
        let _ = std::fs::remove_file(&path);
        result
    }
}

fn read_wav(path: &PathBuf) -> Result<Pcm, TtsError> {
    let reader = hound::WavReader::open(path).map_err(|e| TtsError::Engine(e.to_string()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_voice_lines() {
        assert_eq!(
            parse_voice_line("Eddy (English (US)) en_US    # Hello! My name is Eddy."),
            Some(("Eddy (English (US))".into(), "en-US".into()))
        );
        assert_eq!(
            parse_voice_line("Samantha            en_US    # Hello!"),
            Some(("Samantha".into(), "en-US".into()))
        );
        assert_eq!(parse_voice_line(""), None);
    }

    #[test]
    fn neutralizes_embedded_commands() {
        assert_eq!(neutralize_commands("a [[slnc 9000]] b"), "a [ [slnc 9000] ] b");
    }

    #[test]
    #[ignore = "invokes the real system voice"]
    fn synthesizes_offline() {
        let e = SayEngine::new();
        let voice = e.voices().unwrap().into_iter().next().unwrap();
        let pcm = e.synthesize("Hello world.", &voice.id, 48_000, &CancelToken::default()).unwrap();
        assert_eq!(pcm.sample_rate, 48_000);
        assert!(pcm.duration_ms() > 300);
    }

    #[test]
    #[ignore = "invokes the real system voice"]
    fn cancels() {
        let e = SayEngine::new();
        let c = CancelToken::default();
        c.cancel();
        assert!(matches!(e.synthesize("Hi.", "Samantha", 48_000, &c), Err(TtsError::Cancelled)));
    }
}

#[cfg(test)]
mod gender_tests {
    use super::*;

    #[test]
    fn reports_gender_from_macos() {
        let voices = SayEngine::new().voices().unwrap();
        let samantha = voices.iter().find(|v| v.id == "Samantha").expect("Samantha is built in");
        assert_eq!(samantha.gender.as_deref(), Some("female"));
        if let Some(daniel) = voices.iter().find(|v| v.id == "Daniel") {
            assert_eq!(daniel.gender.as_deref(), Some("male"));
        }
        let with_gender = voices.iter().filter(|v| v.gender.is_some()).count();
        println!("{with_gender} of {} system voices report a gender", voices.len());
    }
}
