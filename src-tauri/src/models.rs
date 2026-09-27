//! Voice models for the Voices screen (spec §4.2, §7.5): the built-in
//! system voices plus every downloadable model in the catalog.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use speakit_models::{Cancel, Family, Installed, ModelError, ModelSpec, Progress, Store};
use speakit_tts::espeak::Phonemizer;
use speakit_tts::kokoro::{KokoroConfig, KokoroEngine, KokoroVoice};
use speakit_tts::pocket::{PocketConfig, PocketEngine, PocketVoice};
use speakit_tts::{Engine, Registry};
use tauri::{AppHandle, Emitter, Manager};

use crate::settings::{Processor, ResourceProfile};
use crate::AppState;

/// ID of the built-in system voice pseudo-model.
pub const SYSTEM_MODEL: &str = "system";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactView {
    id: String,
    label: String,
    description: String,
    bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    id: String,
    name: String,
    description: String,
    /// Built into the OS: nothing to download or delete.
    built_in: bool,
    /// Voice IDs of this model start with `<voicePrefix>:`; empty for system.
    voice_prefix: String,
    license: Option<String>,
    license_url: Option<String>,
    card_url: Option<String>,
    artifacts: Vec<ArtifactView>,
    voices_bytes: u64,
    voice_count: usize,
    /// Catalog voices not yet on disk for an installed model.
    voices_missing: usize,
    voices_missing_bytes: u64,
    installed_artifact: Option<String>,
    downloading: Option<String>,
    progress: Option<Progress>,
    /// Recent download speed in bytes per second, once it can be measured.
    bytes_per_second: Option<f64>,
    /// Estimated seconds left at the recent speed.
    seconds_left: Option<u64>,
    error: Option<String>,
    /// Requirements that are missing on this computer, e.g. `espeak-ng`.
    missing: Vec<String>,
}

struct Job {
    artifact: String,
    cancel: Cancel,
    progress: Option<Progress>,
    /// (time, bytes done) samples for a sliding-window speed.
    samples: VecDeque<(Instant, u64)>,
}

/// Averaging window for the displayed speed.
const SPEED_WINDOW: Duration = Duration::from_secs(4);

impl Job {
    fn record(&mut self, p: Progress) {
        let now = Instant::now();
        // A restart from zero (e.g. a new file after verification) or a
        // verify pass is not download throughput.
        if p.verifying || self.samples.back().is_some_and(|&(_, b)| p.done_bytes < b) {
            self.samples.clear();
        }
        if !p.verifying {
            self.samples.push_back((now, p.done_bytes));
        }
        while self.samples.len() > 2 && self.samples.front().is_some_and(|&(t, _)| now - t > SPEED_WINDOW) {
            self.samples.pop_front();
        }
        self.progress = Some(p);
    }

    fn speed(&self) -> Option<f64> {
        let (&(t0, b0), &(t1, b1)) = (self.samples.front()?, self.samples.back()?);
        let dt = (t1 - t0).as_secs_f64();
        (dt >= 0.75 && b1 > b0).then(|| (b1 - b0) as f64 / dt)
    }
}

#[derive(Default)]
pub struct ModelManager {
    /// One download at a time, keyed by model ID.
    job: Mutex<Option<(String, Job)>>,
    errors: Mutex<HashMap<String, String>>,
}

pub fn store(app: &AppHandle) -> Store {
    let dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from(".")).join("models");
    Store::new(dir)
}

/// Inference workers ship next to the app executable; in development they
/// are built into the release target directory.
pub fn worker_bin(worker: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let name = if cfg!(windows) { format!("{worker}.exe") } else { worker.to_string() };
    [dir.join(&name), dir.join("../release").join(&name), dir.join("../debug").join(&name)]
        .into_iter()
        .find(|p| p.is_file())
}

fn worker_for(family: Family) -> &'static str {
    match family {
        Family::Kokoro => "sovirae-kokoro-worker",
        Family::Pocket => "sovirae-pocket-worker",
    }
}

/// Inference threads for a profile (spec §8.2).
pub fn threads_for(profile: ResourceProfile) -> usize {
    let p = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    match profile {
        ResourceProfile::Eco => 2.min(p.saturating_sub(1)).max(1),
        ResourceProfile::Balanced => 4.min(p / 2).max(1),
        ResourceProfile::Performance => p.saturating_sub(1).max(1),
    }
}

/// What this computer lacks to run a model family.
fn missing(model: &ModelSpec) -> Vec<String> {
    let mut out = Vec::new();
    if worker_bin(worker_for(model.family)).is_none() {
        out.push("voice engine".into());
    }
    if model.requires.iter().any(|r| r == "espeak-ng") && Phonemizer::find().is_none() {
        out.push("espeak-ng".into());
    }
    out
}

