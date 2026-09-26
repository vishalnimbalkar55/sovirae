//! Data-driven model catalog (spec §7.4).
//!
//! Models live in `catalog/models.json`. Adding a Hugging Face model whose
//! family is already supported is a catalog entry: pin the revision, list
//! every file with its exact byte size and SHA-256, and describe its voices.
//! A new family also needs an engine adapter in `speakit-tts`.

use std::sync::OnceLock;

use serde::Deserialize;

const CATALOG_JSON: &str = include_str!("../catalog/models.json");

/// Engine adapters that exist in this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Kokoro,
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
    /// Phonemizer language (an espeak-ng voice such as `en-us` or `cmn`).
    pub g2p: String,
    /// `approximate` when this build's phonemizer is not the model's
    /// reference pipeline for the language.
    pub pronunciation: Option<String>,
    pub file: FileSpec,
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
    pub artifacts: Vec<Artifact>,
    pub voices: Vec<VoiceSpec>,
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
            let files = m.artifacts.iter().map(|a| &a.file).chain(m.voices.iter().map(|v| &v.file));
            for f in files {
                assert_eq!(f.sha256.len(), 64, "{}", f.path);
                assert!(f.sha256.chars().all(|c| c.is_ascii_hexdigit()), "{}", f.path);
                assert!(f.bytes > 0, "{}", f.path);
                assert!(!f.path.contains(".."), "{}", f.path);
            }
            let mut voice_ids = HashSet::new();
            for v in &m.voices {
                assert!(voice_ids.insert(&v.id), "{}: duplicate voice {}", m.id, v.id);
                assert!(v.language.contains('-'), "{}: {}", m.id, v.language);
                assert!(matches!(v.gender.as_deref(), None | Some("female") | Some("male")));
                assert!(!v.g2p.is_empty(), "{}: {} needs a phonemizer language", m.id, v.id);
                assert!(matches!(v.pronunciation.as_deref(), None | Some("approximate")));
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
    }
}
