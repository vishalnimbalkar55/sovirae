//! Chrome bridge (spec §13): native-host registration, the user-owned Unix
//! socket the host connects to, pairing, and the request/state protocol.
//!
//! Chrome → extension worker → `speakit-native-host` (stdio) → this socket.
//! The host only relays; every decision is made here.

use std::collections::{HashMap, HashSet};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, SystemTime};

use serde::Serialize;
use speakit_core::{PlaybackSnapshot, SourceKind, SourceReference, SpeakRequest};
use speakit_protocol::{
    codes, digest, extension_id, read_frame, write_frame, Action, Dedup, DedupResult, FrameError, Incoming, Outgoing,
    HOST_NAME, MAX_INCOMING_BYTES, MAX_TEXT_UTF16, PROTOCOL_VERSION,
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::controller::Command;
use crate::AppState;

/// ID of the Sovirae extension, fixed by the `key` in `ext/manifest.json`.
pub const EXTENSION_ID: &str = "jhbbmlhbjhjdepmaebpniefhaoljfgoe";

/// How long a speak request may wait for the session controller.
const SPEAK_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
pub struct Bridge {
    clients: Mutex<Vec<Arc<Client>>>,
    next_client: AtomicU64,
    dedup: Mutex<Dedup>,
    /// Extension IDs with a pairing prompt on screen.
    prompting: Mutex<HashSet<String>>,
    /// Extension IDs the user declined while the app has been running.
    declined: Mutex<HashSet<String>>,
    /// Session → the request that started it, for state messages.
    requests: Mutex<HashMap<u64, String>>,
    last_connection: Mutex<Option<SystemTime>>,
    last_extension: Mutex<Option<String>>,
    listener_stop: Mutex<Option<Arc<AtomicBool>>>,
}

struct Client {
    id: u64,
    writer: Mutex<UnixStream>,
    extension: Mutex<Option<String>>,
}

impl Client {
    fn send(&self, msg: &Outgoing) {
        let mut w = self.writer.lock().unwrap();
        if write_frame(&mut *w, &msg.to_json()).is_err() {
            let _ = w.shutdown(std::net::Shutdown::Both);
        }
    }

    fn extension(&self) -> Option<String> {
        self.extension.lock().unwrap().clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pairing {
    Paired,
    Pending,
    Declined,
    /// The connection did not come from a well-formed extension origin.
    Unknown,
}

impl Pairing {
    fn as_str(self) -> &'static str {
        match self {
            Pairing::Paired => "paired",
            Pairing::Pending => "pending",
            Pairing::Declined | Pairing::Unknown => "declined",
        }
    }
}

// ---- Registration ------------------------------------------------------------

/// Chromium-family browsers and where they look for per-user host manifests.
fn browser_roots() -> Vec<(&'static str, PathBuf)> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else { return Vec::new() };
    #[cfg(target_os = "macos")]
    let list = [
        ("Google Chrome", "Library/Application Support/Google/Chrome"),
        ("Chrome Beta", "Library/Application Support/Google/Chrome Beta"),
        ("Chrome Canary", "Library/Application Support/Google/Chrome Canary"),
        ("Chromium", "Library/Application Support/Chromium"),
        ("Microsoft Edge", "Library/Application Support/Microsoft Edge"),
        ("Brave", "Library/Application Support/BraveSoftware/Brave-Browser"),
        ("Vivaldi", "Library/Application Support/Vivaldi"),
        ("Arc", "Library/Application Support/Arc/User Data"),
    ];
    #[cfg(not(target_os = "macos"))]
    let list = [
        ("Google Chrome", ".config/google-chrome"),
        ("Chromium", ".config/chromium"),
        ("Microsoft Edge", ".config/microsoft-edge"),
        ("Brave", ".config/BraveSoftware/Brave-Browser"),
        ("Vivaldi", ".config/vivaldi"),
    ];
    list.iter().map(|(name, rel)| (*name, home.join(rel))).collect()
}

fn manifest_path(root: &Path) -> PathBuf {
    root.join("NativeMessagingHosts").join(format!("{HOST_NAME}.json"))
}

/// The host ships next to the app executable; in development it is built
/// into the release target directory.
pub fn host_bin() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let name = "speakit-native-host";
    [dir.join(name), dir.join("../release").join(name), dir.join("../debug").join(name)]
        .into_iter()
        .find(|p| p.is_file())
        .and_then(|p| p.canonicalize().ok())
}

/// The folder to load as an unpacked extension.
pub fn extension_folder(app: &AppHandle) -> Option<PathBuf> {
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ext");
    if cfg!(debug_assertions) && dev.join("manifest.json").is_file() {
        return dev.canonicalize().ok();
    }
    let bundled = app.path().resource_dir().ok()?.join("ext");
    bundled.join("manifest.json").is_file().then_some(bundled)
}

/// Writes host manifests for every installed Chromium browser and records
/// how the host can start the app. Safe to run on every launch.
pub fn register(app: &AppHandle) -> Result<(), String> {
    let host = host_bin().ok_or("The native host is missing from this build.")?;
    let paired = app.state::<AppState>().settings.lock().unwrap().paired_extensions.clone();
    let mut origins: Vec<String> = vec![format!("chrome-extension://{EXTENSION_ID}/")];
    for id in paired {
        let o = format!("chrome-extension://{id}/");
        if !origins.contains(&o) {
            origins.push(o);
        }
    }
    let manifest = serde_json::json!({
        "name": HOST_NAME,
        "description": "Sovirae: read web pages aloud with local voices",
        "path": host,
        "type": "stdio",
        "allowed_origins": origins,
    });
    let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    for (_, root) in browser_roots().into_iter().filter(|(_, r)| r.is_dir()) {
        let path = manifest_path(&root);
        if std::fs::read_to_string(&path).ok().as_deref() != Some(json.as_str()) {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(&path, &json) {
                log::warn!("could not write {}: {e}", path.display());
            }
        }
    }
    // Record how to start the app: the .app bundle when packaged.
    if let (Some(file), Ok(exe)) = (speakit_protocol::app_launch_file(), std::env::current_exe()) {
        let exe = exe.canonicalize().unwrap_or(exe);
        let s = exe.to_string_lossy();
        let target = match s.find(".app/Contents/MacOS/") {
            Some(i) => s[..i + 4].to_string(),
            None => s.into_owned(),
        };
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(file, target);
    }
    Ok(())
}

/// Removes host manifests so Chrome reports the bridge as off.
fn unregister() {
    for (_, root) in browser_roots() {
        let path = manifest_path(&root);
        if std::fs::read_to_string(&path).is_ok_and(|s| s.contains(HOST_NAME)) {
            let _ = std::fs::remove_file(path);
        }
    }
}

// ---- Listener ----------------------------------------------------------------

/// Starts or stops the bridge to match the setting.
pub fn apply_enabled(app: &AppHandle) {
    let enabled = app.state::<AppState>().settings.lock().unwrap().chrome_bridge;
    if enabled {
        if let Err(e) = register(app) {
            log::warn!("Chrome bridge registration: {e}");
        }
        start(app);
    } else {
        stop(app);
        unregister();
    }
    emit_status(app);
}

fn start(app: &AppHandle) {
    let bridge = &app.state::<AppState>().bridge;
    let mut guard = bridge.listener_stop.lock().unwrap();
    if guard.is_some() {
        return;
    }
    let Some(path) = speakit_protocol::socket_path() else { return };
    let listener = match bind(&path) {
        Ok(l) => l,
        Err(e) => {
            log::error!("Chrome bridge could not listen on {}: {e}", path.display());
            return;
        }
    };
    let stop = Arc::new(AtomicBool::new(false));
    *guard = Some(stop.clone());
    let app = app.clone();
    std::thread::Builder::new()
        .name("bridge-listener".into())
        .spawn(move || {
            for conn in listener.incoming() {
                if stop.load(Ordering::Acquire) {
                    break;
                }
                let Ok(stream) = conn else { continue };
                if !same_user(&stream) {
                    log::warn!("refused a bridge connection from another user");
                    continue;
                }
                let app = app.clone();
                std::thread::spawn(move || serve(app, stream));
            }
            let _ = std::fs::remove_file(&path);
        })
        .expect("spawn bridge listener");
}

fn stop(app: &AppHandle) {
    let bridge = &app.state::<AppState>().bridge;
    if let Some(flag) = bridge.listener_stop.lock().unwrap().take() {
        flag.store(true, Ordering::Release);
        // Wake the blocking accept so the thread sees the flag.
        if let Some(path) = speakit_protocol::socket_path() {
            let _ = UnixStream::connect(&path);
        }
    }
    for c in bridge.clients.lock().unwrap().drain(..) {
        let _ = c.writer.lock().unwrap().shutdown(std::net::Shutdown::Both);
    }
}

/// Binds the socket with owner-only permissions, replacing a stale file
/// left by a crash but never a live listener.
fn bind(path: &Path) -> std::io::Result<UnixListener> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            return Err(std::io::Error::new(std::io::ErrorKind::AddrInUse, "another Sovirae is listening"));
        }
        std::fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Accepts only processes running as this user.
