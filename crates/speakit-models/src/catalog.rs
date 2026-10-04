//! Data-driven model catalog (spec §7.4).
//!
//! Models live in `catalog/models.json`. Adding a Hugging Face model whose
//! family is already supported is a catalog entry: pin the revision, list
//! every file with its exact byte size and SHA-256, and describe its voices.
//! A new family also needs an engine adapter in `speakit-tts`.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const CATALOG_JSON: &str = include_str!("../catalog/models.json");

/// Engine adapters that exist in this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Kokoro,
    Pocket,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSpec {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub label: String,
    pub description: String,
    pub file: FileSpec,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSpec {
    pub id: String,
    pub label: String,
    /// BCP-47 tag such as `en-US`.
    pub language: String,
    /// `female`, `male`, or absent when the provider does not say.
    pub gender: Option<String>,
    /// Phonemizer language (an espeak-ng voice such as `en-us` or `cmn`),
    /// for families that phonemize text.
    #[serde(default)]
    pub g2p: String,
    /// `approximate` when this build's phonemizer is not the model's
    /// reference pipeline for the language.
    pub pronunciation: Option<String>,
    /// License of the recording the voice was made from, when it differs
    /// from the model's (SPDX ID, or `unverified`).
    pub license: Option<String>,
    pub file: FileSpec,
}

/// A script marker the model itself understands, such as `[laugh]`
/// for models trained with non-verbal tags. The Studio lists them and
/// passes them through to the engine unchanged.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureSpec {
    /// The word inside the brackets, e.g. `laugh`.
    pub id: String,
    pub label: String,
    pub description: String,
    /// How to write it in a script, e.g. `[laugh]`.
    pub example: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSpec {
    pub id: String,
    pub name: String,
    pub family: Family,
    /// Voice IDs of this model are `<voicePrefix>:<voice id>`.
    pub voice_prefix: String,
    pub description: String,
    pub repo: String,
    pub revision: String,
    pub license: String,
    pub license_url: String,
    pub card_url: String,
    /// Sample rate of the waveform the model produces.
    pub sample_rate: u32,
    /// External tools the family needs, e.g. `espeak-ng`.
    #[serde(default)]
    pub requires: Vec<String>,
    /// Files every artifact needs, e.g. a tokenizer.
    #[serde(default)]
    pub files: Vec<FileSpec>,
    /// Engine settings for this model, passed to its worker as JSON.
    #[serde(default)]
    pub options: serde_json::Map<String, serde_json::Value>,
    pub artifacts: Vec<Artifact>,
    pub voices: Vec<VoiceSpec>,
    /// Script markers this model honours (none for models that read plain
    /// text; the Studio then offers only app-level markers such as pauses).
    #[serde(default)]
    pub features: Vec<FeatureSpec>,
}

impl ModelSpec {
    pub fn url(&self, path: &str) -> String {
        format!("https://huggingface.co/{}/resolve/{}/{}", self.repo, self.revision, path)
    }

    pub fn voices_bytes(&self) -> u64 {
        self.voices.iter().map(|v| v.file.bytes).sum()
    }

    pub fn artifact(&self, id: &str) -> Option<&Artifact> {
        self.artifacts.iter().find(|a| a.id == id)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogFile {
    schema_version: u32,
    models: Vec<ModelSpec>,
}

/// Every downloadable model in this build.
pub fn models() -> &'static [ModelSpec] {
    static CATALOG: OnceLock<Vec<ModelSpec>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let file: CatalogFile = serde_json::from_str(CATALOG_JSON).expect("models.json is valid");
        assert_eq!(file.schema_version, 1, "unsupported catalog schema");
        file.models
    })
}

