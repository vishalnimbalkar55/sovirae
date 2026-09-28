//! Narrow UI → Rust commands. The UI never owns playback state.

use serde::Serialize;
use serde_json::Value;
use speakit_core::{PlaybackSnapshot, SourceKind, SourceReference, SpeakRequest};
use speakit_tts::{Engine, VoiceInfo};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use crate::actions::{self, ClipboardText};
use crate::controller::{Command, DocumentView};
use crate::settings::{round_rate, Settings, Shortcuts};
use crate::shortcuts::{self, ShortcutStatus};
use crate::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialState {
    settings: Settings,
    snapshot: PlaybackSnapshot,
    platform: &'static str,
    engine: &'static str,
}

#[tauri::command]
pub fn get_state(state: State<'_, AppState>) -> InitialState {
    InitialState {
        settings: state.settings.lock().unwrap().clone(),
        snapshot: state.snapshot.lock().unwrap().clone(),
        platform: std::env::consts::OS,
        engine: state.engine.system_id(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorStatus {
    /// This build can run downloaded voices on the GPU.
    gpu_available: bool,
    /// Why the GPU was requested but the CPU is in use.
    fallback: Option<String>,
}

#[tauri::command]
pub fn processor_status(state: State<'_, AppState>) -> ProcessorStatus {
    ProcessorStatus {
        gpu_available: speakit_tts::kokoro::GPU_AVAILABLE,
        fallback: state.engine.gpu_fallback(),
    }
}

#[tauri::command]
pub fn speak_text(state: State<'_, AppState>, text: String, truncate: Option<bool>) {
    state.controller.send(Command::Speak {
        request: SpeakRequest {
            text,
            source: SourceReference { kind: SourceKind::Manual, display_name: Some("Read screen".into()) },
            language_hint: None,
        },
        truncate: truncate.unwrap_or(false),
        reply: None,
    });
}

#[tauri::command]
pub fn speak_clipboard(app: AppHandle, truncate: Option<bool>) {
    actions::speak_clipboard(app, truncate.unwrap_or(false));
}

/// Paste button on the Read screen: reads the clipboard only when clicked.
#[tauri::command]
pub async fn read_clipboard_text() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| match actions::read_clipboard() {
        ClipboardText::Text(t) => Ok(t),
        ClipboardText::Empty => Err("Clipboard is empty. Copy some text and try again.".into()),
        ClipboardText::NotText => Err("Clipboard does not contain text.".into()),
        ClipboardText::Busy => Err("Clipboard is busy. Try again.".into()),
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn control(app: AppHandle, state: State<'_, AppState>, action: String, value: Option<f64>) -> Result<(), String> {
    let c = &state.controller;
    match action.as_str() {
        "play" => c.send(Command::Play),
        "pause" => c.send(Command::Pause),
        "toggle" => c.send(Command::TogglePause),
        "stop" => c.send(Command::Stop),
        "skip" => {
            let delta_ms = value.filter(|v| v.is_finite()).unwrap_or(10_000.0) as i64;
            let snap = state.settings.lock().unwrap().sentence_snap;
            c.send(Command::Skip { delta_ms: delta_ms.clamp(-600_000, 600_000), snap });
        }
        "seekSegment" => {
            let i = value.filter(|v| v.is_finite() && *v >= 0.0).ok_or("invalid segment")?;
            c.send(Command::SeekSegment(i as usize));
        }
        "seekFraction" => {
            let f = value.filter(|v| v.is_finite()).ok_or("invalid position")?;
            c.send(Command::SeekFraction(f));
        }
        "rate" => {
            let r = value.filter(|v| v.is_finite()).ok_or("invalid rate")?;
            update_settings(app, state, serde_json::json!({ "rate": r }))?;
        }
        "volume" => {
            let v = value.filter(|v| v.is_finite()).ok_or("invalid volume")?;
            update_settings(app, state, serde_json::json!({ "volume": v }))?;
        }
        _ => return Err(format!("unknown action {action}")),
    }
    Ok(())
}

#[tauri::command]
pub async fn list_voices(state: State<'_, AppState>) -> Result<Vec<VoiceInfo>, String> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || engine.voices().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn preview_voice(state: State<'_, AppState>, voice: String) {
    state.controller.send(Command::Preview(voice));
}

#[tauri::command]
pub fn get_document(state: State<'_, AppState>) -> Option<DocumentView> {
    state.document.lock().unwrap().clone()
}

/// Applies a partial settings object, persists it, and performs side effects.
#[tauri::command]
pub fn update_settings(app: AppHandle, state: State<'_, AppState>, patch: Value) -> Result<Settings, String> {
    let before = state.settings.lock().unwrap().clone();
    let mut merged = serde_json::to_value(&before).map_err(|e| e.to_string())?;
    let (Value::Object(target), Value::Object(patch)) = (&mut merged, patch) else {
        return Err("settings patch must be an object".into());
    };
    for (k, v) in patch {
        // Shortcuts and pairings change only through their own commands.
        if k != "shortcuts" && k != "pairedExtensions" && target.contains_key(&k) {
            target.insert(k, v);
        }
    }
    let mut next: Settings = serde_json::from_value(merged).map_err(|e| e.to_string())?;
    next.rate = round_rate(next.rate);
    let next = next.sanitize();

    if next.start_at_login != before.start_at_login {
        let al = app.autolaunch();
        let r = if next.start_at_login { al.enable() } else { al.disable() };
        if let Err(e) = r {
            return Err(format!("Start at login could not be changed: {e}"));
        }
    }
    *state.settings.lock().unwrap() = next.clone();
    state.store.save(&next);

    let c = &state.controller;
    if next.rate != before.rate {
        c.send(Command::SetRate(next.rate));
    }
    if next.volume != before.volume {
        c.send(Command::SetVolume(next.volume));
    }
    if next.match_page_language != before.match_page_language {
        c.send(Command::SetMatchLanguage(next.match_page_language));
    }
    if next.chrome_bridge != before.chrome_bridge {
        let a = app.clone();
        std::thread::spawn(move || crate::bridge::apply_enabled(&a));
    }
    if next.voice != before.voice {
        c.send(Command::SetVoice(next.voice.clone()));
    }
    if next.resource_profile != before.resource_profile {
        c.send(Command::SetProfile(next.resource_profile));
        state.engine.set_threads(crate::models::threads_for(next.resource_profile));
        state.engine.set_parallel(crate::models::parallel_for(next.resource_profile));
    }
    if next.processor != before.processor {
        state.engine.set_gpu(next.processor == crate::settings::Processor::Gpu);
    }
    if next.hotkeys_paused != before.hotkeys_paused {
        let a = app.clone();
        std::thread::spawn(move || shortcuts::apply(&a));
        crate::tray::sync(&app);
    }
    if next.player_topmost != before.player_topmost {
        if let Some(w) = app.get_webview_window("player") {
            let _ = w.set_always_on_top(next.player_topmost);
        }
    }
    let _ = app.emit("settings", &next);
    Ok(next)
}

#[tauri::command]
pub async fn get_shortcuts(app: AppHandle) -> Vec<ShortcutStatus> {
    shortcuts::status(&app)
}

#[tauri::command]
pub async fn set_shortcut(app: AppHandle, action: String, binding: Option<String>) -> Result<Vec<ShortcutStatus>, String> {
    let state = app.state::<AppState>();
    let binding = match binding {
        Some(b) => {
            shortcuts::validate_binding(&b)?;
            b
        }
        None => Shortcuts::default()
            .entries()
            .iter()
            .find(|(a, _, _)| *a == action)
            .map(|(_, b, _)| b.to_string())
            .ok_or("unknown action")?,
    };
    {
        let mut s = state.settings.lock().unwrap();
        let normalized = binding.to_ascii_lowercase();
        if let Some((other, _, _)) = s
            .shortcuts
            .entries()
            .iter()
            .find(|(a, b, _)| *a != action && b.to_ascii_lowercase() == normalized)
        {
            return Err(format!("This shortcut is already used by {}.", action_label(other)));
        }
        if !s.shortcuts.set(&action, binding) {
            return Err("unknown action".into());
        }
        state.store.save(&s);
    }
    let a = app.clone();
    tauri::async_runtime::spawn_blocking(move || shortcuts::apply(&a)).await.map_err(|e| e.to_string())?;
    Ok(shortcuts::status(&app))
}

/// Temporarily releases all global shortcuts while the recorder listens.
#[tauri::command]
pub async fn suspend_shortcuts(app: AppHandle, suspended: bool) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let a = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if suspended {
            let _ = a.global_shortcut().unregister_all();
            a.state::<AppState>().shortcuts.forget_all();
        } else {
            shortcuts::apply(&a);
        }
    })
    .await
    .map_err(|e| e.to_string())
}

fn action_label(action: &str) -> &'static str {
    match action {
        "speakClipboard" => "Speak clipboard",
        "readSelection" => "Read selected text",
        "playPause" => "Play/pause",
        "skipBack" => "Skip back",
        "skipForward" => "Skip forward",
        "speedUp" => "Speed up",
        "slowDown" => "Slow down",
        "stop" => "Stop",
        _ => "another action",
    }
}