fn same_user(stream: &UnixStream) -> bool {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: valid socket fd and out-pointers.
    let rc = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    rc == 0 && uid == unsafe { libc::getuid() }
}

fn serve(app: AppHandle, stream: UnixStream) {
    let bridge = &app.state::<AppState>().bridge;
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let Ok(writer) = stream.try_clone() else { return };
    let client = Arc::new(Client {
        id: bridge.next_client.fetch_add(1, Ordering::Relaxed),
        writer: Mutex::new(writer),
        extension: Mutex::new(None),
    });
    bridge.clients.lock().unwrap().push(client.clone());
    *bridge.last_connection.lock().unwrap() = Some(SystemTime::now());
    emit_status(&app);

    let mut reader = stream;
    loop {
        match read_frame(&mut reader, MAX_INCOMING_BYTES) {
            Ok(Some(json)) => handle(&app, &client, &json),
            Ok(None) => break,
            Err(FrameError::TooLarge(_)) => client.send(&Outgoing::error(codes::PAYLOAD_TOO_LARGE, "That message is too large.")),
            Err(FrameError::NotUtf8) => client.send(&Outgoing::error(codes::INVALID_REQUEST, "The message was not UTF-8.")),
            Err(FrameError::Io(_)) => break,
        }
    }
    bridge.clients.lock().unwrap().retain(|c| c.id != client.id);
    emit_status(&app);
}

