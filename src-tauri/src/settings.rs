//! Rust-owned settings persisted as JSON in the user's app-config directory
//! (spec §4.5, §14).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceProfile {
    Eco,
    Balanced,
    Performance,
}

impl ResourceProfile {
    /// Desired lookahead at the current speed (spec §8.2).
    pub fn lookahead_ms(self) -> u64 {
        match self {
            Self::Eco => 8_000,
            Self::Balanced => 15_000,
            Self::Performance => 30_000,
        }
    }

    /// Maximum synthesized PCM kept in memory.
    pub fn pcm_budget_bytes(self) -> usize {
        match self {
            Self::Eco => 32 << 20,
            Self::Balanced => 64 << 20,
            Self::Performance => 128 << 20,
        }
    }

    /// How long the audio device stays open after a reading ends.
    pub fn idle_release_secs(self) -> u64 {
        match self {
            Self::Eco => 30,
            Self::Balanced => 60,
            Self::Performance => 180,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Shortcuts {
    pub speak_clipboard: String,
    pub read_selection: String,
    pub play_pause: String,
    pub skip_back: String,
    pub skip_forward: String,
    pub speed_up: String,
    pub slow_down: String,
    pub stop: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        // Spec §11.1: Windows/Linux Ctrl+Shift layout, macOS Command+Option.
        let m = if cfg!(target_os = "macos") { "Command+Alt" } else { "Control+Shift" };
        Self {
            speak_clipboard: format!("{m}+R"),
            read_selection: format!("{m}+S"),
            play_pause: format!("{m}+Space"),
            skip_back: format!("{m}+ArrowLeft"),
            skip_forward: format!("{m}+ArrowRight"),
            speed_up: format!("{m}+ArrowUp"),
            slow_down: format!("{m}+ArrowDown"),
            stop: format!("{m}+X"),
        }
    }
}

impl Shortcuts {
    /// (action id, binding, playback-only)
    pub fn entries(&self) -> [(&'static str, &str, bool); 8] {
        [
            ("speakClipboard", &self.speak_clipboard, false),
            ("readSelection", &self.read_selection, false),
            ("playPause", &self.play_pause, true),
            ("skipBack", &self.skip_back, true),
            ("skipForward", &self.skip_forward, true),
            ("speedUp", &self.speed_up, true),
            ("slowDown", &self.slow_down, true),
            ("stop", &self.stop, true),
        ]
    }

    pub fn set(&mut self, action: &str, binding: String) -> bool {
        let slot = match action {
            "speakClipboard" => &mut self.speak_clipboard,
            "readSelection" => &mut self.read_selection,
            "playPause" => &mut self.play_pause,
            "skipBack" => &mut self.skip_back,
            "skipForward" => &mut self.skip_forward,
            "speedUp" => &mut self.speed_up,
            "slowDown" => &mut self.slow_down,
            "stop" => &mut self.stop,
            _ => return false,
        };
        *slot = binding;
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: String,
    pub reading_font_size: u8,
    pub reading_font: String,
    pub rate: f32,
    pub volume: f32,
    pub voice: Option<String>,
    pub resource_profile: ResourceProfile,
    pub start_at_login: bool,
    pub keep_running: bool,
    pub start_minimized: bool,
    pub hotkeys_paused: bool,
    pub sentence_snap: bool,
    pub follow_reading: bool,
    pub player_topmost: bool,
    pub player_position: Option<(f64, f64)>,
    pub shortcuts: Shortcuts,
    /// Accept requests from the Chrome extension (spec §13).
    pub chrome_bridge: bool,
    /// Extension IDs the user allowed to send text.
    pub paired_extensions: Vec<String>,
    /// Read web pages with a voice in the page's language when one exists.
    pub match_page_language: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            reading_font_size: 18,
            reading_font: "serif".into(),
            rate: 1.0,
            volume: 0.8,
            voice: None,
            resource_profile: ResourceProfile::Balanced,
            start_at_login: false,
            keep_running: true,
            start_minimized: false,
            hotkeys_paused: false,
            sentence_snap: false,
            follow_reading: true,
            player_topmost: true,
            player_position: None,
            shortcuts: Shortcuts::default(),
            chrome_bridge: true,
            paired_extensions: Vec::new(),
            match_page_language: true,
        }
    }
}

impl Settings {
    /// Clamps values that may have been edited by hand.
    pub fn sanitize(mut self) -> Self {
        self.rate = round_rate(self.rate);
        self.volume = if self.volume.is_finite() { self.volume.clamp(0.0, 1.0) } else { 0.8 };
        self.reading_font_size = self.reading_font_size.clamp(14, 28);
        if !["system", "light", "dark"].contains(&self.theme.as_str()) {
            self.theme = "system".into();
        }
        if !["serif", "sans"].contains(&self.reading_font.as_str()) {
            self.reading_font = "serif".into();
        }
        if !self.start_at_login {
            self.start_minimized = false;
        }
        self.paired_extensions.retain(|id| id.len() == 32 && id.bytes().all(|b| (b'a'..=b'p').contains(&b)));
        self.paired_extensions.dedup();
        self
    }
}

/// Playback speed: 0.5–3.0× in 0.1 steps (spec §4.5).
pub fn round_rate(rate: f32) -> f32 {
    if !rate.is_finite() {
        return 1.0;
    }
    ((rate.clamp(0.5, 3.0) * 10.0).round()) / 10.0
}

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Self { path: dir.join("settings.json") }
    }

    pub fn load(&self) -> Settings {
        std::fs::read(&self.path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Settings>(&b).ok())
            .unwrap_or_default()
            .sanitize()
    }

    /// Writes atomically so a crash never leaves a truncated file.
    pub fn save(&self, settings: &Settings) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = self.path.with_extension("json.tmp");
        match serde_json::to_vec_pretty(settings) {
            Ok(bytes) => {
                if std::fs::write(&tmp, bytes).is_ok() {
                    let _ = std::fs::rename(&tmp, &self.path);
                }
            }
            Err(e) => log::error!("settings serialization failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_is_clamped_and_stepped() {
        assert_eq!(round_rate(0.1), 0.5);
        assert_eq!(round_rate(1.04), 1.0);
        assert_eq!(round_rate(1.25), 1.3);
        assert_eq!(round_rate(9.0), 3.0);
        assert_eq!(round_rate(f32::NAN), 1.0);
    }

    #[test]
    fn unknown_fields_fall_back_to_defaults() {
        let s: Settings = serde_json::from_str(r#"{"rate": 7, "theme": "neon"}"#).unwrap();
        let s = s.sanitize();
        assert_eq!(s.rate, 3.0);
        assert_eq!(s.theme, "system");
        assert!(s.keep_running);
    }
}
