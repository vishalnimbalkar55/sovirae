//! Global shortcuts (spec §11). Read actions are always registered; playback
//! actions only while a reading session exists, so ordinary editing chords
//! such as Ctrl+Shift+Arrow keep working in other apps the rest of the time.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::controller::Command;
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub action: &'static str,
    pub binding: String,
    pub playback_only: bool,
    pub registered: bool,
    pub error: Option<String>,
    /// Known uses of this chord elsewhere on this platform.
    pub conflict: Option<&'static str>,
}

#[derive(Default)]
pub struct ShortcutManager {
    /// action → registered shortcut
    active: Mutex<HashMap<&'static str, Shortcut>>,
    errors: Mutex<HashMap<&'static str, String>>,
    last_fired: Mutex<HashMap<&'static str, Instant>>,
    apply_lock: Mutex<()>,
}

impl ShortcutManager {
    /// Records that every shortcut was released outside `apply`.
    pub fn forget_all(&self) {
        let _serial = self.apply_lock.lock().unwrap();
        self.active.lock().unwrap().clear();
    }
}

pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                on_pressed(app, shortcut);
            }
        })
        .build()
}

pub fn parse(binding: &str) -> Result<Shortcut, String> {
    binding.parse::<Shortcut>().map_err(|e| e.to_string())
}

/// Chords the platform or common apps already use (spec §11.1).
pub fn known_conflict(binding: &str) -> Option<&'static str> {
    let b = binding.to_ascii_lowercase().replace("option", "alt").replace("cmd", "command");
    let table: &[(&str, &str)] = if cfg!(target_os = "macos") {
        &[
            ("command+alt+space", "Opens a Finder search window by default."),
            ("command+alt+arrowleft", "Switches tabs in Chrome, Safari, and many editors."),
            ("command+alt+arrowright", "Switches tabs in Chrome, Safari, and many editors."),
        ]
    } else {
        &[
            ("control+shift+arrowleft", "Selects text word by word in most editors."),
            ("control+shift+arrowright", "Selects text word by word in most editors."),
            ("control+shift+arrowup", "Selects or moves paragraphs and lines in Word and code editors."),
            ("control+shift+arrowdown", "Selects or moves paragraphs and lines in Word and code editors."),
            ("control+shift+r", "Hard reload in Chrome."),
            ("control+shift+s", "Save As in many applications."),
        ]
    };
    table.iter().find(|(k, _)| *k == b).map(|(_, v)| *v)
}

/// Validates a recorded binding before saving it.
pub fn validate_binding(binding: &str) -> Result<(), String> {
    let has_modifier = binding.split('+').count() > 1;
    if !has_modifier {
        return Err("Global shortcuts need at least one modifier key.".into());
    }
    parse(binding).map(|_| ())
}

/// Registers exactly the shortcuts the current state calls for.
pub fn apply<R: Runtime>(app: &AppHandle<R>) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let settings = state.settings.lock().unwrap().clone();
    let playback = state.playback_active.load(std::sync::atomic::Ordering::Acquire);
    let gs = app.global_shortcut();

    let mut desired: HashMap<&'static str, Shortcut> = HashMap::new();
    let mut errors = HashMap::new();
    if !settings.hotkeys_paused {
        for (action, binding, playback_only) in settings.shortcuts.entries() {
            if playback_only && !playback {
                continue;
            }
            match parse(binding) {
                Ok(s) => {
                    desired.insert(action, s);
                }
                Err(e) => {
                    errors.insert(action, e);
                }
            }
        }
    }

    // Registration round-trips through the main thread, where the key
    // handler also runs, so never hold `active` while registering.
    let _serial = state.shortcuts.apply_lock.lock().unwrap();
    let mut current = state.shortcuts.active.lock().unwrap().clone();
    let stale: Vec<&'static str> = current
        .iter()
        .filter(|(a, s)| desired.get(*a) != Some(*s))
        .map(|(a, _)| *a)
        .collect();
    for action in stale {
        if let Some(s) = current.remove(action) {
            let _ = gs.unregister(s);
        }
    }
    for (action, shortcut) in desired {
        if current.contains_key(action) {
            continue;
        }
        if current.values().any(|s| *s == shortcut) {
            errors.insert(action, "Another Sovirae action already uses this shortcut.".into());
            continue;
        }
        match gs.register(shortcut) {
            Ok(()) => {
                current.insert(action, shortcut);
            }
            Err(e) => {
                log::warn!("shortcut {action} could not be registered: {e}");
                errors.insert(action, "Another application is using this shortcut, or the system denied it.".into());
            }
        }
    }
    *state.shortcuts.active.lock().unwrap() = current;
    *state.shortcuts.errors.lock().unwrap() = errors;
}

pub fn status<R: Runtime>(app: &AppHandle<R>) -> Vec<ShortcutStatus> {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap();
    let active = state.shortcuts.active.lock().unwrap();
    let errors = state.shortcuts.errors.lock().unwrap();
    settings
        .shortcuts
        .entries()
        .into_iter()
        .map(|(action, binding, playback_only)| ShortcutStatus {
            action,
            binding: binding.to_string(),
            playback_only,
            registered: active.contains_key(action),
            error: errors.get(action).cloned(),
            conflict: known_conflict(binding),
        })
        .collect()
}

fn on_pressed<R: Runtime>(app: &AppHandle<R>, shortcut: &Shortcut) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let action = {
        let active = state.shortcuts.active.lock().unwrap();
        active.iter().find(|(_, s)| *s == shortcut).map(|(a, _)| *a)
    };
    let Some(action) = action else { return };

    // Ignore repeats; limit held skip/speed keys to four actions per second.
    let min_gap = match action {
        "skipBack" | "skipForward" | "speedUp" | "slowDown" => Duration::from_millis(250),
        _ => Duration::from_millis(400),
    };
    {
        let mut last = state.shortcuts.last_fired.lock().unwrap();
        if last.get(action).is_some_and(|t| t.elapsed() < min_gap) {
            return;
        }
        last.insert(action, Instant::now());
    }

    match action {
        "speakClipboard" => crate::actions::speak_clipboard(app.clone(), false),
        "readSelection" => crate::actions::read_selection(app.clone()),
        "playPause" => state.controller.send(Command::TogglePause),
        "stop" => state.controller.send(Command::Stop),
        "skipBack" | "skipForward" => {
            let snap = state.settings.lock().unwrap().sentence_snap;
            let delta_ms = if action == "skipBack" { -10_000 } else { 10_000 };
            state.controller.send(Command::Skip { delta_ms, snap });
        }
        "speedUp" | "slowDown" => {
            let delta = if action == "speedUp" { 0.1 } else { -0.1 };
            crate::actions::change_rate(app, delta);
        }
        _ => {}
    }
}