/// Builds the engine adapter for an installed model.
fn engine_for(model: &ModelSpec, installed: &Installed, profile: ResourceProfile, processor: Processor) -> Option<Arc<dyn Engine>> {
    match model.family {
        Family::Kokoro => {
            let voices = model
                .voices
                .iter()
                .map(|v| KokoroVoice {
                    id: v.id.clone(),
                    label: v.label.clone(),
                    language: v.language.clone(),
                    gender: v.gender.clone(),
                    g2p: v.g2p.clone(),
                    approximate_pronunciation: v.pronunciation.is_some(),
                    file: installed.voice_file(&v.id),
                })
                .collect();
            Some(Arc::new(KokoroEngine::new(KokoroConfig {
                model_id: model.id.clone(),
                model_name: model.name.clone(),
                voice_prefix: model.voice_prefix.clone(),
                model_file: installed.model_file.clone(),
                worker_bin: worker_bin(worker_for(model.family))?,
                voices,
                threads: threads_for(profile),
                gpu: processor == Processor::Gpu,
                model_rate: model.sample_rate,
            })))
        }
        Family::Pocket => {
            let voices = model
                .voices
                .iter()
                .map(|v| PocketVoice {
                    id: v.id.clone(),
                    label: v.label.clone(),
                    language: v.language.clone(),
                    gender: v.gender.clone(),
                    file: installed.voice_file(&v.id),
                })
                .collect();
            Some(Arc::new(PocketEngine::new(PocketConfig {
                model_id: model.id.clone(),
                model_name: model.name.clone(),
                voice_prefix: model.voice_prefix.clone(),
                model_file: installed.model_file.clone(),
                tokenizer_file: installed.support_file(model.files.first()?),
                worker_bin: worker_bin(worker_for(model.family))?,
                voices,
                threads: threads_for(profile),
                options: model.options.clone(),
                model_rate: model.sample_rate,
            })))
        }
    }
}

/// Registers every installed model with the engine registry at startup.
pub fn register_installed(app: &AppHandle, registry: &Registry, profile: ResourceProfile, processor: Processor) {
    let store = store(app);
    for model in speakit_models::models() {
        if let Some(installed) = store.installed(model) {
            registry.set_model(&model.voice_prefix, engine_for(model, &installed, profile, processor));
        }
    }
}

fn system_view(app: &AppHandle) -> ModelView {
    let state = app.state::<AppState>();
    let voice_count = state
        .engine
        .voices()
        .map(|v| v.iter().filter(|v| v.model == SYSTEM_MODEL).count())
        .unwrap_or(0);
    let os = if cfg!(target_os = "macos") { "macOS" } else if cfg!(windows) { "Windows" } else { "this system" };
    ModelView {
        id: SYSTEM_MODEL.into(),
        name: "System voices".into(),
        description: format!("Built into {os}. No download needed."),
        built_in: true,
        voice_prefix: String::new(),
        license: None,
        license_url: None,
        card_url: None,
        artifacts: Vec::new(),
        voices_bytes: 0,
        voice_count,
        voices_missing: 0,
        voices_missing_bytes: 0,
        installed_artifact: Some("built-in".into()),
        downloading: None,
        progress: None,
        bytes_per_second: None,
        seconds_left: None,
        error: None,
        missing: Vec::new(),
    }
}

fn model_view(app: &AppHandle, model: &ModelSpec) -> ModelView {
    let state = app.state::<AppState>();
    let store = store(app);
    let installed = store.installed(model);
    let missing_voices = store.missing_voices(model);
    let (downloading, progress, speed) = match state.models.job.lock().unwrap().as_ref() {
        Some((id, job)) if *id == model.id => (Some(job.artifact.clone()), job.progress, job.speed()),
        _ => (None, None, None),
    };
    let seconds_left = match (progress, speed) {
        (Some(p), Some(s)) if !p.verifying => Some((p.total_bytes.saturating_sub(p.done_bytes) as f64 / s).ceil() as u64),
        _ => None,
    };
    let error = state.models.errors.lock().unwrap().get(&model.id).cloned();
    ModelView {
        id: model.id.clone(),
        name: model.name.clone(),
        description: model.description.clone(),
        built_in: false,
        voice_prefix: model.voice_prefix.clone(),
        license: Some(model.license.clone()),
        license_url: Some(model.license_url.clone()),
        card_url: Some(model.card_url.clone()),
        artifacts: model
            .artifacts
            .iter()
            .map(|a| ArtifactView {
                id: a.id.clone(),
                label: a.label.clone(),
                description: a.description.clone(),
                bytes: a.file.bytes,
            })
            .collect(),
        voices_bytes: model.voices_bytes(),
        voice_count: model.voices.len(),
        voices_missing: missing_voices.len(),
        voices_missing_bytes: missing_voices.iter().map(|v| v.file.bytes).sum(),
        installed_artifact: installed.map(|i| i.marker.artifact_id),
        downloading,
        progress,
        bytes_per_second: speed,
        seconds_left,
        error,
        missing: missing(model),
    }
}

