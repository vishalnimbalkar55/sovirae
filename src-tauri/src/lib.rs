//! SpeakIt desktop shell: Tauri startup, windows, tray, and the bridge
//! between the session controller and the React UI.

mod actions;
mod bridge;
mod commands;
mod controller;
mod models;
mod platform;
mod settings;
mod shortcuts;
mod studio;
mod tray;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use speakit_core::PlaybackSnapshot;
use speakit_tts::{Engine, Registry};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, RunEvent, Runtime, WebviewWindow, WindowEvent};

use controller::{Command, Controller, DocumentView, Envelope, Host, Notice};
use settings::{Settings, Store};

pub struct AppState {
    pub controller: Controller,
    pub engine: Arc<Registry>,
    pub models: models::ModelManager,
    pub settings: Mutex<Settings>,
    pub store: Store,
    pub snapshot: Mutex<PlaybackSnapshot>,
    pub document: Mutex<Option<DocumentView>>,
    pub shortcuts: shortcuts::ShortcutManager,
    pub playback_active: AtomicBool,
    /// The main window waits for its page to finish loading before it is
    /// shown, so a launch never presents a blank window.
    pub show_pending: AtomicBool,
    pub bridge: bridge::Bridge,
    pub studio: studio::Studio,
}

struct AppHost {
    app: AppHandle,
}

impl Host for AppHost {
    fn snapshot(&self, snapshot: &PlaybackSnapshot) {
        if let Some(state) = self.app.try_state::<AppState>() {
            *state.snapshot.lock().unwrap() = snapshot.clone();
        }
        let _ = self.app.emit("playback", snapshot);
        bridge::on_snapshot(&self.app, snapshot);
    }

    fn notice(&self, notice: &Notice) {
        host_notice(&self.app, notice);
    }

    fn document(&self, document: &DocumentView) {
        if let Some(state) = self.app.try_state::<AppState>() {
            *state.document.lock().unwrap() = Some(document.clone());
        }
        let _ = self.app.emit("document", document);
    }

    fn envelope(&self, envelope: &Envelope) {
        let _ = self.app.emit_to("player", "envelope", envelope);
    }

    fn show_player(&self) {
        show_player(&self.app);
    }

    fn hide_player(&self) {
        hide_player(&self.app);
    }

    fn playback_active(&self, active: bool) {
        if let Some(state) = self.app.try_state::<AppState>() {
            state.playback_active.store(active, Ordering::Release);
        }
        shortcuts::apply(&self.app);
    }
}

/// Notices go to every window; the player appears passively so hotkey
/// errors are visible even when no reading is active (spec §15).
pub(crate) fn host_notice<R: Runtime>(app: &AppHandle<R>, notice: &Notice) {
    let _ = app.emit("notice", notice);
    let main_focused = app
        .get_webview_window("main")
        .and_then(|w| w.is_focused().ok())
        .unwrap_or(false);
    if !main_focused {
        show_player(app);
    }
}

pub(crate) fn show_player<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("player") {
        if !w.is_visible().unwrap_or(false) {
            clamp_player(app, &w);
        }
        platform::show_passive(&w);
    }
}

pub(crate) fn hide_player<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("player") {
        let _ = w.hide();
        let _ = app.emit("player-collapse", ());
        let height = app.try_state::<AppState>().map_or(112.0, |s| s.settings.lock().unwrap().player_height());
        let _ = w.set_size(tauri::LogicalSize::new(520.0, height));
        platform::set_player_keyable(&w, false);
    }
    if let Some(state) = app.try_state::<AppState>() {
        state.store.save(&state.settings.lock().unwrap());
    }
}

/// Restores the remembered position, or bottom-centre of the primary
/// display, clamped to a display that still exists (spec §10.2).
pub(crate) fn clamp_player<R: Runtime>(app: &AppHandle<R>, w: &WebviewWindow<R>) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let saved = state.settings.lock().unwrap().player_position;
    let monitors = w.available_monitors().unwrap_or_default();
    let size = w.outer_size().unwrap_or_default();
    let fits = |x: i32, y: i32| {
        monitors.iter().any(|m| {
            let (p, s) = (m.position(), m.size());
            x >= p.x && y >= p.y && x + 40 <= p.x + s.width as i32 && y + 40 <= p.y + s.height as i32
        })
    };
    if let Some((x, y)) = saved {
        let (x, y) = (x as i32, y as i32);
        if fits(x, y) {
            let _ = w.set_position(PhysicalPosition::new(x, y));
            return;
        }
    }
    if let Ok(Some(m)) = w.primary_monitor() {
        let area = m.work_area();
        let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
        let bottom_gap = (96.0 * m.scale_factor()) as i32;
        let y = area.position.y + area.size.height as i32 - size.height as i32 - bottom_gap;
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
}

pub(crate) fn show_main<R: Runtime>(app: &AppHandle<R>, screen: Option<&str>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        if let Err(e) = w.show() {
            log::error!("main: show failed: {e}");
        }
        let _ = w.set_focus();
    }
    if let Some(screen) = screen {
        let _ = app.emit_to("main", "navigate", screen);
    }
}

pub(crate) fn exit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<AppState>() {
        state.controller.send(Command::Shutdown);
        state.store.save(&state.settings.lock().unwrap());
    }
    app.exit(0);
}

