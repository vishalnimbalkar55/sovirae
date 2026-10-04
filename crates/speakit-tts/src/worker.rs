//! Client side of the inference worker protocol shared by the neural
//! engines (spec §5.1, §8.4): one JSON request per line on the worker's
//! stdin, binary frames back on its stdout.
//!
//! Frame: u64 request id | u32 status | u32 length | payload (little-endian)
//!   status 0: `length` f32 samples
//!   status 1: `length` bytes of UTF-8 error text
//!   status 2: ready, sent once after loading; payload names the device

use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use crate::{CancelToken, TtsError};

pub(crate) struct Frame {
    pub id: u64,
    pub status: u32,
    pub payload: Vec<u8>,
}

/// A running worker. Dropping it kills the process.
pub(crate) struct WorkerProc {
    /// Device the worker reported when it became ready.
    pub device: &'static str,
    child: Child,
    stdin: ChildStdin,
    frames: Receiver<Frame>,
}

impl Drop for WorkerProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl WorkerProc {
    /// Starts `cmd` and waits until the worker reports it is ready. With
    /// `below_normal` the process runs at reduced CPU priority (spec §8.2),
    /// so foreground apps stay responsive while a sentence is generated.
    pub fn spawn(mut cmd: Command, name: &str, cancel: &CancelToken, timeout: Duration, below_normal: bool) -> Result<Self, TtsError> {
        let mut child = crate::no_console(&mut cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| TtsError::Engine(format!("the voice worker could not start: {e}")))?;
        if below_normal {
            lower_priority(&child);
        }
        let stdin = child.stdin.take().expect("piped stdin");
        let mut stdout = child.stdout.take().expect("piped stdout");
        let (tx, frames) = mpsc::channel();
        std::thread::Builder::new()
            .name(format!("{name}-reader"))
            .spawn(move || loop {
                let mut header = [0u8; 16];
                if stdout.read_exact(&mut header).is_err() {
                    break;
                }
                let id = u64::from_le_bytes(header[0..8].try_into().unwrap());
                let status = u32::from_le_bytes(header[8..12].try_into().unwrap());
                let len = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
                let bytes = if status == 0 { len * 4 } else { len };
                let mut payload = vec![0u8; bytes];
                if stdout.read_exact(&mut payload).is_err() {
                    break;
                }
                if tx.send(Frame { id, status, payload }).is_err() {
                    break;
                }
            })
            .map_err(|e| TtsError::Engine(e.to_string()))?;

        let mut proc = WorkerProc { device: "CPU", child, stdin, frames };
        let ready = proc.wait(0, cancel, timeout)?;
        if ready.status != 2 {
            return Err(TtsError::Engine(String::from_utf8_lossy(&ready.payload).into_owned()));
        }
        // Label the session with what the worker loaded, not what was asked.
        proc.device = if ready.payload == b"GPU" { "GPU" } else { "CPU" };
        Ok(proc)
    }

    /// Sends one request line.
    pub fn send(&mut self, request: &serde_json::Value) -> Result<(), TtsError> {
        writeln!(self.stdin, "{request}")
            .and_then(|_| self.stdin.flush())
            .map_err(|_| TtsError::Engine("the voice worker stopped unexpectedly".into()))
    }

    /// Waits for the reply to request `id`, skipping stale replies to
    /// cancelled requests.
    pub fn wait(&mut self, id: u64, cancel: &CancelToken, timeout: Duration) -> Result<Frame, TtsError> {
        let start = Instant::now();
        loop {
            if cancel.is_cancelled() {
                return Err(TtsError::Cancelled);
            }
            match self.frames.recv_timeout(Duration::from_millis(15)) {
                Ok(f) if f.id == id || f.status == 2 || (f.id == 0 && f.status == 1) => return Ok(f),
                Ok(_) => continue,
                Err(RecvTimeoutError::Timeout) => {
                    if start.elapsed() > timeout {
                        return Err(TtsError::Engine("the voice worker stopped responding".into()));
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(TtsError::Engine("the voice worker stopped unexpectedly".into()));
                }
            }
        }
    }
}

/// Lowers a child's scheduling priority. Best effort: a failure leaves it
/// at normal priority.
#[cfg(unix)]
fn lower_priority(child: &Child) {
    // nice 10: clearly below interactive work, still well above idle.
    // SAFETY: plain syscall on a process ID this process just created.
    let r = unsafe { libc::setpriority(libc::PRIO_PROCESS, child.id() as libc::id_t, 10) };
    if r != 0 {
        log::warn!("could not lower worker priority: {}", std::io::Error::last_os_error());
    }
}

#[cfg(windows)]
fn lower_priority(child: &Child) {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Threading::{SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS};
    // SAFETY: the handle belongs to `child`, which outlives this call.
    if let Err(e) = unsafe { SetPriorityClass(HANDLE(child.as_raw_handle()), BELOW_NORMAL_PRIORITY_CLASS) } {
        log::warn!("could not lower worker priority: {e}");
    }
}

#[cfg(not(any(unix, windows)))]
fn lower_priority(_: &Child) {}

/// Decodes a status-0 payload.
pub(crate) fn samples(payload: &[u8]) -> Vec<f32> {
    payload.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn lowers_the_child_priority() {
        let child = Command::new("sleep").arg("5").spawn().unwrap();
        lower_priority(&child);
        // SAFETY: reading the priority of a live child process.
        let nice = unsafe { libc::getpriority(libc::PRIO_PROCESS, child.id() as libc::id_t) };
        let mut child = child;
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(nice, 10);
    }
}
