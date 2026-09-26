//! Versioned protocol shared by the Chrome extension, the native messaging
//! host, and the desktop app (spec §13).
//!
//! Every hop uses Chrome's framing: a 32-bit native-endian length, then that
//! many bytes of UTF-8 JSON. The native host relays frames unchanged between
//! Chrome's stdio and the app's user-owned Unix socket, so all validation and
//! decisions happen in the app.

use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROTOCOL_VERSION: u32 = 1;
/// Native messaging host name registered with Chrome.
pub const HOST_NAME: &str = "com.sovirae.bridge";
/// Largest frame accepted from the extension (spec §13.1).
pub const MAX_INCOMING_BYTES: usize = 2 << 20;
/// Largest frame the app sends toward Chrome; Chrome's own limit is 1 MB.
pub const MAX_OUTGOING_BYTES: usize = 64 << 10;
/// Text limit in UTF-16 code units, the same as every other entry path.
pub const MAX_TEXT_UTF16: usize = 200_000;
/// Chrome's documented browser-to-host ceiling; larger frames are impossible.
const CHROME_MAX_FRAME: usize = 64 << 20;

pub mod codes {
    pub const NOT_AUTHORIZED: &str = "NOT_AUTHORIZED";
    pub const PAIRING_DECLINED: &str = "PAIRING_DECLINED";
    pub const NO_TEXT: &str = "NO_TEXT";
    pub const TEXT_TOO_LONG: &str = "TEXT_TOO_LONG";
    pub const PAYLOAD_TOO_LARGE: &str = "PAYLOAD_TOO_LARGE";
    pub const NO_VOICE: &str = "NO_VOICE";
    pub const APP_UNAVAILABLE: &str = "APP_UNAVAILABLE";
    pub const UNSUPPORTED_PROTOCOL: &str = "UNSUPPORTED_PROTOCOL";
    pub const STALE_SESSION: &str = "STALE_SESSION";
    pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
    pub const INTERNAL: &str = "INTERNAL";
}

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("frame of {0} bytes exceeds the limit")]
    TooLarge(usize),
    #[error("frame is not valid UTF-8")]
    NotUtf8,
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Reads one frame. `Ok(None)` means a clean end of stream. An oversized
/// frame is consumed and discarded so the stream stays in sync.
pub fn read_frame(r: &mut impl Read, max: usize) -> Result<Option<String>, FrameError> {
    let mut header = [0u8; 4];
    match r.read_exact(&mut header) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let len = u32::from_ne_bytes(header) as usize;
    if len > max {
        if len > CHROME_MAX_FRAME {
            // Not a Chrome frame at all; the stream cannot be trusted.
            return Err(io::Error::new(io::ErrorKind::InvalidData, "corrupt frame header").into());
        }
        io::copy(&mut r.take(len as u64), &mut io::sink())?;
        return Err(FrameError::TooLarge(len));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map(Some).map_err(|_| FrameError::NotUtf8)
}

pub fn write_frame(w: &mut impl Write, json: &str) -> io::Result<()> {
    let len = u32::try_from(json.len()).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame too large"))?;
    let mut frame = Vec::with_capacity(4 + json.len());
    frame.extend_from_slice(&len.to_ne_bytes());
    frame.extend_from_slice(json.as_bytes());
    w.write_all(&frame)?;
    w.flush()
}

/// Turns `chrome-extension://<id>/` into `<id>`, or `None` for anything that
/// is not a well-formed extension origin.
pub fn extension_id(origin: &str) -> Option<&str> {
    let id = origin.strip_prefix("chrome-extension://")?.trim_end_matches('/');
    (id.len() == 32 && id.bytes().all(|b| (b'a'..=b'p').contains(&b))).then_some(id)
}

/// Where the app listens: a socket inside the user's private app-data folder.
pub fn socket_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let base = PathBuf::from(home);
    #[cfg(target_os = "macos")]
    let dir = base.join("Library/Application Support/com.sovirae.desktop");
    #[cfg(not(target_os = "macos"))]
    let dir = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join(".local/share"))
        .join("com.sovirae.desktop");
    Some(dir.join("bridge.sock"))
}

