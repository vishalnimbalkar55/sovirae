//! Menu-bar/tray menu (spec §11.4). It never reads the clipboard on a timer.

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use std::sync::OnceLock;

use tauri::{AppHandle, Manager};

use crate::controller::Command;
use crate::AppState;

const TRAY_ID: &str = "speakit";

static PAUSE_ITEM: OnceLock<CheckMenuItem<tauri::Wry>> = OnceLock::new();

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let paused = app.state::<AppState>().settings.lock().unwrap().hotkeys_paused;
    let pause_item = CheckMenuItem::with_id(app, "pause_hotkeys", "Pause shortcuts", true, paused, None::<&str>)?;
    let _ = PAUSE_ITEM.set(pause_item.clone());
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", "Open Sovirae", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "speak_clipboard", "Speak clipboard", true, None::<&str>)?,
            &MenuItem::with_id(app, "toggle", "Play/Pause", true, None::<&str>)?,
            &MenuItem::with_id(app, "stop", "Stop", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &pause_item,
            &MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "exit", "Quit Sovirae", true, None::<&str>)?,
        ],
    )?;

    let icon = Image::from_bytes(include_bytes!("../icons/tray@2x.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Sovirae")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let state = app.state::<AppState>();
            match event.id.as_ref() {
                "open" => crate::show_main(app, None),
                "settings" => crate::show_main(app, Some("settings")),
                "speak_clipboard" => crate::actions::speak_clipboard(app.clone(), false),
                "toggle" => state.controller.send(Command::TogglePause),
                "stop" => state.controller.send(Command::Stop),
                "pause_hotkeys" => {
                    let paused = !state.settings.lock().unwrap().hotkeys_paused;
                    let _ = crate::commands::update_settings(
                        app.clone(),
                        app.state::<AppState>(),
                        serde_json::json!({ "hotkeysPaused": paused }),
                    );
                }
                "exit" => crate::exit(app),
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

/// Keeps the Pause hotkeys check mark in sync with settings.
pub fn sync(app: &AppHandle) {
    let paused = app.state::<AppState>().settings.lock().unwrap().hotkeys_paused;
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(if paused { "Sovirae: shortcuts paused" } else { "Sovirae" }));
    }
    if let Some(item) = PAUSE_ITEM.get() {
        let _ = item.set_checked(paused);
    }
}