pub fn run() {
    let builder = tauri::Builder::default()
        // Single-instance must be registered first; a second launch focuses
        // the running app instead of starting another process.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app, None)))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(shortcuts::plugin())
        .plugin(tauri_plugin_dialog::init())
        // Diagnostics (spec §14): warnings and errors from the engines,
        // downloads, and the bridge go to the app's log directory
        // (Logs/com.sovirae.desktop on macOS, AppData\Local\...\logs on
        // Windows), capped so the file never grows unbounded.
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: Some("sovirae".into()) }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stderr),
                ])
                .level(log::LevelFilter::Info)
                .level_for("tao", log::LevelFilter::Warn)
                .level_for("wry", log::LevelFilter::Warn)
                .max_file_size(2 << 20)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let store = Store::new(app.path().app_config_dir()?);
            let settings = store.load();
            let engine = Arc::new(Registry::new(speakit_tts::system_engine()));
            models::register_installed(&handle, &engine, settings.resource_profile, settings.processor);
            let controller = controller::spawn(
                engine.clone() as Arc<dyn Engine>,
                controller::Config {
                    match_language: settings.match_page_language,
                    rate: settings.rate,
                    volume: settings.volume,
                    voice: settings.voice.clone(),
                    profile: settings.resource_profile,
                    pronunciations: settings.pronunciations.clone(),
                },
                AppHost { app: handle.clone() },
            );
            // `--background` comes from the Chrome host starting the app on demand.
            let start_hidden = (settings.start_minimized && std::env::args().any(|a| a == "--minimized"))
                || std::env::args().any(|a| a == "--background");
            let topmost = settings.player_topmost;
            app.manage(AppState {
                controller,
                engine,
                snapshot: Mutex::new(PlaybackSnapshot::idle(settings.rate, settings.volume)),
                settings: Mutex::new(settings),
                store,
                document: Mutex::new(None),
                shortcuts: Default::default(),
                models: Default::default(),
                playback_active: AtomicBool::new(false),
                show_pending: AtomicBool::new(!start_hidden),
                bridge: Default::default(),
                studio: Default::default(),
            });

            log::info!("Sovirae {} started", app.package_info().version);
            tray::create(&handle)?;

            if let Some(player) = app.get_webview_window("player") {
                let _ = player.set_always_on_top(topmost);
            }
            // Shown from `on_page_load` below; if the page never reports a
            // finished load, the window still appears so the failure is seen.
            if !start_hidden {
                let h = handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(2500));
                    if h.try_state::<AppState>().is_some_and(|s| s.show_pending.swap(false, Ordering::SeqCst)) {
                        log::warn!("main: page did not finish loading; showing anyway");
                        show_main(&h, None);
                    }
                });
            }
            // Registration round-trips through the main thread.
            let h = handle.clone();
            std::thread::spawn(move || shortcuts::apply(&h));
            let h = handle.clone();
            std::thread::spawn(move || bridge::apply_enabled(&h));
            Ok(())
        })
        // A window that never reports a finished load, or whose WebKit
        // content process died, is the cause of a blank window; both are
        // logged, and a dead content process is reloaded.
        .on_page_load(|webview, payload| match payload.event() {
            tauri::webview::PageLoadEvent::Started => log::info!("{}: loading {}", webview.label(), payload.url()),
            tauri::webview::PageLoadEvent::Finished => {
                log::info!("{}: loaded {}", webview.label(), payload.url());
                let pending = webview.label() == "main"
                    && webview
                        .try_state::<AppState>()
                        .is_some_and(|s| s.show_pending.swap(false, Ordering::SeqCst));
                if pending {
                    show_main(webview.app_handle(), None);
                }
            }
        });
    #[cfg(target_os = "macos")]
    let builder = builder.on_web_content_process_terminate(|webview| {
        log::warn!("{}: web content process terminated; reloading", webview.label());
        if let Err(e) = webview.reload() {
            log::error!("{}: reload failed: {e}", webview.label());
        }
    });
    builder
        .on_window_event(|window, event| match (window.label(), event) {
            ("main", WindowEvent::CloseRequested { api, .. }) => {
                let app = window.app_handle();
                let keep = app.state::<AppState>().settings.lock().unwrap().keep_running;
                if keep {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    exit(app);
                }
            }
            ("player", WindowEvent::Moved(pos)) => {
                let app = window.app_handle();
                if window.is_visible().unwrap_or(false) {
                    if let Some(state) = app.try_state::<AppState>() {
                        state.settings.lock().unwrap().player_position = Some((pos.x as f64, pos.y as f64));
                    }
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::speak_text,
            commands::speak_clipboard,
            commands::read_clipboard_text,
            commands::control,
            commands::list_voices,
            commands::preview_voice,
            commands::get_document,
            commands::update_settings,
            commands::processor_status,
            commands::get_shortcuts,
            commands::set_shortcut,
            commands::suspend_shortcuts,
            commands::player_expand,
            commands::dismiss_player,
            commands::open_main,
            commands::exit_app,
            commands::report_ui_error,
            models::list_models,
            models::download_model,
            models::cancel_download,
            models::remove_model,
            commands::bridge_status,
            commands::bridge_test,
            commands::bridge_revoke,
            commands::bridge_allow_again,
            commands::open_extension_folder,
            studio::studio_list,
            studio::studio_create,
            studio::studio_get,
            studio::studio_rename,
            studio::studio_delete,
            studio::studio_save_script,
            studio::studio_select_version,
            studio::studio_delete_version,
            studio::studio_set_voice,
            studio::studio_features,
            studio::studio_generate,
            studio::studio_cancel,
            studio::studio_audio,
            studio::studio_export,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Sovirae")
        .run(|app, event| match event {
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { has_visible_windows, .. } => {
                if !has_visible_windows {
                    show_main(app, None);
                }
            }
            RunEvent::ExitRequested { code: None, api, .. } => {
                // Closing windows leaves the tray running; only Exit quits.
                let keep = app.state::<AppState>().settings.lock().unwrap().keep_running;
                if keep {
                    api.prevent_exit();
                }
            }
            _ => {}
        });
}