/// File beside the socket recording how to start the app.
pub fn app_launch_file() -> Option<PathBuf> {
    socket_path().map(|p| p.with_file_name("bridge-app-path"))
}

// ---- Messages ---------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// Page origin only; full URLs can carry private tokens (spec §14).
    pub origin: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Play,
    Pause,
    Toggle,
    Stop,
    Skip,
    Rate,
}

/// Messages from the extension, plus the host's `attach`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "t")]
pub enum Incoming {
    /// Sent by the native host before relaying anything: the extension
    /// origin Chrome put on the host's command line.
    #[serde(rename = "attach")]
    Attach { origin: String },
    #[serde(rename = "hello")]
    Hello { v: u32, #[serde(default)] ext: Option<String> },
    #[serde(rename = "speak", rename_all = "camelCase")]
    Speak {
        v: u32,
        id: String,
        text: String,
        #[serde(default)]
        source: Option<Source>,
        #[serde(default)]
        language_hint: Option<String>,
    },
    #[serde(rename = "control", rename_all = "camelCase")]
    Control {
        v: u32,
        #[serde(default)]
        session_id: Option<u64>,
        action: Action,
        #[serde(default)]
        delta_ms: Option<f64>,
        #[serde(default)]
        rate: Option<f64>,
    },
    #[serde(rename = "state.get")]
    StateGet { v: u32 },
    #[serde(rename = "ping")]
    Ping { v: u32 },
}

#[derive(Debug, Error, PartialEq)]
pub enum InvalidMessage {
    #[error("unsupported protocol version {0}")]
    Version(u32),
    #[error("{0}")]
    Invalid(String),
}

impl InvalidMessage {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Version(_) => codes::UNSUPPORTED_PROTOCOL,
            Self::Invalid(_) => codes::INVALID_REQUEST,
        }
    }
}

impl Incoming {
    /// Parses and validates types, ranges, and enum members (spec §13.3).
    pub fn parse(json: &str) -> Result<Self, InvalidMessage> {
        let msg: Incoming =
            serde_json::from_str(json).map_err(|e| InvalidMessage::Invalid(format!("malformed message: {e}")))?;
        msg.validate()?;
        Ok(msg)
    }

