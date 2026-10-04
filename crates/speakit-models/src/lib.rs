//! Model catalog, verified downloads, and installation state (spec §7.4–7.5).
//!
//! Every file is pinned to an exact revision, byte size, and SHA-256. A
//! model version counts as installed only after all files verify and the
//! `installed.json` marker is written last. Nothing here executes code from
//! the catalog.

pub mod catalog;
mod download;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use catalog::FeatureSpec;
pub use catalog::{find, models, Artifact, Family, FileSpec, ModelSpec, VoiceSpec};

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("Download was cancelled.")]
    Cancelled,
    #[error("Not enough disk space: {needed} MB needed, {available} MB free.")]
    DiskSpace { needed: u64, available: u64 },
    #[error("The download could not be verified. Download again.")]
    HashMismatch,
    #[error("The download failed: {0}")]
    Network(String),
    #[error("Files could not be written: {0}")]
    Io(String),
}

impl From<std::io::Error> for ModelError {
    fn from(e: std::io::Error) -> Self {
        ModelError::Io(e.to_string())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done_bytes: u64,
    pub total_bytes: u64,
    /// True while hashing a file already on disk.
    pub verifying: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMarker {
    pub model_id: String,
    pub artifact_id: String,
    pub revision: String,
    pub voices: Vec<String>,
}

/// Paths of one installed model version.
#[derive(Debug, Clone)]
pub struct Installed {
    pub marker: InstalledMarker,
    pub dir: PathBuf,
    pub model_file: PathBuf,
}

impl Installed {
    pub fn voice_file(&self, voice: &str) -> PathBuf {
        self.dir.join("voices").join(format!("{voice}.bin"))
    }

    /// Where a model-level support file (`ModelSpec::files`) is stored.
    pub fn support_file(&self, file: &FileSpec) -> PathBuf {
        self.dir.join(file_name(&file.path))
    }
}

pub struct Store {
    root: PathBuf,
}

impl Store {
    /// `root` is the per-user models directory, e.g. `<app data>/models`.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn version_dir(&self, model: &ModelSpec, artifact: &Artifact) -> PathBuf {
        self.root.join(&model.id).join(format!("{}-{}", artifact.id, &model.revision[..8]))
    }

    /// The installed artifact of `model`, if its marker verifies.
    pub fn installed(&self, model: &ModelSpec) -> Option<Installed> {
        model.artifacts.iter().find_map(|a| {
            let dir = self.version_dir(model, a);
            let marker: InstalledMarker =
                serde_json::from_slice(&std::fs::read(dir.join("installed.json")).ok()?).ok()?;
            let model_file = dir.join(file_name(&a.file.path));
            let complete = |f: &FileSpec| {
                std::fs::metadata(dir.join(file_name(&f.path))).map(|m| m.len() == f.bytes).unwrap_or(false)
            };
            let ok = marker.revision == model.revision && complete(&a.file) && model.files.iter().all(complete);
            ok.then_some(Installed { marker, dir, model_file })
        })
    }

    /// Downloads, verifies, and atomically marks an artifact installed.
    /// A previously installed artifact stays usable until this succeeds.
    pub fn install(
        &self,
        model: &ModelSpec,
        artifact_id: &str,
        cancel: &Cancel,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<Installed, ModelError> {
        let artifact = model
            .artifacts
            .iter()
            .find(|a| a.id == artifact_id)
            .ok_or_else(|| ModelError::Io(format!("unknown artifact {artifact_id}")))?;
        let dir = self.version_dir(model, artifact);
        std::fs::create_dir_all(dir.join("voices"))?;

        let mut files: Vec<(&FileSpec, PathBuf)> = vec![(&artifact.file, dir.join(file_name(&artifact.file.path)))];
        for f in &model.files {
            files.push((f, dir.join(file_name(&f.path))));
        }
        for v in &model.voices {
            files.push((&v.file, dir.join("voices").join(format!("{}.bin", v.id))));
        }
        let total: u64 = files.iter().map(|(f, _)| f.bytes).sum();

        // Free space for everything not yet present, plus temp overhead.
        let missing: u64 = files
            .iter()
            .filter(|(f, p)| std::fs::metadata(p).map(|m| m.len() != f.bytes).unwrap_or(true))
            .map(|(f, _)| f.bytes)
            .sum();
        if let Some(available) = free_space(&dir) {
            let needed = missing + missing / 10 + (16 << 20);
            if available < needed {
                return Err(ModelError::DiskSpace { needed: needed >> 20, available: available >> 20 });
            }
        }

        let mut done = 0u64;
        for (spec, path) in &files {
            let base = done;
            download::fetch_verified(&model.url(&spec.path), spec, path, cancel, &mut |d, verifying| {
                progress(Progress { done_bytes: base + d, total_bytes: total, verifying })
            })?;
            done += spec.bytes;
        }

        let marker = InstalledMarker {
            model_id: model.id.clone(),
            artifact_id: artifact.id.clone(),
            revision: model.revision.clone(),
            voices: model.voices.iter().map(|v| v.id.clone()).collect(),
        };
        let tmp = dir.join("installed.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&marker).map_err(|e| ModelError::Io(e.to_string()))?)?;
        std::fs::rename(&tmp, dir.join("installed.json"))?;
        // Remove other artifacts of this model only after the new one works.
        for other in model.artifacts.iter().filter(|a| a.id != artifact.id) {
            let _ = std::fs::remove_dir_all(self.version_dir(model, other));
        }
        Ok(Installed { model_file: dir.join(file_name(&artifact.file.path)), dir, marker })
    }

    /// Catalog voices of an installed model whose files are not on disk,
    /// e.g. after the catalog gained voices. Fetched by `install` again.
    pub fn missing_voices<'m>(&self, model: &'m ModelSpec) -> Vec<&'m VoiceSpec> {
        let Some(installed) = self.installed(model) else { return Vec::new() };
        model
            .voices
            .iter()
            .filter(|v| {
                std::fs::metadata(installed.voice_file(&v.id)).map(|m| m.len() != v.file.bytes).unwrap_or(true)
            })
            .collect()
    }

    pub fn remove(&self, model: &ModelSpec) -> Result<(), ModelError> {
        let dir = self.root.join(&model.id);
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        Ok(())
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(unix)]
fn free_space(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c` is a valid NUL-terminated path and `s` is writable.
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 {
        return None;
    }
    Some(s.f_bavail as u64 * s.f_frsize as u64)
}

#[cfg(windows)]
fn free_space(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available = 0u64;
    // SAFETY: `wide` is NUL-terminated; unused outputs are null.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, std::ptr::null_mut(), std::ptr::null_mut()) };
    (ok != 0).then_some(available)
}

#[cfg(not(any(unix, windows)))]
fn free_space(_: &Path) -> Option<u64> {
    None
}
