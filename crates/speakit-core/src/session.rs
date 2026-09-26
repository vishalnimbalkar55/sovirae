//! Session records shared by the desktop, native host, and extension (spec §9.2, §10.1).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    Manual,
    Clipboard,
    Selection,
    Chrome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceReference {
    pub kind: SourceKind,
    /// Short display label such as an origin or app name. Descriptive only.
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakRequest {
    pub text: String,
    pub source: SourceReference,
    pub language_hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatus {
    Idle,
    LoadingVoice,
    Preparing,
    Buffering,
    Playing,
    Paused,
    Recovering,
    Completed,
    Error,
}

impl PlaybackStatus {
    /// A session exists and playback shortcuts should be registered.
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::LoadingVoice
                | Self::Preparing
                | Self::Buffering
                | Self::Playing
                | Self::Paused
                | Self::Recovering
        )
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSnapshot {
    pub session_id: u64,
    pub status: PlaybackStatus,
    pub source: Option<SourceReference>,
    pub segment_id: Option<usize>,
    pub sentence_id: Option<usize>,
    /// Elapsed source-audio time at normal synthesis pace.
    pub position_ms: u64,
    /// Sum of known segment durations plus an estimate for the rest.
    pub duration_ms: u64,
    pub duration_is_final: bool,
    pub rate: f32,
    pub volume: f32,
    pub voice: Option<String>,
    pub device: &'static str,
    pub message: Option<String>,
}

impl PlaybackSnapshot {
    pub fn idle(rate: f32, volume: f32) -> Self {
        Self {
            session_id: 0,
            status: PlaybackStatus::Idle,
            source: None,
            segment_id: None,
            sentence_id: None,
            position_ms: 0,
            duration_ms: 0,
            duration_is_final: false,
            rate,
            volume,
            voice: None,
            device: "CPU",
            message: None,
        }
    }
}