/// Readable platform label, e.g. "⌘⌥R" on macOS or "Ctrl+Shift+R".
pub fn display_binding(binding: &str) -> String {
    let parts: Vec<&str> = binding.split('+').collect();
    let mac = cfg!(target_os = "macos");
    let label = |p: &str| -> String {
        match p.to_ascii_lowercase().as_str() {
            "command" | "cmd" | "super" => if mac { "⌘".into() } else if cfg!(windows) { "Win".into() } else { "Super".into() },
            "alt" | "option" => if mac { "⌥".into() } else { "Alt".into() },
            "control" | "ctrl" => if mac { "⌃".into() } else { "Ctrl".into() },
            "shift" => if mac { "⇧".into() } else { "Shift".into() },
            "arrowleft" => "←".into(),
            "arrowright" => "→".into(),
            "arrowup" => "↑".into(),
            "arrowdown" => "↓".into(),
            other => {
                let o = other.strip_prefix("key").unwrap_or(other);
                let mut c = o.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            }
        }
    };
    let labels: Vec<String> = parts.iter().map(|p| label(p)).collect();
    if mac { labels.concat() } else { labels.join("+") }
}

#[tauri::command]
pub fn player_expand(app: AppHandle, state: State<'_, AppState>, expanded: bool) {
    if let Some(w) = app.get_webview_window("player") {
        let collapsed = state.settings.lock().unwrap().player_height();
        let (width, height) = if expanded { (620.0, 500.0) } else { (520.0, collapsed) };
        let _ = w.set_resizable(expanded);
        let _ = w.set_size(tauri::LogicalSize::new(width, height));
        crate::platform::set_player_keyable(&w, expanded);
        crate::clamp_player(&app, &w);
    }
}

