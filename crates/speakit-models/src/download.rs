//! Resumable, hash-verified HTTPS download of one file.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{Cancel, FileSpec, ModelError};

const CHUNK: usize = 256 * 1024;

/// Ensures `dest` holds exactly `spec`. Reuses a verified existing file,
/// resumes a `.part` file when the server confirms the same content, and
/// renames into place only after size and SHA-256 match.
pub fn fetch_verified(
    url: &str,
    spec: &FileSpec,
    dest: &Path,
    cancel: &Cancel,
    progress: &mut dyn FnMut(u64, bool),
) -> Result<(), ModelError> {
    if std::fs::metadata(dest).map(|m| m.len() == spec.bytes).unwrap_or(false) {
        progress(0, true);
        if hash_file(dest, cancel)? == spec.sha256 {
            progress(spec.bytes, false);
            return Ok(());
        }
        std::fs::remove_file(dest)?;
    }

    let part = with_suffix(dest, "part");
    let etag_file = with_suffix(dest, "part.etag");
    let mut offset = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    let saved_etag = std::fs::read_to_string(&etag_file).ok();
    if offset > spec.bytes || saved_etag.is_none() {
        offset = 0;
    }

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(20)))
        .timeout_recv_body(Some(std::time::Duration::from_secs(60)))
        .build()
        .into();
    let mut req = agent.get(url);
    if offset > 0 {
        // Resume only if the server still has the same entity.
        req = req.header("Range", &format!("bytes={offset}-")).header("If-Range", saved_etag.as_deref().unwrap_or(""));
    }
    let mut resp = req.call().map_err(|e| ModelError::Network(e.to_string()))?;
    let resumed = resp.status() == 206 && offset > 0;
    if !resumed {
        offset = 0;
    }
    if let Some(etag) = resp.headers().get("etag").and_then(|v| v.to_str().ok()) {
        let _ = std::fs::write(&etag_file, etag);
    }

    let mut hasher = Sha256::new();
    if resumed {
        // Rehash the bytes already on disk so the final check covers them.
        let mut f = File::open(&part)?;
        let mut buf = vec![0u8; CHUNK];
        let mut left = offset;
        while left > 0 {
            let n = f.read(&mut buf[..CHUNK.min(left as usize)])?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            left -= n as u64;
        }
    }
    let mut file = OpenOptions::new().create(true).write(true).append(resumed).truncate(!resumed).open(&part)?;
    let mut body = resp.body_mut().as_reader();
    let mut buf = vec![0u8; CHUNK];
    let mut written = offset;
    progress(written, false);
    loop {
        if cancel.is_cancelled() {
            return Err(ModelError::Cancelled);
        }
        let n = body.read(&mut buf).map_err(|e| ModelError::Network(e.to_string()))?;
        if n == 0 {
            break;
        }
        written += n as u64;
        if written > spec.bytes {
            drop(file);
            let _ = std::fs::remove_file(&part);
            return Err(ModelError::HashMismatch);
        }
        file.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
        progress(written, false);
    }
    file.sync_all()?;
    drop(file);

    if written != spec.bytes {
        return Err(ModelError::Network(format!("connection ended after {written} of {} bytes", spec.bytes)));
    }
    if hex(&hasher.finalize()) != spec.sha256 {
        let _ = std::fs::remove_file(&part);
        let _ = std::fs::remove_file(&etag_file);
        return Err(ModelError::HashMismatch);
    }
    std::fs::rename(&part, dest)?;
    let _ = std::fs::remove_file(&etag_file);
    Ok(())
}

fn hash_file(path: &Path, cancel: &Cancel) -> Result<String, ModelError> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        if cancel.is_cancelled() {
            return Err(ModelError::Cancelled);
        }
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn with_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".");
    s.push(suffix);
    PathBuf::from(s)
}