pub fn find(id: &str) -> Option<&'static ModelSpec> {
    models().iter().find(|m| m.id == id)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn catalog_is_well_formed() {
        let models = models();
        assert!(!models.is_empty());
        let mut ids = HashSet::new();
        let mut prefixes = HashSet::new();
        for m in models {
            assert!(ids.insert(&m.id), "duplicate model id {}", m.id);
            assert!(prefixes.insert(&m.voice_prefix), "duplicate voice prefix {}", m.voice_prefix);
            assert!(!m.voice_prefix.contains(':') && !m.voice_prefix.is_empty());
            assert_eq!(m.revision.len(), 40, "{}: pin a full commit", m.id);
            assert!(!m.artifacts.is_empty() && !m.voices.is_empty(), "{}", m.id);
            let files = m.artifacts.iter().map(|a| &a.file).chain(&m.files).chain(m.voices.iter().map(|v| &v.file));
            for f in files {
                assert_eq!(f.sha256.len(), 64, "{}", f.path);
                assert!(f.sha256.chars().all(|c| c.is_ascii_hexdigit()), "{}", f.path);
                assert!(f.bytes > 0, "{}", f.path);
                assert!(!f.path.contains(".."), "{}", f.path);
            }
            let mut voice_ids = HashSet::new();
            // Support files land next to the model file, so names must differ.
            let mut names: Vec<&str> = m.files.iter().map(|f| f.path.rsplit('/').next().unwrap()).collect();
            names.extend(m.artifacts.iter().map(|a| a.file.path.rsplit('/').next().unwrap()));
            assert_eq!(names.len(), names.iter().collect::<HashSet<_>>().len(), "{}: file names collide", m.id);
            for v in &m.voices {
                assert!(voice_ids.insert(&v.id), "{}: duplicate voice {}", m.id, v.id);
                // BCP-47: a language, optionally with a region (`en`, `en-US`).
                let (lang, region) = v.language.split_once('-').unwrap_or((&v.language, "US"));
                assert!(matches!(lang.len(), 2 | 3) && lang.chars().all(|c| c.is_ascii_lowercase()), "{}: {}", m.id, v.language);
                assert!(region.len() == 2 && region.chars().all(|c| c.is_ascii_uppercase()), "{}: {}", m.id, v.language);
                assert!(matches!(v.gender.as_deref(), None | Some("female") | Some("male")));
                if m.family == Family::Kokoro {
                    assert!(!v.g2p.is_empty(), "{}: {} needs a phonemizer language", m.id, v.id);
                }
                assert!(matches!(v.pronunciation.as_deref(), None | Some("approximate")));
                if let Some(l) = &v.license {
                    assert!(!l.is_empty() && !l.contains(' '), "{}: {} license {l}", m.id, v.id);
                }
            }
            if m.family == Family::Pocket {
                assert_eq!(m.files.len(), 1, "{}: needs its tokenizer", m.id);
                assert!(m.files[0].path.ends_with("tokenizer.json"), "{}", m.id);
                assert!(m.voices.iter().all(|v| v.file.path.ends_with(".safetensors")), "{}", m.id);
            }
        }
        let k = find("kokoro-82m-v1.0").unwrap();
        assert_eq!(k.voice_prefix, "kokoro");
        assert!(k.voices.iter().any(|v| v.id == "af_heart" && v.gender.as_deref() == Some("female")));
        // Every published Kokoro voice is listed, not only English ones.
        assert_eq!(k.voices.len(), 55);
        for id in ["am_echo", "am_eric", "ef_dora", "jf_alpha", "zf_xiaoxiao", "hf_alpha", "pm_alex"] {
            assert!(k.voices.iter().any(|v| v.id == id), "missing {id}");
        }

        // Pocket TTS: every language Kyutai ships (all languages requested
        // 2026-10-04), each with the same 27 voices, Kyutai's default voice
        // first, and the language's own tokenizer, weights, and text rules.
        let pocket: Vec<_> = models.iter().filter(|m| m.family == Family::Pocket).collect();
        let langs: Vec<&str> = pocket.iter().map(|m| m.voices[0].language.as_str()).collect();
        assert_eq!(langs, ["en", "fr", "de", "es", "it", "pt", "nl"]);
        let en = find("pocket-tts-en").unwrap();
        assert_eq!(en.voice_prefix, "pocket-en");
        for m in &pocket {
            let lang = &m.voices[0].language;
            assert_eq!(m.voice_prefix, format!("pocket-{lang}"));
            assert_eq!(m.voices.len(), 27, "{}", m.id);
            assert_eq!(m.voices[0].id, "alba");
            assert!(m.voices.iter().all(|v| &v.language == lang && v.license.is_some()), "{}", m.id);
            assert!(m.files[0].path.contains("/tokenizer.json"), "{}", m.id);
            // Each language has its own files: nothing is shared with English.
            if m.id != en.id {
                assert_ne!(m.artifacts[0].file.sha256, en.artifacts[0].file.sha256, "{}", m.id);
                assert_ne!(m.files[0].sha256, en.files[0].sha256, "{}", m.id);
                assert!(m.options.contains_key("replaceCharacters"), "{}: text rules", m.id);
            }
            // Recordings from non-commercial datasets are marked as such.
            for id in ["cosette", "jean"] {
                assert_eq!(m.voices.iter().find(|v| v.id == id).unwrap().license.as_deref(), Some("CC-BY-NC-4.0"));
            }
        }
        // Rules copied from pocket_tts/config/<language>.yaml.
        assert_eq!(find("pocket-tts-fr").unwrap().options["removeSemicolons"], true);
        assert_eq!(find("pocket-tts-es").unwrap().options["replaceCharacters"]["\u{bf}"], "");
    }
}