/// Hides the player when it only showed a notice and no reading exists.
#[tauri::command]
pub fn dismiss_player(app: AppHandle, state: State<'_, AppState>) {
    if state.snapshot.lock().unwrap().session_id == 0 {
        crate::hide_player(&app);
    }
}

#[tauri::command]
pub fn open_main(app: AppHandle, screen: Option<String>) {
    crate::show_main(&app, screen.as_deref());
}

#[tauri::command]
pub fn exit_app(app: AppHandle) {
    crate::exit(&app);
}

#[cfg(test)]
mod tests {
    use super::display_binding;

    #[test]
    fn labels_bindings() {
        if cfg!(target_os = "macos") {
            assert_eq!(display_binding("Command+Alt+R"), "⌘⌥R");
            assert_eq!(display_binding("Command+Alt+ArrowLeft"), "⌘⌥←");
        } else {
            assert_eq!(display_binding("Control+Shift+Space"), "Ctrl+Shift+Space");
        }
    }
}

/// Friendly name for a stored voice ID, e.g. `kokoro:af_heart` → `Heart`.
pub fn voice_display(id: &str) -> String {
    match id.split_once(':') {
        Some((_, v)) => {
            let name = v.split_once('_').map_or(v, |(_, n)| n);
            let mut c = name.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        }
        None => id.split(" (").next().unwrap_or(id).to_string(),
    }
}

#[tauri::command]
pub async fn bridge_status(app: AppHandle) -> crate::bridge::BridgeStatus {
    crate::bridge::status(&app)
}

#[tauri::command]
pub async fn bridge_test(app: AppHandle) -> Result<crate::bridge::TestResult, String> {
    tauri::async_runtime::spawn_blocking(move || crate::bridge::test_connection(&app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn bridge_revoke(app: AppHandle, id: String) {
    crate::bridge::revoke(&app, &id);
}

#[tauri::command]
pub fn bridge_allow_again(app: AppHandle, id: String) {
    crate::bridge::forget_decline(&app, &id);
}

/// Shows the unpacked extension folder in Finder or Explorer for "Load unpacked".
#[tauri::command]
pub fn open_extension_folder(app: AppHandle) -> Result<(), String> {
    let folder = crate::bridge::extension_folder(&app).ok_or("The extension folder is missing from this build.")?;
    crate::platform::open_folder(&folder).map_err(|e| e.to_string())
}