    fn validate(&self) -> Result<(), InvalidMessage> {
        let version = match self {
            Self::Attach { .. } => return Ok(()),
            Self::Hello { v, .. } | Self::Speak { v, .. } | Self::Control { v, .. } | Self::StateGet { v } | Self::Ping { v } => *v,
        };
        if version != PROTOCOL_VERSION {
            return Err(InvalidMessage::Version(version));
        }
        match self {
            Self::Speak { id, source, language_hint, .. } => {
                valid_id(id)?;
                if let Some(s) = source {
                    if s.origin.as_ref().is_some_and(|o| o.len() > 512) || s.title.as_ref().is_some_and(|t| t.len() > 1024) {
                        return Err(InvalidMessage::Invalid("source metadata is too long".into()));
                    }
                }
                if language_hint.as_ref().is_some_and(|l| l.len() > 35 || !l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')) {
                    return Err(InvalidMessage::Invalid("invalid language hint".into()));
                }
            }
            Self::Control { action, delta_ms, rate, .. } => {
                if let Some(d) = delta_ms {
                    if !d.is_finite() || d.abs() > 600_000.0 {
                        return Err(InvalidMessage::Invalid("deltaMs out of range".into()));
                    }
                }
                if let Some(r) = rate {
                    if !r.is_finite() || !(0.5..=3.0).contains(r) {
                        return Err(InvalidMessage::Invalid("rate out of range".into()));
                    }
                }
                if *action == Action::Rate && rate.is_none() {
                    return Err(InvalidMessage::Invalid("rate action needs a rate".into()));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn valid_id(id: &str) -> Result<(), InvalidMessage> {
    let ok = !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(InvalidMessage::Invalid("request id must be 1–64 letters, digits, - or _".into()))
    }
}

/// Messages to the extension.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "t")]
pub enum Outgoing {
    #[serde(rename = "hello.ack", rename_all = "camelCase")]
    HelloAck {
        v: u32,
        app: String,
        /// `paired`, `pending` (the user has not answered yet), or `declined`.
        pairing: String,
        voice: Option<String>,
        ready: bool,
    },
    #[serde(rename = "speak.accepted", rename_all = "camelCase")]
    SpeakAccepted {
        v: u32,
        id: String,
        session_id: u64,
        status: String,
        /// Unknown until synthesis reports real timing (spec §13.3).
        estimated_duration_ms: Option<u64>,
        duration_is_final: bool,
    },
    #[serde(rename = "speak.rejected")]
    SpeakRejected { v: u32, id: String, code: String, message: String },
    #[serde(rename = "state", rename_all = "camelCase")]
    State {
        v: u32,
        session_id: u64,
        request_id: Option<String>,
        status: String,
        position_ms: u64,
        duration_ms: u64,
        duration_is_final: bool,
        rate: f32,
        sentence_id: Option<usize>,
        voice: Option<String>,
    },
    #[serde(rename = "pong")]
    Pong { v: u32 },
    #[serde(rename = "error")]
    Error { v: u32, code: String, message: String },
}

impl Outgoing {
    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self::Error { v: PROTOCOL_VERSION, code: code.into(), message: message.into() }
    }

    pub fn rejected(id: &str, code: &str, message: impl Into<String>) -> Self {
        Self::SpeakRejected { v: PROTOCOL_VERSION, id: id.into(), code: code.into(), message: message.into() }
    }

    /// Serializes within the outgoing size limit.
    pub fn to_json(&self) -> String {
        let json = serde_json::to_string(self).expect("outgoing messages serialize");
        if json.len() <= MAX_OUTGOING_BYTES {
            json
        } else {
            serde_json::to_string(&Self::error(codes::INTERNAL, "reply too large")).expect("serialize")
        }
    }
}

// ---- Request de-duplication --------------------------------------------------

/// Remembers recent request outcomes so a request resent after a reconnect
/// returns the original result instead of restarting audio (spec §13.3).
pub struct Dedup {
    entries: VecDeque<(Instant, String, u64, Outgoing)>,
    capacity: usize,
    ttl: Duration,
}

pub enum DedupResult {
    New,
    Replay(Outgoing),
    Conflict,
}

impl Default for Dedup {
    fn default() -> Self {
        Self::new(100, Duration::from_secs(300))
    }
}

impl Dedup {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self { entries: VecDeque::new(), capacity, ttl }
    }

    fn expire(&mut self) {
        while self.entries.front().is_some_and(|(t, ..)| t.elapsed() > self.ttl) {
            self.entries.pop_front();
        }
    }

    /// `key` identifies the request (extension + id); `payload` is a digest
    /// of its content.
    pub fn check(&mut self, key: &str, payload: u64) -> DedupResult {
        self.expire();
        match self.entries.iter().find(|(_, k, ..)| k == key) {
            Some((_, _, p, reply)) if *p == payload => DedupResult::Replay(reply.clone()),
            Some(_) => DedupResult::Conflict,
            None => DedupResult::New,
        }
    }

    pub fn record(&mut self, key: &str, payload: u64, reply: Outgoing) {
        self.expire();
        self.entries.retain(|(_, k, ..)| k != key);
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back((Instant::now(), key.to_string(), payload, reply));
    }
}

/// Stable 64-bit digest (FNV-1a) for de-duplication.
pub fn digest(parts: &[&str]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in parts {
        for b in p.bytes().chain(std::iter::once(0xff)) {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn frame(json: &str) -> Vec<u8> {
        let mut v = Vec::new();
        write_frame(&mut v, json).unwrap();
        v
    }

    #[test]
    fn frames_round_trip_and_end_cleanly() {
        let mut data = frame(r#"{"t":"ping","v":1}"#);
        data.extend(frame("{}"));
        let mut c = Cursor::new(data);
        assert_eq!(read_frame(&mut c, 1024).unwrap().as_deref(), Some(r#"{"t":"ping","v":1}"#));
        assert_eq!(read_frame(&mut c, 1024).unwrap().as_deref(), Some("{}"));
        assert_eq!(read_frame(&mut c, 1024).unwrap(), None);
    }

    #[test]
    fn oversized_frames_are_skipped_without_losing_sync() {
        let big = format!(r#"{{"x":"{}"}}"#, "a".repeat(5000));
        let mut data = frame(&big);
        data.extend(frame(r#"{"ok":1}"#));
        let mut c = Cursor::new(data);
        assert!(matches!(read_frame(&mut c, 1000), Err(FrameError::TooLarge(_))));
        assert_eq!(read_frame(&mut c, 1000).unwrap().as_deref(), Some(r#"{"ok":1}"#));
    }

    #[test]
    fn partial_frames_are_errors_not_messages() {
        let mut data = frame(r#"{"t":"ping","v":1}"#);
        data.truncate(data.len() - 3);
        assert!(read_frame(&mut Cursor::new(data), 1024).is_err());
    }

    #[test]
    fn parses_extension_ids() {
        assert_eq!(extension_id("chrome-extension://abcdefghijklmnopabcdefghijklmnop/"), Some("abcdefghijklmnopabcdefghijklmnop"));
        assert_eq!(extension_id("chrome-extension://abcdefghijklmnopabcdefghijklmnoq/"), None);
        assert_eq!(extension_id("https://example.com/"), None);
        assert_eq!(extension_id("chrome-extension://short/"), None);
    }

    #[test]
    fn validates_messages() {
        let ok = r#"{"t":"speak","v":1,"id":"req-1","text":"Hi","source":{"origin":"https://a.b","title":"T"},"languageHint":"fr-FR"}"#;
        assert!(matches!(Incoming::parse(ok), Ok(Incoming::Speak { .. })));
        assert_eq!(Incoming::parse(r#"{"t":"ping","v":2}"#), Err(InvalidMessage::Version(2)));
        for bad in [
            r#"{"t":"speak","v":1,"id":"a b","text":"x"}"#,
            r#"{"t":"speak","v":1,"id":"","text":"x"}"#,
            r#"{"t":"control","v":1,"action":"rate","rate":9}"#,
            r#"{"t":"control","v":1,"action":"rate"}"#,
            r#"{"t":"control","v":1,"action":"explode"}"#,
            r#"{"t":"control","v":1,"action":"skip","deltaMs":1e12}"#,
            r#"{"t":"nope","v":1}"#,
            r#"{"v":1}"#,
            "not json",
        ] {
            assert!(matches!(Incoming::parse(bad), Err(InvalidMessage::Invalid(_))), "{bad}");
        }
    }

    #[test]
    fn serializes_replies_with_expected_tags() {
        let json = Outgoing::rejected("r1", codes::NO_TEXT, "Nothing to read").to_json();
        assert_eq!(json, r#"{"t":"speak.rejected","v":1,"id":"r1","code":"NO_TEXT","message":"Nothing to read"}"#);
        let acc = Outgoing::SpeakAccepted {
            v: 1, id: "r".into(), session_id: 4, status: "preparing".into(), estimated_duration_ms: None, duration_is_final: false,
        };
        assert!(acc.to_json().contains(r#""estimatedDurationMs":null"#));
    }

    #[test]
    fn deduplicates_by_request_id_and_payload() {
        let mut d = Dedup::new(2, Duration::from_secs(60));
        let reply = Outgoing::Pong { v: 1 };
        assert!(matches!(d.check("ext/req-1", 7), DedupResult::New));
        d.record("ext/req-1", 7, reply.clone());
        assert!(matches!(d.check("ext/req-1", 7), DedupResult::Replay(r) if r == reply));
        assert!(matches!(d.check("ext/req-1", 8), DedupResult::Conflict));
        d.record("ext/req-2", 1, reply.clone());
        d.record("ext/req-3", 1, reply.clone());
        assert!(matches!(d.check("ext/req-1", 7), DedupResult::New), "capacity evicts the oldest");
        let mut short = Dedup::new(10, Duration::from_millis(0));
        short.record("k", 1, reply);
        std::thread::sleep(Duration::from_millis(2));
        assert!(matches!(short.check("k", 1), DedupResult::New), "entries expire");
    }

    #[test]
    fn digest_is_stable_and_separates_parts() {
        assert_eq!(digest(&["a", "b"]), digest(&["a", "b"]));
        assert_ne!(digest(&["ab", ""]), digest(&["a", "b"]));
    }
}
