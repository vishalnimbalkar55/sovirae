//! Actions shared by shortcuts, the tray, and UI commands.

use std::time::Duration;

use speakit_core::{SourceKind, SourceReference, SpeakRequest};
use tauri::{AppHandle, Manager, Runtime};

use crate::controller::{Command, Notice};
use crate::settings::round_rate;
use crate::AppState;

pub enum ClipboardText {
    Text(String),
    Empty,
    NotText,
    Busy,
}

/// Reads Unicode text with a short bounded retry for contention (spec §6.1).
/// Never modifies the clipboard.
pub fn read_clipboard() -> ClipboardText {
    let mut last = ClipboardText::Busy;
    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(60));
        }
        let mut cb = match arboard::Clipboard::new() {
            Ok(cb) => cb,
            Err(_) => continue,
        };
        match cb.get_text() {
            Ok(t) if t.trim().is_empty() => return ClipboardText::Empty,
            Ok(t) => return ClipboardText::Text(t),
            Err(arboard::Error::ContentNotAvailable) => {
                // Distinguish an empty clipboard from non-text content.
                last = if cb.get_image().is_ok() { ClipboardText::NotText } else { ClipboardText::Empty };
                return last;
            }
            Err(arboard::Error::ClipboardOccupied) => last = ClipboardText::Busy,
            Err(_) => last = ClipboardText::NotText,
        }
    }
    last
}

pub fn notify<R: Runtime>(app: &AppHandle<R>, code: &'static str, message: &str) {
    crate::host_notice(app, &Notice { code, message: message.into(), action: None });
}

pub fn speak_clipboard<R: Runtime>(app: AppHandle<R>, truncate: bool) {
    // Clipboard access can block briefly; keep it off the main thread.
    std::thread::spawn(move || {
        let Some(state) = app.try_state::<AppState>() else { return };
        match read_clipboard() {
            ClipboardText::Text(text) => state.controller.send(Command::Speak {
                request: SpeakRequest {
                    text,
                    source: SourceReference { kind: SourceKind::Clipboard, display_name: Some("Clipboard".into()) },
                    language_hint: None,
                },
                truncate,
            }),
            ClipboardText::Empty => notify(&app, "NO_TEXT", "Clipboard is empty. Copy some text and try again."),
            ClipboardText::NotText => notify(&app, "NOT_TEXT", "Clipboard does not contain text."),
            ClipboardText::Busy => notify(&app, "CLIPBOARD_BUSY", "Clipboard is busy. Try again."),
        }
    });
}

/// Direct selection capture is delivered in step 8; until then explain the
/// reliable fallback instead of guessing (spec §6.2, §11.2).
pub fn read_selection<R: Runtime>(app: AppHandle<R>) {
    let chord = app
        .try_state::<AppState>()
        .map(|s| crate::commands::display_binding(&s.settings.lock().unwrap().shortcuts.speak_clipboard))
        .unwrap_or_default();
    notify(
        &app,
        "CAPTURE_UNAVAILABLE",
        &format!("Reading a selection directly isn't available yet. Copy the text, then press {chord}."),
    );
}

pub fn change_rate<R: Runtime>(app: &AppHandle<R>, delta: f32) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let rate = {
        let mut s = state.settings.lock().unwrap();
        s.rate = round_rate(s.rate + delta);
        state.store.save(&s);
        s.rate
    };
    state.controller.send(Command::SetRate(rate));
}