// ---- Protocol ----------------------------------------------------------------

fn handle(app: &AppHandle, client: &Arc<Client>, json: &str) {
    let msg = match Incoming::parse(json) {
        Ok(m) => m,
        Err(e) => {
            // A malformed speak still gets a correlated rejection when its id is readable.
            let id = serde_json::from_str::<serde_json::Value>(json)
                .ok()
                .and_then(|v| v.get("id").and_then(|i| i.as_str()).map(str::to_string));
            match id {
                Some(id) if json.contains("\"speak\"") && id.len() <= 64 => client.send(&Outgoing::rejected(&id, e.code(), e.to_string())),
                _ => client.send(&Outgoing::error(e.code(), e.to_string())),
            }
            return;
        }
    };
    match msg {
        Incoming::Attach { origin } => {
            let ext = extension_id(&origin).map(str::to_string);
            if ext.is_none() {
                log::warn!("bridge attach without a valid extension origin");
            }
            *app.state::<AppState>().bridge.last_extension.lock().unwrap() = ext.clone();
            *client.extension.lock().unwrap() = ext;
            emit_status(app);
        }
        Incoming::Hello { .. } => {
            let pairing = pairing(app, client, true);
            client.send(&hello_ack(app, pairing));
        }
        Incoming::Ping { .. } => client.send(&Outgoing::Pong { v: PROTOCOL_VERSION }),
        Incoming::StateGet { .. } => {
            if pairing(app, client, false) == Pairing::Paired {
                let snap = app.state::<AppState>().snapshot.lock().unwrap().clone();
                client.send(&state_message(app, &snap));
            } else {
                client.send(&Outgoing::error(codes::NOT_AUTHORIZED, "Allow this extension in Sovirae first."));
            }
        }
        Incoming::Speak { id, text, source, language_hint, .. } => {
            let reply = speak(app, client, &id, text, source, language_hint);
            client.send(&reply);
        }
        Incoming::Control { session_id, action, delta_ms, rate, .. } => control(app, client, session_id, action, delta_ms, rate),
    }
}

fn hello_ack(app: &AppHandle, pairing: Pairing) -> Outgoing {
    let state = app.state::<AppState>();
    let voice = state.settings.lock().unwrap().voice.clone();
    Outgoing::HelloAck {
        v: PROTOCOL_VERSION,
        app: env!("CARGO_PKG_VERSION").into(),
        pairing: pairing.as_str().into(),
        voice: voice.map(|v| crate::commands::voice_display(&v)),
        ready: pairing == Pairing::Paired,
    }
}

