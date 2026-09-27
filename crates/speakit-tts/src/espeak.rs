//! English grapheme-to-phoneme conversion for Kokoro through espeak-ng,
//! mapped into Kokoro's (misaki) phoneme alphabet.
//!
//! The mapping follows misaki's `EspeakFallback` (Apache-2.0,
//! github.com/hexgrad/misaki). misaki's dictionary lookup is not ported yet,
//! so every word currently takes the espeak path. espeak-ng itself is
//! GPL-3.0 and runs as a separate process; see spec §7.1.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::TtsError;

/// espeak IPA → misaki symbols, longest keys first (misaki E2M).
const E2M: &[(&str, &str)] = &[
    ("ʔˌn\u{329}", "ʔn"),
    ("ʔn\u{329}", "ʔn"),
    ("a^ɪ", "I"),
    ("a^ʊ", "W"),
    ("d^ʒ", "ʤ"),
    ("e^ɪ", "A"),
    ("t^ʃ", "ʧ"),
    ("ɔ^ɪ", "Y"),
    ("ə^l", "ᵊl"),
    ("ʲo", "jo"),
    ("ʲə", "jə"),
    ("ʲ", ""),
    ("ɚ", "əɹ"),
    ("\u{303}", ""),
    ("e", "A"),
    ("r", "ɹ"),
    ("x", "k"),
    ("ç", "k"),
    ("ɐ", "ə"),
    ("ɬ", "l"),
];

/// Punctuation Kokoro understands; it shapes pauses and intonation.
const PUNCT: &[char] = &[';', ':', ',', '.', '!', '?', '—', '…', '"', '(', ')', '“', '”'];
const OPENING: &[char] = &['(', '“'];

#[derive(Debug, Clone)]
pub struct Phonemizer {
    bin: PathBuf,
}

impl Phonemizer {
    /// Finds espeak-ng: `SOVIRAE_ESPEAK`, a bundled copy next to the app,
    /// then common install locations and `PATH`.
    pub fn find() -> Option<Self> {
        let exe = if cfg!(windows) { "espeak-ng.exe" } else { "espeak-ng" };
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(p) = std::env::var("SOVIRAE_ESPEAK") {
            candidates.push(p.into());
        }
        if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
            candidates.push(dir.join(exe));
            // Windows bundles it in a folder with its data; in development
            // scripts/prepare-windows.mjs stages it under target/.
            candidates.push(dir.join("espeak-ng").join(exe));
            candidates.push(dir.join("../windows-bundle/espeak-ng").join(exe));
        }
        if cfg!(windows) {
            // Where the official eSpeak NG installer puts it.
            for var in ["ProgramFiles", "ProgramFiles(x86)"] {
                if let Ok(dir) = std::env::var(var) {
                    candidates.push(Path::new(&dir).join("eSpeak NG").join(exe));
                }
            }
        } else {
            for p in ["/opt/homebrew/bin/espeak-ng", "/usr/local/bin/espeak-ng", "/usr/bin/espeak-ng"] {
                candidates.push(p.into());
            }
        }
        if let Some(path) = std::env::var_os("PATH") {
            candidates.extend(std::env::split_paths(&path).map(|dir| dir.join(exe)));
        }
        candidates.into_iter().find(|p| p.is_file()).map(|bin| Self { bin })
    }

    pub fn path(&self) -> &Path {
        &self.bin
    }

    /// Converts text to Kokoro phonemes, keeping punctuation. `language` is
    /// an espeak-ng voice: `en-us`/`en-gb` use misaki's English rules, any
    /// other language misaki's general espeak mapping.
    pub fn phonemize(&self, text: &str, language: &str) -> Result<String, TtsError> {
        let pieces = split(&normalize(text));
        let words: Vec<&str> = pieces
            .iter()
            .filter_map(|p| match p {
                Piece::Text(t) => Some(t.as_str()),
                Piece::Punct(_) => None,
            })
            .collect();
        let mut phonemes = self.run_batch(&words, language)?;
        if phonemes.len() != words.len() {
            // espeak split a line; fall back to one call per piece.
            phonemes = words.iter().map(|w| self.run_batch(&[w], language).map(|v| v.join(" "))).collect::<Result<_, _>>()?;
        }
        let mut ph = phonemes.into_iter().map(|p| to_kokoro(&strip_language_flags(&p), language));
        let mut out = String::new();
        for piece in &pieces {
            match piece {
                Piece::Text(_) => {
                    let p = ph.next().unwrap_or_default();
                    if p.is_empty() {
                        continue;
                    }
                    if !out.is_empty() && !out.ends_with(' ') && !out.ends_with(OPENING) {
                        out.push(' ');
                    }
                    out.push_str(&p);
                }
                Piece::Punct(c) => {
                    if OPENING.contains(c) && !out.is_empty() && !out.ends_with(' ') {
                        out.push(' ');
                    }
                    out.push(*c);
                }
            }
        }
        Ok(out.trim().to_string())
    }

    fn run_batch(&self, lines: &[&str], language: &str) -> Result<Vec<String>, TtsError> {
        if lines.is_empty() {
            return Ok(Vec::new());
        }
        let mut cmd = Command::new(&self.bin);
        // A bundled copy has no installer registry entry pointing at its
        // data, and crashes without one.
        if let Some(dir) = self.bin.parent().filter(|d| d.join("espeak-ng-data").is_dir()) {
            cmd.env("ESPEAK_DATA_PATH", dir);
        }
        let mut child = crate::no_console(&mut cmd)
            .args(["-q", "--ipa", "--tie=^", "-v", language])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| TtsError::Engine(format!("espeak-ng could not start: {e}")))?;
        let input = lines.join("\n") + "\n";
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
        let out = child.wait_with_output().map_err(|e| TtsError::Engine(e.to_string()))?;
        if !out.status.success() {
            return Err(TtsError::Engine("espeak-ng failed".into()));
        }
        Ok(String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim().to_string()).collect())
    }
}

