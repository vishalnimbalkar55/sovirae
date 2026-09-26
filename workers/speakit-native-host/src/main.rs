//! Chrome native messaging host for Sovirae (spec §13).
//!
//! Chrome starts this process when the extension connects and passes the
//! extension's origin as the first argument. It is a relay with no protocol
//! logic: it connects to the app's user-owned Unix socket (starting the app
//! if needed), announces the origin, then copies frames both ways. stdout
//! carries protocol bytes only; diagnostics go to stderr.

use std::io::{self, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use speakit_protocol::{codes, read_frame, write_frame, FrameError, Outgoing, MAX_INCOMING_BYTES, MAX_OUTGOING_BYTES};

/// Spec §13.2: wait at most five seconds for the app to come up.
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(5);

fn main() {
    let origin = std::env::args().nth(1).unwrap_or_default();
    let socket = std::env::var_os("SOVIRAE_BRIDGE_SOCKET")
        .map(Into::into)
        .or_else(speakit_protocol::socket_path);
    let Some(socket) = socket else {
        fail(codes::APP_UNAVAILABLE, "The home folder is unknown, so Sovirae cannot be found.");
    };

    let stream = match connect(&socket) {
        Ok(s) => s,
        Err(message) => fail(codes::APP_UNAVAILABLE, &message),
    };

    let attach = format!(r#"{{"t":"attach","origin":{}}}"#, json_string(&origin));
    let mut to_app = stream.try_clone().expect("clone socket");
    if write_frame(&mut to_app, &attach).is_err() {
        fail(codes::APP_UNAVAILABLE, "Sovirae closed the connection.");
    }

    // App → Chrome.
    let mut from_app = stream;
    std::thread::spawn(move || {
        let stdout = io::stdout();
        loop {
            match read_frame(&mut from_app, MAX_OUTGOING_BYTES) {
                Ok(Some(json)) => {
                    if write_frame(&mut stdout.lock(), &json).is_err() {
                        break;
                    }
                }
                Ok(None) | Err(FrameError::Io(_)) | Err(FrameError::NotUtf8) => break,
                Err(FrameError::TooLarge(n)) => eprintln!("dropped a {n}-byte frame from the app"),
            }
        }
        // The app went away; ending the process closes Chrome's port.
        std::process::exit(0);
    });

    // Chrome → app.
    let mut stdin = io::stdin().lock();
    loop {
        match read_frame(&mut stdin, MAX_INCOMING_BYTES) {
            Ok(Some(json)) => {
                if write_frame(&mut to_app, &json).is_err() {
                    break;
                }
            }
            Err(FrameError::TooLarge(n)) => {
                reply(Outgoing::error(
                    codes::PAYLOAD_TOO_LARGE,
                    format!("That message is {} MB; the limit is 2 MB.", n / (1 << 20).max(1)),
                ));
            }
            Err(FrameError::NotUtf8) => reply(Outgoing::error(codes::INVALID_REQUEST, "The message was not UTF-8.")),
            Ok(None) | Err(FrameError::Io(_)) => break,
        }
    }
}

/// Connects to the app, starting it in the background if the socket is
/// missing or refused, and retrying until the launch timeout.
fn connect(socket: &Path) -> Result<UnixStream, String> {
    if let Some(s) = try_connect(socket)? {
        return Ok(s);
    }
    launch_app()?;
    let start = Instant::now();
    while start.elapsed() < LAUNCH_TIMEOUT {
        std::thread::sleep(Duration::from_millis(150));
        if let Some(s) = try_connect(socket)? {
            return Ok(s);
        }
    }
    Err("Sovirae did not start in time. Open it and try again.".into())
}

/// `Ok(None)` when nothing is listening yet; `Err` when the socket is not
/// safe to use.
fn try_connect(socket: &Path) -> Result<Option<UnixStream>, String> {
    let Ok(meta) = std::fs::symlink_metadata(socket) else { return Ok(None) };
    // Only talk to a socket this user owns: another account must not be able
    // to impersonate the app and receive page text.
    let uid = unsafe { libc::getuid() };
    if !meta.file_type().is_socket() || meta.uid() != uid {
        return Err("The Sovirae bridge socket is not owned by this user, so it was not used.".into());
    }
    match UnixStream::connect(socket) {
        Ok(s) => Ok(Some(s)),
        Err(_) => Ok(None),
    }
}

/// Starts the app from the location it recorded, never from page input.
fn launch_app() -> Result<(), String> {
    let file = speakit_protocol::app_launch_file().ok_or("Sovirae's location is unknown.")?;
    let path = std::fs::read_to_string(&file)
        .map_err(|_| "Sovirae is not running. Open it once so the extension can start it later.".to_string())?;
    let path = path.trim();
    if path.is_empty() || !Path::new(path).exists() {
        return Err("Sovirae was moved or removed. Open it once to repair the extension connection.".into());
    }
    let mut cmd = if path.ends_with(".app") {
        let mut c = Command::new("/usr/bin/open");
        c.args(["-g", "-a", path, "--args", "--background"]);
        c
    } else {
        let mut c = Command::new(path);
        c.arg("--background");
        c
    };
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    // Detach so the app keeps running when Chrome ends this host process.
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    cmd.spawn().map(|_| ()).map_err(|e| format!("Sovirae could not be started: {e}"))
}

fn reply(msg: Outgoing) {
    let _ = write_frame(&mut io::stdout().lock(), &msg.to_json());
}

fn fail(code: &str, message: &str) -> ! {
    reply(Outgoing::error(code, message));
    let _ = io::stdout().flush();
    eprintln!("sovirae native host: {message}");
    std::process::exit(1);
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