/// The client's pairing state; with `ask`, an unknown extension triggers a
/// one-time prompt in the app (spec §13.2).
fn pairing(app: &AppHandle, client: &Arc<Client>, ask: bool) -> Pairing {
    let Some(ext) = client.extension() else { return Pairing::Unknown };
    let state = app.state::<AppState>();
    if state.settings.lock().unwrap().paired_extensions.contains(&ext) {
        return Pairing::Paired;
    }
    if state.bridge.declined.lock().unwrap().contains(&ext) {
        return Pairing::Declined;
    }
    if ask && state.bridge.prompting.lock().unwrap().insert(ext.clone()) {
        let app = app.clone();
        std::thread::spawn(move || prompt_pairing(app, ext));
    }
    Pairing::Pending
}

fn prompt_pairing(app: AppHandle, ext: String) {
    crate::show_main(&app, Some("extension"));
    let ours = ext == EXTENSION_ID;
    let message = if ours {
        "The Sovirae Chrome extension wants to send web page text to Sovirae to be read aloud.\n\nText is read on this Mac and is never sent anywhere else.".to_string()
    } else {
        format!(
            "A Chrome extension that is not the official Sovirae extension wants to send web page text to Sovirae.\n\nExtension ID: {ext}\n\nOnly allow it if you installed it yourself."
        )
    };
    let allowed = app
        .dialog()
        .message(message)
        .title("Allow Chrome to use Sovirae?")
        .kind(if ours { MessageDialogKind::Info } else { MessageDialogKind::Warning })
        .buttons(MessageDialogButtons::OkCancelCustom("Allow".into(), "Don't Allow".into()))
        .blocking_show();

    let state = app.state::<AppState>();
    state.bridge.prompting.lock().unwrap().remove(&ext);
    if allowed {
        {
            let mut s = state.settings.lock().unwrap();
            if !s.paired_extensions.contains(&ext) {
                s.paired_extensions.push(ext.clone());
            }
            state.store.save(&s);
        }
        let _ = register(&app);
    } else {
        state.bridge.declined.lock().unwrap().insert(ext.clone());
    }
    // Tell connected workers the outcome so they need not poll.
    let pairing = if allowed { Pairing::Paired } else { Pairing::Declined };
    for c in state.bridge.clients.lock().unwrap().iter().filter(|c| c.extension().as_deref() == Some(&ext)) {
        c.send(&hello_ack(&app, pairing));
    }
    emit_status(&app);
}

fn speak(
    app: &AppHandle,
    client: &Arc<Client>,
    id: &str,
    text: String,
    source: Option<speakit_protocol::Source>,
    language_hint: Option<String>,
) -> Outgoing {
    match pairing(app, client, true) {
        Pairing::Paired => {}
        Pairing::Pending => return Outgoing::rejected(id, codes::NOT_AUTHORIZED, "Allow this extension in Sovirae, then try again."),
        _ => return Outgoing::rejected(id, codes::PAIRING_DECLINED, "This extension is not allowed. Allow it in Sovirae › Extension."),
    }
    let state = app.state::<AppState>();
    let ext = client.extension().unwrap_or_default();
    let key = format!("{ext}/{id}");
    let origin = source.as_ref().and_then(|s| s.origin.clone()).unwrap_or_default();
    let payload = digest(&[&text, &origin, language_hint.as_deref().unwrap_or("")]);
    match state.bridge.dedup.lock().unwrap().check(&key, payload) {
        DedupResult::Replay(reply) => return reply,
        DedupResult::Conflict => {
            return Outgoing::rejected(id, codes::INVALID_REQUEST, "That request ID was already used for different text.")
        }
        DedupResult::New => {}
    }

    let reply = if text.trim().is_empty() {
        Outgoing::rejected(id, codes::NO_TEXT, "There is no text to read.")
    } else if text.chars().map(char::len_utf16).sum::<usize>() > MAX_TEXT_UTF16 {
        Outgoing::rejected(id, codes::TEXT_TOO_LONG, "That text is longer than 200,000 characters.")
    } else {
        let host = origin
            .split("://")
            .nth(1)
            .map(|h| h.trim_end_matches('/').to_string())
            .filter(|h| !h.is_empty());
        let display = host.or_else(|| source.as_ref().and_then(|s| s.title.clone())).unwrap_or_else(|| "Chrome".into());
        let (tx, rx) = mpsc::channel();
        state.controller.send(Command::Speak {
            request: SpeakRequest {
                text,
                source: SourceReference { kind: SourceKind::Chrome, display_name: Some(display) },
                language_hint,
            },
            truncate: false,
            reply: Some(tx),
        });
        match rx.recv_timeout(SPEAK_TIMEOUT) {
            Ok(Ok(session_id)) => {
                state.bridge.requests.lock().unwrap().insert(session_id, id.to_string());
                Outgoing::SpeakAccepted {
                    v: PROTOCOL_VERSION,
                    id: id.into(),
                    session_id,
                    status: "preparing".into(),
                    estimated_duration_ms: None,
                    duration_is_final: false,
                }
            }
            Ok(Err(notice)) => {
                let code = match notice.code {
                    "NO_TEXT" => codes::NO_TEXT,
                    "TEXT_TOO_LONG" => codes::TEXT_TOO_LONG,
                    "NO_VOICE" => codes::NO_VOICE,
                    _ => codes::INTERNAL,
                };
                Outgoing::rejected(id, code, notice.message)
            }
            Err(_) => Outgoing::rejected(id, codes::INTERNAL, "Sovirae is busy. Try again."),
        }
    };
    state.bridge.dedup.lock().unwrap().record(&key, payload, reply.clone());
    reply
}