#[derive(Debug, PartialEq)]
enum Piece {
    Text(String),
    Punct(char),
}

fn normalize(text: &str) -> String {
    text.replace("...", "…")
        .replace(" – ", " — ")
        .replace(" - ", " — ")
        .replace(['«', '„'], "“")
        .replace('»', "”")
        .replace(['‘', '’'], "'")
        .replace(['¡', '¿'], "")
        // Full-width CJK punctuation → the marks Kokoro knows.
        .replace(['，', '、'], ", ")
        .replace('。', ". ")
        .replace('？', "? ")
        .replace('！', "! ")
        .replace('：', ": ")
        .replace('；', "; ")
        .replace('（', " (")
        .replace('）', ") ")
        .replace(['「', '『'], "“")
        .replace(['」', '』'], "”")
}

/// Splits text into word runs and punctuation. Sentence marks inside tokens
/// ("3.5", "U.S.", "1,000") stay in the word run for espeak to read.
fn split(text: &str) -> Vec<Piece> {
    let chars: Vec<char> = text.chars().collect();
    let mut pieces = Vec::new();
    let mut run = String::new();
    for (i, &c) in chars.iter().enumerate() {
        let next = chars.get(i + 1).copied();
        let boundary = next.is_none_or(|n| n.is_whitespace() || PUNCT.contains(&n));
        let is_break = match c {
            ';' | ':' | ',' | '.' | '!' | '?' => boundary && !(c == '.' && is_abbreviation(&run)),
            c => PUNCT.contains(&c),
        };
        if is_break {
            if !run.trim().is_empty() {
                pieces.push(Piece::Text(run.trim().to_string()));
            }
            run.clear();
            pieces.push(Piece::Punct(c));
        } else {
            run.push(c);
        }
    }
    if !run.trim().is_empty() {
        pieces.push(Piece::Text(run.trim().to_string()));
    }
    pieces
}

/// "U.S." or "e.g." ending the run: keep the final dot with the word.
fn is_abbreviation(run: &str) -> bool {
    let last = run.rsplit(char::is_whitespace).next().unwrap_or("");
    last.contains('.') && last.chars().filter(|c| c.is_alphabetic()).count() <= 4
}