fn emit(app: &AppHandle, model: &ModelSpec) {
    let _ = app.emit("model", model_view(app, model));
}

fn find(id: &str) -> Result<&'static ModelSpec, String> {
    speakit_models::find(id).ok_or_else(|| format!("unknown model {id}"))
}

#[tauri::command]
pub async fn list_models(app: AppHandle) -> Vec<ModelView> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut all = vec![system_view(&app)];
        all.extend(speakit_models::models().iter().map(|m| model_view(&app, m)));
        all
    })
    .await
    .unwrap_or_default()
}

/// Starts a download after the user chose it; one download at a time.
#[tauri::command]
pub fn download_model(app: AppHandle, model: String, artifact: String) -> Result<(), String> {
    let spec = find(&model)?;
    if spec.artifact(&artifact).is_none() {
        return Err("unknown model version".into());
    }
    let state = app.state::<AppState>();
    let cancel = {
        let mut job = state.models.job.lock().unwrap();
        if let Some((other, _)) = job.as_ref() {
            let name = speakit_models::find(other).map_or("another model", |m| m.name.as_str());
            return Err(format!("Wait for {name} to finish downloading."));
        }
        let cancel = Cancel::default();
        *job = Some((
            model.clone(),
            Job { artifact: artifact.clone(), cancel: cancel.clone(), progress: None, samples: VecDeque::new() },
        ));
        cancel
    };
    state.models.errors.lock().unwrap().remove(&model);
    emit(&app, spec);

    std::thread::Builder::new()
        .name("model-download".into())
        .spawn(move || {
            let state = app.state::<AppState>();
            let mut last = Instant::now() - Duration::from_secs(1);
            let result = store(&app).install(spec, &artifact, &cancel, &mut |p| {
                if let Some((_, job)) = state.models.job.lock().unwrap().as_mut() {
                    job.record(p);
                }
                if last.elapsed() > Duration::from_millis(200) {
                    last = Instant::now();
                    emit(&app, spec);
                }
            });
            match result {
                Ok(installed) => {
                    let (profile, processor) = {
                        let s = state.settings.lock().unwrap();
                        (s.resource_profile, s.processor)
                    };
                    state.engine.set_model(&spec.voice_prefix, engine_for(spec, &installed, profile, processor));
                }
                Err(ModelError::Cancelled) => {}
                Err(e) => {
                    log::error!("{} download failed: {e}", spec.id);
                    state.models.errors.lock().unwrap().insert(spec.id.clone(), e.to_string());
                }
            }
            *state.models.job.lock().unwrap() = None;
            emit(&app, spec);
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn cancel_download(app: AppHandle, model: String) {
    if let Some((id, job)) = app.state::<AppState>().models.job.lock().unwrap().as_ref() {
        if *id == model {
            job.cancel.cancel();
        }
    }
}

/// Deletes a downloaded model. If one of its voices is selected, readings
/// fall back to the system voice; a reading in progress is stopped first.
#[tauri::command]
pub fn remove_model(app: AppHandle, model: String) -> Result<(), String> {
    let spec = find(&model)?;
    let state = app.state::<AppState>();
    if state.models.job.lock().unwrap().as_ref().is_some_and(|(id, _)| *id == model) {
        return Err("Cancel the download first.".into());
    }
    state.controller.send(crate::controller::Command::Stop);
    state.engine.set_model(&spec.voice_prefix, None);
    let prefix = format!("{}:", spec.voice_prefix);
    let uses_it = state.settings.lock().unwrap().voice.as_deref().is_some_and(|v| v.starts_with(&prefix));
    if uses_it {
        let _ = crate::commands::update_settings(app.clone(), app.state::<AppState>(), serde_json::json!({ "voice": null }));
    }
    store(&app).remove(spec).map_err(|e| e.to_string())?;
    state.models.errors.lock().unwrap().remove(&model);
    emit(&app, spec);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job() -> Job {
        Job { artifact: "a".into(), cancel: Cancel::default(), progress: None, samples: VecDeque::new() }
    }

    fn at(p: u64) -> Progress {
        Progress { done_bytes: p, total_bytes: 100_000_000, verifying: false }
    }

    #[test]
    fn measures_speed_over_a_sliding_window() {
        let mut j = job();
        let start = Instant::now();
        j.samples.push_back((start, 0));
        j.samples.push_back((start + Duration::from_secs(2), 20_000_000));
        let s = j.speed().unwrap();
        assert!((s - 10_000_000.0).abs() < 1.0, "{s}");
    }

    #[test]
    fn no_speed_until_enough_time_or_while_verifying() {
        let mut j = job();
        j.record(at(1_000));
        assert_eq!(j.speed(), None, "one sample is not a rate");
        j.record(Progress { done_bytes: 0, total_bytes: 1, verifying: true });
        assert!(j.samples.is_empty(), "verification resets the window");
    }
}