fn control(app: &AppHandle, client: &Arc<Client>, session_id: Option<u64>, action: Action, delta_ms: Option<f64>, rate: Option<f64>) {
    if pairing(app, client, false) != Pairing::Paired {
        client.send(&Outgoing::error(codes::NOT_AUTHORIZED, "Allow this extension in Sovirae first."));
        return;
    }
    let state = app.state::<AppState>();
    let current = state.snapshot.lock().unwrap().session_id;
    if session_id.is_some_and(|s| s != current) {
        client.send(&Outgoing::error(codes::STALE_SESSION, "That reading has already ended."));
        return;
    }
    let c = &state.controller;
    match action {
        Action::Play => c.send(Command::Play),
        Action::Pause => c.send(Command::Pause),
        Action::Toggle => c.send(Command::TogglePause),
        Action::Stop => c.send(Command::Stop),
        Action::Skip => {
            let snap = state.settings.lock().unwrap().sentence_snap;
            c.send(Command::Skip { delta_ms: delta_ms.unwrap_or(10_000.0) as i64, snap });
        }
        Action::Rate => {
            if let Some(r) = rate {
                let _ = crate::commands::update_settings(app.clone(), app.state::<AppState>(), serde_json::json!({ "rate": r }));
            }
        }
    }
}

fn state_message(app: &AppHandle, s: &PlaybackSnapshot) -> Outgoing {
    let request_id = app.state::<AppState>().bridge.requests.lock().unwrap().get(&s.session_id).cloned();
    Outgoing::State {
        v: PROTOCOL_VERSION,
        session_id: s.session_id,
        request_id,
        status: serde_json::to_value(s.status).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default(),
        position_ms: s.position_ms,
        duration_ms: s.duration_ms,
        duration_is_final: s.duration_is_final,
        rate: s.rate,
        sentence_id: s.sentence_id,
        voice: s.voice.clone(),
    }
}

/// Pushes playback state to paired extensions; the controller already
/// limits steady updates to 4 Hz (spec §13.3).
pub fn on_snapshot(app: &AppHandle, snap: &PlaybackSnapshot) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let clients: Vec<Arc<Client>> = state.bridge.clients.lock().unwrap().clone();
    if clients.is_empty() {
        return;
    }
    let paired = state.settings.lock().unwrap().paired_extensions.clone();
    let msg = state_message(app, snap);
    for c in clients.iter().filter(|c| c.extension().is_some_and(|e| paired.contains(&e))) {
        c.send(&msg);
    }
    // Forget request IDs of sessions that ended long ago.
    let mut reqs = state.bridge.requests.lock().unwrap();
    if reqs.len() > 64 {
        let keep = snap.session_id;
        reqs.retain(|s, _| *s + 32 > keep);
    }
}

// ---- Status and diagnostics --------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserStatus {
    name: String,
    registered: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatus {
    enabled: bool,
    listening: bool,
    extension_id: String,
    extension_folder: Option<String>,
    host_installed: bool,
    browsers: Vec<BrowserStatus>,
    paired: Vec<String>,
    connected: usize,
    /// Seconds since a browser last connected.
    last_connection_secs: Option<u64>,
    last_extension: Option<String>,
}