/// espeak marks a switch into another script's language as "(en)…(ja)";
/// text punctuation is split out beforehand, so parentheses here are flags.
fn strip_language_flags(ipa: &str) -> String {
    let mut out = String::with_capacity(ipa.len());
    let mut depth = 0;
    for c in ipa.chars() {
        match c {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            c if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

fn to_kokoro(ipa: &str, language: &str) -> String {
    match language {
        "en-us" => to_misaki(ipa, false),
        "en-gb" => to_misaki(ipa, true),
        _ => to_misaki_general(ipa),
    }
}

/// misaki `EspeakG2P` mapping for non-English languages.
fn to_misaki_general(ipa: &str) -> String {
    const E2M_GENERAL: &[(&str, &str)] = &[
        ("a^ɪ", "I"),
        ("a^ʊ", "W"),
        ("d^z", "ʣ"),
        ("d^ʒ", "ʤ"),
        ("e^ɪ", "A"),
        ("o^ʊ", "O"),
        ("ə^ʊ", "Q"),
        ("s^s", "S"),
        ("t^s", "ʦ"),
        ("t^ʃ", "ʧ"),
        ("ɔ^ɪ", "Y"),
    ];
    let mut ps = ipa.to_string();
    for (from, to) in E2M_GENERAL {
        ps = ps.replace(from, to);
    }
    ps.replace('^', "").replace('-', "")
}

fn to_misaki(ipa: &str, british: bool) -> String {
    let mut ps = ipa.to_string();
    for (from, to) in E2M {
        ps = ps.replace(from, to);
    }
    // Syllabic consonants: "n̩" → "ᵊn".
    let mut s = String::with_capacity(ps.len());
    let chars: Vec<char> = ps.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i + 1] == '\u{329}' && !chars[i].is_whitespace() {
            s.push('ᵊ');
            s.push(chars[i]);
            i += 2;
        } else {
            if chars[i] != '\u{329}' {
                s.push(chars[i]);
            }
            i += 1;
        }
    }
    ps = s;
    if british {
        ps = ps.replace("e^ə", "ɛː").replace("iə", "ɪə").replace("ə^ʊ", "Q");
    } else {
        ps = ps
            .replace("o^ʊ", "O")
            .replace("ɜːɹ", "ɜɹ")
            .replace("ɜː", "ɜɹ")
            .replace("ɪə", "iə")
            .replace('ː', "");
    }
    ps = ps.replace('o', "ɔ").replace('ɾ', "T").replace('ʔ', "t");
    ps.replace('^', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_punctuation_but_not_numbers_or_abbreviations() {
        let p = split("It cost 3.5 dollars, in the U.S. today. Really?");
        assert_eq!(
            p,
            vec![
                Piece::Text("It cost 3.5 dollars".into()),
                Piece::Punct(','),
                Piece::Text("in the U.S. today".into()),
                Piece::Punct('.'),
                Piece::Text("Really".into()),
                Piece::Punct('?'),
            ]
        );
    }

    #[test]
    fn maps_espeak_ipa_to_misaki() {
        assert_eq!(to_misaki("həlˈo^ʊ wˈɜːld", false), "həlˈO wˈɜɹld");
        assert_eq!(to_misaki("a^ɪm ɡˌo^ʊɪŋ", false), "Im ɡˌOɪŋ");
        assert_eq!(to_misaki("həlˈə^ʊ", true), "həlˈQ");
        assert_eq!(to_misaki("bˈʌɾɚ", false), "bˈʌTəɹ");
    }

    #[test]
    fn maps_other_languages_and_strips_flags() {
        assert_eq!(strip_language_flags("(^e^n)ˈe^ɪt^ʃ(^j^a) kˈo"), "ˈe^ɪt^ʃ kˈo");
        assert_eq!(to_kokoro("ˈo^ʊla t^saɪ", "it"), "ˈOla ʦaɪ");
        assert_eq!(to_kokoro("bɔ̃ʒuʁ", "fr-fr"), "bɔ̃ʒuʁ");
    }

    #[test]
    #[ignore = "requires espeak-ng"]
    fn phonemizes_every_catalog_language() {
        let p = Phonemizer::find().expect("espeak-ng installed");
        for (lang, text) in [
            ("es", "Hola, ¿cómo estás?"),
            ("fr-fr", "Bonjour, comment allez-vous ?"),
            ("hi", "नमस्ते, आप कैसे हैं?"),
            ("it", "Ciao, come stai?"),
            ("ja", "こんにちは、元気ですか？"),
            ("pt-br", "Olá, tudo bem?"),
            ("cmn", "你好，你好吗？"),
        ] {
            let out = p.phonemize(text, lang).unwrap();
            println!("{lang}: {out}");
            assert!(!out.is_empty() && !out.contains('^') && !out.contains('('), "{lang}: {out}");
        }
    }

    #[test]
    #[ignore = "requires espeak-ng"]
    fn phonemizes_with_punctuation() {
        let p = Phonemizer::find().expect("espeak-ng installed");
        let out = p.phonemize("Hello world! Is it 3.5 today?", "en-us").unwrap();
        assert!(out.starts_with("həlˈO wˈɜɹld!"), "{out}");
        assert!(out.ends_with('?'), "{out}");
        assert!(!out.contains('^'));
    }
}