pub fn status(app: &AppHandle) -> BridgeStatus {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    let bridge = &state.bridge;
    let browsers = browser_roots()
        .into_iter()
        .filter(|(_, root)| root.is_dir())
        .map(|(name, root)| BrowserStatus {
            name: name.into(),
            registered: std::fs::read_to_string(manifest_path(&root)).is_ok_and(|s| s.contains(EXTENSION_ID)),
        })
        .collect();
    let listening = bridge.listener_stop.lock().unwrap().is_some();
    let connected = bridge.clients.lock().unwrap().iter().filter(|c| c.extension().is_some()).count();
    let last_connection_secs =
        bridge.last_connection.lock().unwrap().and_then(|t| t.elapsed().ok()).map(|d| d.as_secs());
    let last_extension = bridge.last_extension.lock().unwrap().clone();
    BridgeStatus {
        enabled: settings.chrome_bridge,
        listening,
        extension_id: EXTENSION_ID.into(),
        extension_folder: extension_folder(app).map(|p| p.to_string_lossy().into_owned()),
        host_installed: host_bin().is_some(),
        browsers,
        paired: settings.paired_extensions.clone(),
        connected,
        last_connection_secs,
        last_extension,
    }
}

pub fn emit_status(app: &AppHandle) {
    let _ = app.emit("bridge", status(app));
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    ok: bool,
    message: String,
}

/// Runs the path Chrome uses: the registered manifest's host, started with
/// the extension origin, completing a hello round trip.
pub fn test_connection(app: &AppHandle) -> TestResult {
    let fail = |message: String| TestResult { ok: false, message };
    if !app.state::<AppState>().settings.lock().unwrap().chrome_bridge {
        return fail("The Chrome connection is turned off.".into());
    }
    let Some((browser, root)) = browser_roots().into_iter().find(|(_, r)| r.is_dir()) else {
        return fail("No Chromium-based browser was found for this user.".into());
    };
    let manifest = match std::fs::read_to_string(manifest_path(&root)) {
        Ok(m) => m,
        Err(_) => return fail(format!("{browser} has no Sovirae connection registered. Turn the connection off and on again.")),
    };
    let parsed: serde_json::Value = serde_json::from_str(&manifest).unwrap_or_default();
    let Some(host) = parsed.get("path").and_then(|p| p.as_str()).map(PathBuf::from) else {
        return fail("The registered connection file is damaged.".into());
    };
    if !host.is_file() {
        return fail(format!("The registered host program is missing: {}", host.display()));
    }
    let mut child = match std::process::Command::new(&host)
        .arg(format!("chrome-extension://{EXTENSION_ID}/"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return fail(format!("The host program could not start: {e}")),
    };
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let _ = write_frame(&mut stdin, r#"{"t":"hello","v":1,"ext":"self-test"}"#);
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(read_frame(&mut stdout, 64 << 10));
    });
    let result = rx.recv_timeout(Duration::from_secs(7));
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    match result {
        Ok(Ok(Some(json))) => {
            let v: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
            match v.get("t").and_then(|t| t.as_str()) {
                Some("hello.ack") => {
                    let pairing = v.get("pairing").and_then(|p| p.as_str()).unwrap_or("");
                    let note = match pairing {
                        "paired" => "The extension is allowed.",
                        "pending" => "Answer the prompt in Sovirae to allow the extension.",
                        _ => "The extension is not allowed yet.",
                    };
                    TestResult { ok: true, message: format!("{browser} can reach Sovirae. {note}") }
                }
                _ => fail(format!(
                    "The host answered with an error: {}",
                    v.get("message").and_then(|m| m.as_str()).unwrap_or("unknown")
                )),
            }
        }
        Ok(_) => fail("The host closed without answering.".into()),
        Err(_) => fail("The host did not answer within 7 seconds.".into()),
    }
}

pub fn revoke(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    {
        let mut s = state.settings.lock().unwrap();
        s.paired_extensions.retain(|x| x != id);
        state.store.save(&s);
    }
    state.bridge.declined.lock().unwrap().insert(id.to_string());
    for c in state.bridge.clients.lock().unwrap().iter().filter(|c| c.extension().as_deref() == Some(id)) {
        let _ = c.writer.lock().unwrap().shutdown(std::net::Shutdown::Both);
    }
    let _ = register(app);
    emit_status(app);
}

/// Lets a declined extension ask again (e.g. after the user changes mind).
pub fn forget_decline(app: &AppHandle, id: &str) {
    app.state::<AppState>().bridge.declined.lock().unwrap().remove(id);
    emit_status(app);
}
