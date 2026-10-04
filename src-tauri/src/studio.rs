//! Studio: script projects turned into downloadable audio (user feature
//! 2026-10-04). A project keeps a script with its iterations, its own voice,
//! and the audio generated from it. Generation runs paragraph by paragraph:
//! a short first part the user listens to and accepts, then the rest.
//!
//! Scripts may carry markers in square brackets. The app itself honours
//! pauses (`[pause]`, `[pause 2s]`); a model's own markers come from the
//! catalog (`features`) and are passed through. Any other bracketed marker
//! is removed so it is never read aloud as words.
//!
//! Files live in `<app data>/studio/<project id>/`: `project.json` plus one
//! WAV per generated paragraph.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use speakit_core::{pronounce, PronunciationRule, SpeechDocument};
use speakit_tts::{CancelToken, Engine, TtsError};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::AppState;

/// Generated audio is 24 kHz mono, the native rate of the neural models.
const SAMPLE_RATE: u32 = 24_000;
const SHORT_PAUSE_MS: u64 = 600;
const MAX_PAUSE_MS: u64 = 10_000;
/// Silence between paragraphs in an exported file.
const PARAGRAPH_GAP_MS: u64 = 700;
/// The first part grows until it has this much text or this many paragraphs.
const PREVIEW_MIN_CHARS: usize = 200;
const PREVIEW_MAX_PARAGRAPHS: usize = 3;
const MAX_SCRIPT_CHARS: usize = 200_000;
const MAX_NAME_CHARS: usize = 80;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub id: u32,
    pub created: u64,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParagraphStatus {
    Pending,
    Generating,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paragraph {
    pub index: usize,
    /// The paragraph as generated. Edits are found by comparing it with the
    /// script, so only changed paragraphs are generated again.
    #[serde(default)]
    pub text: String,
    /// Opening words, for the list.
    pub preview: String,
    pub chars: usize,
    pub status: ParagraphStatus,
    pub file: Option<String>,
    pub duration_ms: u64,
    /// Bracketed markers the voice does not support, dropped from the text.
    pub skipped_markers: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Generated {
    /// Script iteration the audio was made from.
    pub version: u32,
    pub voice: String,
    pub paragraphs: Vec<Paragraph>,
    /// Paragraphs in the first part the user listens to before the rest.
    pub preview_count: usize,
    /// The user accepted the first part (or asked for everything).
    pub accepted: bool,
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created: u64,
    pub updated: u64,
    /// Voice ID for this project, independent of the reading voice.
    pub voice: Option<String>,
    /// Script iterations, oldest first.
    pub versions: Vec<Version>,
    /// ID of the iteration being edited.
    pub current: u32,
    pub generated: Option<Generated>,
}

impl Project {
    fn version(&self) -> Option<&Version> {
        self.versions.iter().find(|v| v.id == self.current)
    }

    fn touch(&mut self) {
        self.updated = now();
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub updated: u64,
    pub voice: Option<String>,
    pub ready: usize,
    pub total: usize,
    pub running: bool,
}

/// A marker the Studio offers for scripts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    pub id: String,
    pub label: String,
    pub description: String,
    pub example: String,
    /// `app` markers work with every voice; `model` ones come from the
    /// model's catalog entry.
    pub source: &'static str,
    /// Spoken as ordinary words the voice can say, not a true expression.
    pub approximate: bool,
}

/// Expressions every voice can approximate by saying a sound. A model that
/// declares the same tag in its catalog `features` takes precedence and
/// receives the tag itself.
const EXPRESSIONS: &[(&str, &str, &str)] = &[
    // (tag, label, spoken form)
    ("laugh", "Laugh", "Ha ha ha!"),
    ("chuckle", "Chuckle", "Heh heh."),
    ("giggle", "Giggle", "Hee hee hee!"),
    ("sigh", "Sigh", "Ahh…"),
    ("clear throat", "Clear throat", "Ahem."),
    ("gasp", "Gasp", "Oh!"),
    ("hmm", "Thinking sound", "Hmm."),
];

/// Looks an expression up by the text inside the brackets, accepting
/// `[laughs]`, `[clears throat]`, and `[clear-throat]` as well.
fn expression(inner: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    let key = inner.to_lowercase().replace(['-', '_'], " ");
    let key = key.split_whitespace().collect::<Vec<_>>().join(" ");
    EXPRESSIONS.iter().find(|(tag, _, _)| {
        let mut words = tag.split(' ').zip(key.split(' '));
        tag.split(' ').count() == key.split(' ').count()
            && words.all(|(t, k)| k == t || k == format!("{t}s") || k == format!("{t}es"))
    })
}

#[derive(Default)]
pub struct Studio {
    /// The one generation job allowed at a time: (project id, cancel).
    job: Mutex<Option<(String, CancelToken)>>,
    /// Serializes read-modify-write of project files.
    io: Mutex<()>,
    /// Project folder used instead of the app data directory (tests).
    pub root: Mutex<Option<PathBuf>>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn root<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    if let Some(r) = app.try_state::<AppState>().and_then(|s| s.studio.root.lock().unwrap().clone()) {
        return r;
    }
    app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from(".")).join("studio")
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 32 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn dir<R: Runtime>(app: &AppHandle<R>, id: &str) -> Result<PathBuf, String> {
    if !valid_id(id) {
        return Err("unknown project".into());
    }
    Ok(root(app).join(id))
}

fn load<R: Runtime>(app: &AppHandle<R>, id: &str) -> Result<Project, String> {
    let path = dir(app, id)?.join("project.json");
    let bytes = std::fs::read(&path).map_err(|_| "This project no longer exists.".to_string())?;
    let mut p: Project = serde_json::from_slice(&bytes).map_err(|e| format!("project file unreadable: {e}"))?;
    p.id = id.to_string();
    migrate(&mut p);
    Ok(p)
}

/// Projects generated before paragraphs stored their text would see every
/// paragraph as changed. Fill the text in from the script they were made
/// from, as long as it still splits into the same number of paragraphs.
fn migrate(p: &mut Project) {
    let Project { generated: Some(g), versions, .. } = p else { return };
    if g.paragraphs.iter().all(|q| !q.text.is_empty()) {
        return;
    }
    let Some(v) = versions.iter().find(|v| v.id == g.version) else { return };
    let split = split_paragraphs(&v.text);
    if split.len() == g.paragraphs.len() {
        for (q, t) in g.paragraphs.iter_mut().zip(split) {
            if q.text.is_empty() {
                q.text = t;
            }
        }
    }
}

/// Writes atomically so a crash never leaves a truncated file.
fn save<R: Runtime>(app: &AppHandle<R>, p: &Project) -> Result<(), String> {
    let d = dir(app, &p.id)?;
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let tmp = d.join("project.json.tmp");
    let bytes = serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, d.join("project.json")).map_err(|e| e.to_string())
}

fn emit<R: Runtime>(app: &AppHandle<R>, p: &Project) {
    let _ = app.emit("studio", p);
}

/// Loads, changes, saves, and broadcasts one project under the I/O lock.
fn update<R: Runtime>(app: &AppHandle<R>, id: &str, f: impl FnOnce(&mut Project) -> Result<(), String>) -> Result<Project, String> {
    let state = app.state::<AppState>();
    let _guard = state.studio.io.lock().unwrap();
    let mut p = load(app, id)?;
    f(&mut p)?;
    p.touch();
    save(app, &p)?;
    emit(app, &p);
    Ok(p)
}

fn running<R: Runtime>(app: &AppHandle<R>, id: &str) -> bool {
    app.state::<AppState>().studio.job.lock().unwrap().as_ref().is_some_and(|(j, _)| j == id)
}

fn clean_name(name: &str) -> String {
    let name: String = name.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(MAX_NAME_CHARS).collect();
    if name.is_empty() { "Untitled project".into() } else { name }
}

// ── Commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn studio_list<R: Runtime>(app: AppHandle<R>) -> Vec<ProjectSummary> {
    let Ok(entries) = std::fs::read_dir(root(&app)) else { return Vec::new() };
    let mut out: Vec<ProjectSummary> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().into_owned();
            let p = load(&app, &id).ok()?;
            let (ready, total) = p.generated.as_ref().map_or((0, 0), |g| {
                (g.paragraphs.iter().filter(|q| q.status == ParagraphStatus::Ready).count(), g.paragraphs.len())
            });
            Some(ProjectSummary { running: running(&app, &id), id, name: p.name, updated: p.updated, voice: p.voice, ready, total })
        })
        .collect();
    out.sort_by(|a, b| b.updated.cmp(&a.updated));
    out
}

#[tauri::command]
pub fn studio_create<R: Runtime>(app: AppHandle<R>, name: String) -> Result<Project, String> {
    let t = now();
    let id = format!("{t:x}-{:04x}", std::process::id() ^ (t as u32).rotate_left(7) & 0xffff);
    let p = Project {
        id,
        name: clean_name(&name),
        created: t,
        updated: t,
        voice: None,
        versions: vec![Version { id: 1, created: t, text: String::new() }],
        current: 1,
        generated: None,
    };
    save(&app, &p)?;
    emit(&app, &p);
    Ok(p)
}

#[tauri::command]
pub fn studio_get<R: Runtime>(app: AppHandle<R>, id: String) -> Result<Project, String> {
    load(&app, &id)
}

#[tauri::command]
pub fn studio_rename<R: Runtime>(app: AppHandle<R>, id: String, name: String) -> Result<Project, String> {
    update(&app, &id, |p| {
        p.name = clean_name(&name);
        Ok(())
    })
}

#[tauri::command]
pub fn studio_delete<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), String> {
    if running(&app, &id) {
        return Err("Stop the generation first.".into());
    }
    let state = app.state::<AppState>();
    let _guard = state.studio.io.lock().unwrap();
    std::fs::remove_dir_all(dir(&app, &id)?).map_err(|e| e.to_string())
}

/// Saves the script into the current iteration, or as a new one.
#[tauri::command]
pub fn studio_save_script<R: Runtime>(app: AppHandle<R>, id: String, text: String, new_version: bool) -> Result<Project, String> {
    if text.chars().count() > MAX_SCRIPT_CHARS {
        return Err(format!("The script is longer than {MAX_SCRIPT_CHARS} characters."));
    }
    update(&app, &id, |p| {
        if new_version {
            let next = p.versions.iter().map(|v| v.id).max().unwrap_or(0) + 1;
            p.versions.push(Version { id: next, created: now(), text });
            p.current = next;
        } else if let Some(v) = p.versions.iter_mut().find(|v| v.id == p.current) {
            v.text = text;
        } else {
            return Err("unknown iteration".into());
        }
        Ok(())
    })
}

#[tauri::command]
pub fn studio_select_version<R: Runtime>(app: AppHandle<R>, id: String, version: u32) -> Result<Project, String> {
    update(&app, &id, |p| {
        if !p.versions.iter().any(|v| v.id == version) {
            return Err("unknown iteration".into());
        }
        p.current = version;
        Ok(())
    })
}

#[tauri::command]
pub fn studio_delete_version<R: Runtime>(app: AppHandle<R>, id: String, version: u32) -> Result<Project, String> {
    update(&app, &id, |p| {
        if p.versions.len() <= 1 {
            return Err("A project keeps at least one iteration.".into());
        }
        p.versions.retain(|v| v.id != version);
        if p.current == version {
            p.current = p.versions.last().map_or(1, |v| v.id);
        }
        Ok(())
    })
}

#[tauri::command]
pub fn studio_set_voice<R: Runtime>(app: AppHandle<R>, id: String, voice: String) -> Result<Project, String> {
    update(&app, &id, |p| {
        p.voice = Some(voice);
        Ok(())
    })
}

/// Markers available for scripts read by `voice`.
#[tauri::command]
pub fn studio_features(state: tauri::State<'_, AppState>, voice: Option<String>) -> Vec<Feature> {
    let mut out = app_features();
    let model = voice.as_deref().and_then(|v| model_of(&*state.engine, v)).map(|m| model_features(&m)).unwrap_or_default();
    out.extend(expression_features(&model));
    out.extend(model);
    out
}

/// Starts generating: `preview` (the first part), `rest` (after accepting
/// it), or `all` (everything again).
#[tauri::command]
pub fn studio_generate<R: Runtime>(app: AppHandle<R>, id: String, scope: String) -> Result<Project, String> {
    let scope = match scope.as_str() {
        "preview" => Scope::Preview,
        "rest" => Scope::Rest,
        "all" => Scope::All,
        _ => return Err("unknown scope".into()),
    };
    let state = app.state::<AppState>();
    let cancel = {
        let mut job = state.studio.job.lock().unwrap();
        if let Some((other, _)) = job.as_ref() {
            let name = load(&app, other).map_or("another project".to_string(), |p| p.name);
            return Err(format!("Wait for \"{name}\" to finish generating."));
        }
        let cancel = CancelToken::default();
        *job = Some((id.clone(), cancel.clone()));
        cancel
    };
    let started = prepare(&app, &id, scope);
    let p = match started {
        Ok(p) => p,
        Err(e) => {
            *state.studio.job.lock().unwrap() = None;
            return Err(e);
        }
    };
    let a = app.clone();
    std::thread::Builder::new()
        .name("studio-generate".into())
        .spawn(move || {
            run(&a, &id, scope, &cancel);
            *a.state::<AppState>().studio.job.lock().unwrap() = None;
            if let Ok(p) = load(&a, &id) {
                emit(&a, &p);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(p)
}

#[tauri::command]
pub fn studio_cancel<R: Runtime>(app: AppHandle<R>, id: String) {
    if let Some((job, cancel)) = app.state::<AppState>().studio.job.lock().unwrap().as_ref() {
        if *job == id {
            cancel.cancel();
        }
    }
}

/// One generated paragraph as a WAV file, for the in-app player.
#[tauri::command]
pub async fn studio_audio<R: Runtime>(app: AppHandle<R>, id: String, paragraph: usize) -> Result<tauri::ipc::Response, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let p = load(&app, &id)?;
        let file = p
            .generated
            .as_ref()
            .and_then(|g| g.paragraphs.get(paragraph))
            .and_then(|q| q.file.clone())
            .ok_or("This paragraph has no audio yet.")?;
        let bytes = std::fs::read(dir(&app, &id)?.join(file)).map_err(|e| e.to_string())?;
        Ok(tauri::ipc::Response::new(bytes))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Saves the first part or the whole project as one WAV file; returns the
/// path, or `None` when the user cancelled the dialog.
#[tauri::command]
pub async fn studio_export<R: Runtime>(app: AppHandle<R>, id: String, scope: String) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    tauri::async_runtime::spawn_blocking(move || {
        let p = load(&app, &id)?;
        let g = p.generated.as_ref().ok_or("Generate audio first.")?;
        let count = if scope == "preview" { g.preview_count.min(g.paragraphs.len()) } else { g.paragraphs.len() };
        let files: Vec<PathBuf> = g.paragraphs[..count]
            .iter()
            .filter_map(|q| q.file.as_ref().map(|f| (q, f)))
            .filter(|(q, _)| q.status == ParagraphStatus::Ready)
            .map(|(_, f)| dir(&app, &id).map(|d| d.join(f)))
            .collect::<Result<_, _>>()?;
        if files.is_empty() {
            return Err("Nothing has been generated yet.".into());
        }
        let suggested = format!(
            "{}{}.wav",
            p.name.chars().map(|c| if c.is_alphanumeric() || c == ' ' { c } else { '-' }).collect::<String>().trim(),
            if scope == "preview" { " (first part)" } else { "" }
        );
        let Some(target) = app
            .dialog()
            .file()
            .set_title("Save audio")
            .add_filter("WAV audio", &["wav"])
            .set_file_name(suggested)
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let target = target.into_path().map_err(|e| e.to_string())?;
        write_joined(&files, &target)?;
        Ok(Some(target.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| e.to_string())?
}

// ── Generation ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Preview,
    Rest,
    All,
}

/// Validates the project and marks the paragraphs about to be generated.
fn prepare<R: Runtime>(app: &AppHandle<R>, id: &str, scope: Scope) -> Result<Project, String> {
    let state = app.state::<AppState>();
    let voices = state.engine.voices().map_err(|e| e.to_string())?;
    update(app, id, |p| {
        let voice = p.voice.clone().ok_or("Choose a voice for this project.")?;
        if !voices.iter().any(|v| v.id == voice) {
            return Err("The project's voice is no longer available. Choose another.".into());
        }
        let version = p.version().ok_or("unknown iteration")?;
        let version_id = version.id;
        let paragraphs = split_paragraphs(&version.text);
        if paragraphs.is_empty() {
            return Err("Write or paste a script first.".into());
        }
        // Audio made for another iteration or voice, or a full regeneration,
        // is not reused. Otherwise paragraphs whose text is unchanged keep
        // their audio and only edited, added, or failed ones are generated.
        let mut old = p.generated.take().filter(|g| scope != Scope::All && g.version == version_id && g.voice == voice);
        // "Generate first part" always makes the first part again, so the
        // user hears the current script before accepting it.
        let n_preview = preview_count(&paragraphs);
        let mut reused: Vec<Paragraph> = Vec::new();
        for (i, t) in paragraphs.iter().enumerate() {
            let kept = old.as_mut().filter(|_| !(scope == Scope::Preview && i < n_preview)).and_then(|g| {
                let at = g.paragraphs.iter().position(|q| q.status == ParagraphStatus::Ready && q.text == *t)?;
                Some(g.paragraphs.remove(at))
            });
            reused.push(match kept {
                Some(q) => Paragraph { index: i, ..q },
                None => Paragraph {
                    index: i,
                    text: t.clone(),
                    preview: t.chars().take(90).collect(),
                    chars: t.chars().count(),
                    status: ParagraphStatus::Pending,
                    file: None,
                    duration_ms: 0,
                    skipped_markers: 0,
                    error: None,
                },
            });
        }
        // Files no paragraph refers to any more are removed.
        let d = dir(app, id)?;
        let keep: Vec<&String> = reused.iter().filter_map(|q| q.file.as_ref()).collect();
        if let Ok(entries) = std::fs::read_dir(&d) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.ends_with(".wav") && !keep.contains(&&name) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        // After the first part the user accepts it again before the rest.
        let accepted = scope != Scope::Preview;
        p.generated = Some(Generated {
            version: version_id,
            voice: voice.clone(),
            preview_count: n_preview,
            paragraphs: reused,
            accepted,
            running: true,
            error: None,
        });
        Ok(())
    })
}

fn targeted(scope: Scope, preview_count: usize, i: usize) -> bool {
    match scope {
        Scope::Preview => i < preview_count,
        Scope::Rest | Scope::All => true,
    }
}

fn run<R: Runtime>(app: &AppHandle<R>, id: &str, scope: Scope, cancel: &CancelToken) {
    let state = app.state::<AppState>();
    let engine = state.engine.clone();
    let rules = state.settings.lock().unwrap().pronunciations.clone();
    let Ok(p) = load(app, id) else { return };
    let Some(g) = p.generated.as_ref() else { return };
    let voice = g.voice.clone();
    let allowed: Vec<String> = model_of(&*engine, &voice).map(|m| model_features(&m).into_iter().map(|f| f.id).collect()).unwrap_or_default();
    let paragraphs: Vec<String> = g.paragraphs.iter().map(|q| q.text.clone()).collect();
    let d = match dir(app, id) {
        Ok(d) => d,
        Err(_) => return,
    };
    let todo: Vec<usize> = g
        .paragraphs
        .iter()
        .filter(|q| targeted(scope, g.preview_count, q.index) && q.status != ParagraphStatus::Ready)
        .map(|q| q.index)
        .collect();
    let version_id = g.version;

    let mut stopped = false;
    for i in todo {
        if cancel.is_cancelled() {
            stopped = true;
            break;
        }
        let _ = update(app, id, |p| {
            if let Some(q) = p.generated.as_mut().and_then(|g| g.paragraphs.get_mut(i)) {
                q.status = ParagraphStatus::Generating;
            }
            Ok(())
        });
        let result = synthesize_paragraph(&*engine, &paragraphs[i], &voice, &rules, &allowed, cancel);
        let outcome = result.and_then(|(samples, skipped)| {
            // Named by content, so a paragraph that moves keeps its file and a
            // rewritten one never overwrites audio another paragraph reuses.
            let file = format!("v{version_id}-p{i}-{:08x}.wav", fingerprint(&paragraphs[i]));
            write_wav(&d.join(&file), &samples).map_err(TtsError::Engine)?;
            Ok((file, samples.len() as u64 * 1000 / SAMPLE_RATE as u64, skipped))
        });
        let cancelled = matches!(outcome, Err(TtsError::Cancelled));
        let _ = update(app, id, |p| {
            let Some(q) = p.generated.as_mut().and_then(|g| g.paragraphs.get_mut(i)) else { return Ok(()) };
            match outcome {
                Ok((file, duration_ms, skipped)) => {
                    q.status = ParagraphStatus::Ready;
                    q.file = Some(file);
                    q.duration_ms = duration_ms;
                    q.skipped_markers = skipped;
                    q.error = None;
                }
                Err(TtsError::Cancelled) => q.status = ParagraphStatus::Pending,
                Err(e) => {
                    log::error!("studio paragraph {i} failed: {e}");
                    q.status = ParagraphStatus::Failed;
                    q.error = Some(e.to_string());
                }
            }
            Ok(())
        });
        if cancelled {
            stopped = true;
            break;
        }
    }
    let _ = update(app, id, |p| {
        if let Some(g) = p.generated.as_mut() {
            g.running = false;
            if stopped {
                g.error = Some("Generation stopped.".into());
            } else if g.paragraphs.iter().any(|q| q.status == ParagraphStatus::Failed) {
                g.error = Some("Some paragraphs could not be generated. Retry them or change the voice.".into());
            }
        }
        Ok(())
    });
}

fn fingerprint(text: &str) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    h.finish() as u32
}

/// Model (catalog ID) owning `voice`, if it is a downloaded model's voice.
fn model_of(engine: &dyn Engine, voice: &str) -> Option<String> {
    engine.voices().ok()?.into_iter().find(|v| v.id == voice).map(|v| v.model).filter(|m| m != "system")
}

fn model_features(model: &str) -> Vec<Feature> {
    speakit_models::find(model)
        .map(|m| {
            m.features
                .iter()
                .map(|f| Feature {
                    id: f.id.clone(),
                    label: f.label.clone(),
                    description: f.description.clone(),
                    example: f.example.clone(),
                    source: "model",
                    approximate: false,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Spoken-sound stand-ins for expressions, minus any the model does itself.
fn expression_features(model: &[Feature]) -> Vec<Feature> {
    EXPRESSIONS
        .iter()
        .filter(|(tag, _, _)| !model.iter().any(|f| f.id.eq_ignore_ascii_case(tag)))
        .map(|(tag, label, spoken)| Feature {
            id: tag.to_string(),
            label: label.to_string(),
            description: format!("Approximate: the voice says “{spoken}”."),
            example: format!("[{tag}]"),
            source: "app",
            approximate: true,
        })
        .collect()
}

/// Markers and cues the app itself provides, for every voice.
fn app_features() -> Vec<Feature> {
    let f = |id: &str, label: &str, description: &str, example: &str| Feature {
        id: id.into(),
        label: label.into(),
        description: description.into(),
        example: example.into(),
        source: "app",
        approximate: false,
    };
    vec![
        f("pause", "Short pause", "A beat of silence, about half a second.", "[pause]"),
        f("pause-length", "Timed pause", "Silence of a given length, up to 10 seconds.", "[pause 2s]"),
        f("paragraph", "New paragraph", "A blank line starts a new paragraph, generated and downloadable on its own.", "(blank line)"),
        f("question", "Question", "A question mark lifts the end of the sentence.", "Really?"),
        f("emphasis", "Emphasis", "An exclamation mark adds energy to the sentence.", "That's it!"),
        f("trail", "Trailing off", "An ellipsis slows the ending.", "I wonder…"),
    ]
}

/// Paragraphs of a script: blocks separated by blank lines, or, when the
/// script has no blank lines, each non-empty line.
pub fn split_paragraphs(text: &str) -> Vec<String> {
    let mut blocks: Vec<Vec<&str>> = vec![Vec::new()];
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            if !blocks.last().unwrap().is_empty() {
                blocks.push(Vec::new());
            }
        } else {
            blocks.last_mut().unwrap().push(line);
        }
    }
    blocks.retain(|b| !b.is_empty());
    if blocks.len() == 1 && blocks[0].len() > 1 {
        return blocks[0].iter().map(|l| l.to_string()).collect();
    }
    blocks.into_iter().map(|b| b.join(" ")).collect()
}

/// How many opening paragraphs form the first part.
pub fn preview_count(paragraphs: &[String]) -> usize {
    let mut chars = 0;
    let mut n = 0;
    for p in paragraphs {
        n += 1;
        chars += p.chars().count();
        if chars >= PREVIEW_MIN_CHARS || n >= PREVIEW_MAX_PARAGRAPHS {
            break;
        }
    }
    n.clamp(1, paragraphs.len().max(1))
}

enum Piece {
    Text(String),
    Pause(u64),
}

/// Splits a paragraph into text and pauses. Markers of the model pass
/// through inside the text; unknown markers are dropped and counted.
fn parse_markers(text: &str, allowed: &[String]) -> (Vec<Piece>, usize) {
    let mut pieces = Vec::new();
    let mut current = String::new();
    let mut skipped = 0;
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let (before, from_open) = rest.split_at(open);
        current.push_str(before);
        let Some(close) = from_open.find(']').filter(|&c| c <= 60 && !from_open[1..c].contains('[')) else {
            current.push('[');
            rest = &from_open[1..];
            continue;
        };
        let inner = from_open[1..close].trim();
        let word = inner.split_whitespace().next().unwrap_or("").to_lowercase();
        let pass_through = allowed.iter().any(|a| a.eq_ignore_ascii_case(&word) || a.eq_ignore_ascii_case(inner));
        if pass_through {
            current.push_str(&from_open[..=close]);
        } else if word == "pause" {
            let ms = inner.split_whitespace().nth(1).map_or(SHORT_PAUSE_MS, parse_duration_ms).min(MAX_PAUSE_MS);
            if !current.trim().is_empty() {
                pieces.push(Piece::Text(std::mem::take(&mut current)));
            } else {
                current.clear();
            }
            pieces.push(Piece::Pause(ms));
        } else if let Some((_, _, spoken)) = expression(inner) {
            // Keep the sound on its own so sentence splitting isolates it.
            if !current.ends_with(|c: char| c.is_whitespace()) && !current.is_empty() {
                current.push(' ');
            }
            current.push_str(spoken);
            current.push(' ');
        } else {
            skipped += 1;
        }
        rest = &from_open[close + 1..];
    }
    current.push_str(rest);
    if !current.trim().is_empty() {
        pieces.push(Piece::Text(current));
    }
    // Inserted sounds and removed markers leave doubled spaces behind.
    for piece in &mut pieces {
        if let Piece::Text(t) = piece {
            while t.contains("  ") {
                *t = t.replace("  ", " ");
            }
        }
    }
    (pieces, skipped)
}

/// `2s`, `1.5s`, `800ms`, or a bare number of seconds.
fn parse_duration_ms(s: &str) -> u64 {
    let s = s.trim().to_lowercase();
    let (num, unit) = match s.strip_suffix("ms") {
        Some(n) => (n, 1.0),
        None => (s.strip_suffix('s').unwrap_or(&s), 1000.0),
    };
    num.parse::<f64>().ok().filter(|n| n.is_finite() && *n >= 0.0).map_or(SHORT_PAUSE_MS, |n| (n * unit) as u64)
}

fn synthesize_paragraph(
    engine: &dyn Engine,
    text: &str,
    voice: &str,
    rules: &[PronunciationRule],
    allowed: &[String],
    cancel: &CancelToken,
) -> Result<(Vec<f32>, usize), TtsError> {
    let (pieces, skipped) = parse_markers(text, allowed);
    let mut out = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Pause(ms) => out.extend(std::iter::repeat(0.0).take((ms * SAMPLE_RATE as u64 / 1000) as usize)),
            Piece::Text(t) => {
                let doc = SpeechDocument::new(pronounce::apply(&t, rules), engine.max_segment_chars());
                for i in 0..doc.segments.len() {
                    if cancel.is_cancelled() {
                        return Err(TtsError::Cancelled);
                    }
                    let pcm = engine.synthesize(doc.segment_text(i), voice, SAMPLE_RATE, cancel)?;
                    out.extend(pcm.samples);
                }
            }
        }
    }
    Ok((out, skipped))
}

fn write_wav(path: &std::path::Path, samples: &[f32]) -> Result<(), String> {
    let spec = hound::WavSpec { channels: 1, sample_rate: SAMPLE_RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for s in samples {
        w.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}

/// Joins paragraph files with a short gap between them.
fn write_joined(files: &[PathBuf], target: &std::path::Path) -> Result<(), String> {
    let spec = hound::WavSpec { channels: 1, sample_rate: SAMPLE_RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(target, spec).map_err(|e| e.to_string())?;
    let gap = (PARAGRAPH_GAP_MS * SAMPLE_RATE as u64 / 1000) as usize;
    for (n, f) in files.iter().enumerate() {
        if n > 0 {
            for _ in 0..gap {
                w.write_sample(0i16).map_err(|e| e.to_string())?;
            }
        }
        let mut r = hound::WavReader::open(f).map_err(|e| e.to_string())?;
        for s in r.samples::<i16>() {
            w.write_sample(s.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        }
    }
    w.finalize().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mock Tauri app with the real engine registry, so the commands run
    /// end to end: project files, synthesis, WAV output.
    #[cfg(target_os = "macos")]
    fn mock_app(root: PathBuf) -> tauri::App<tauri::test::MockRuntime> {
        use std::sync::atomic::AtomicBool;
        use std::sync::{Arc, Mutex};
        struct NoHost;
        impl crate::controller::Host for NoHost {
            fn snapshot(&self, _: &speakit_core::PlaybackSnapshot) {}
            fn notice(&self, _: &crate::controller::Notice) {}
            fn document(&self, _: &crate::controller::DocumentView) {}
            fn envelope(&self, _: &crate::controller::Envelope) {}
            fn show_player(&self) {}
            fn hide_player(&self) {}
            fn playback_active(&self, _: bool) {}
        }
        let app = tauri::test::mock_builder().build(tauri::test::mock_context(tauri::test::noop_assets())).unwrap();
        let engine = Arc::new(speakit_tts::Registry::new(speakit_tts::system_engine()));
        let settings = crate::settings::Settings::default();
        let controller = crate::controller::spawn(
            engine.clone() as Arc<dyn Engine>,
            crate::controller::Config { match_language: false, rate: 1.0, volume: 0.0, voice: None, profile: settings.resource_profile, pronunciations: Vec::new() },
            NoHost,
        );
        let studio = Studio::default();
        *studio.root.lock().unwrap() = Some(root);
        app.manage(AppState {
            controller,
            engine,
            snapshot: Mutex::new(speakit_core::PlaybackSnapshot::idle(1.0, 1.0)),
            settings: Mutex::new(settings),
            store: crate::settings::Store::new(std::env::temp_dir()),
            document: Mutex::new(None),
            shortcuts: Default::default(),
            models: Default::default(),
            playback_active: AtomicBool::new(false),
            bridge: Default::default(),
            studio,
        });
        app
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore = "invokes the real system voice"]
    fn generates_first_part_then_rest_with_the_system_voice() {
        let root = std::env::temp_dir().join(format!("sovirae-studio-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let app = mock_app(root.clone());
        let h = app.handle().clone();

        let p = studio_create(h.clone(), "  Demo   project ".into()).unwrap();
        assert_eq!(p.name, "Demo project");
        assert!(studio_list(h.clone()).iter().any(|s| s.id == p.id));

        // No voice, no script: clear errors, nothing started.
        assert!(studio_generate(h.clone(), p.id.clone(), "preview".into()).unwrap_err().contains("voice"));
        let voice = h.state::<AppState>().engine.voices().unwrap().into_iter().find(|v| v.model == "system").expect("a system voice").id;
        studio_set_voice(h.clone(), p.id.clone(), voice.clone()).unwrap();
        assert!(studio_generate(h.clone(), p.id.clone(), "preview".into()).unwrap_err().contains("script"));

        let script = "Hello there. [laugh] [pause 1s] This is the first part.\n\nSecond paragraph here, long enough that the first two paragraphs together pass the two hundred character mark and so form the first part on their own. [whisper] Still second.\n\nThird one.";
        let p = studio_save_script(h.clone(), p.id.clone(), script.into(), false).unwrap();
        assert_eq!(p.versions.len(), 1);
        let p = studio_generate(h.clone(), p.id.clone(), "preview".into()).unwrap();
        let g = p.generated.as_ref().unwrap();
        assert_eq!(g.paragraphs.len(), 3);
        assert_eq!(g.preview_count, 2, "two short paragraphs make the first part");
        assert!(g.running && !g.accepted);

        let wait = |done: &dyn Fn(&Project) -> bool| {
            let start = std::time::Instant::now();
            loop {
                let p = studio_get(h.clone(), p.id.clone()).unwrap();
                if done(&p) { return p; }
                assert!(start.elapsed() < std::time::Duration::from_secs(60), "timed out");
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        };
        let p = wait(&|p| !p.generated.as_ref().unwrap().running);
        let g = p.generated.as_ref().unwrap();
        let st: Vec<ParagraphStatus> = g.paragraphs.iter().map(|q| q.status).collect();
        assert_eq!(st, [ParagraphStatus::Ready, ParagraphStatus::Ready, ParagraphStatus::Pending]);
        assert!(g.paragraphs[0].duration_ms >= 1000, "the pause is in the audio: {}", g.paragraphs[0].duration_ms);
        assert_eq!(g.paragraphs[1].skipped_markers, 1, "[whisper] is not supported by this voice");
        assert!(root.join(&p.id).join(g.paragraphs[0].file.as_ref().unwrap()).is_file());

        // Audio for the player.
        let bytes = tauri::async_runtime::block_on(studio_audio(h.clone(), p.id.clone(), 0)).unwrap();
        drop(bytes);
        assert!(tauri::async_runtime::block_on(studio_audio(h.clone(), p.id.clone(), 2)).is_err(), "nothing yet");

        // Accepting generates the rest only.
        let p = studio_generate(h.clone(), p.id.clone(), "rest".into()).unwrap();
        assert!(p.generated.as_ref().unwrap().accepted);
        let p = wait(&|p| !p.generated.as_ref().unwrap().running);
        assert!(p.generated.as_ref().unwrap().paragraphs.iter().all(|q| q.status == ParagraphStatus::Ready));

        // Editing one paragraph in the same iteration regenerates only it;
        // the others keep their audio files.
        let before: Vec<Option<String>> = p.generated.as_ref().unwrap().paragraphs.iter().map(|q| q.file.clone()).collect();
        let edited = script.replace("Third one.", "Third one, now edited.");
        studio_save_script(h.clone(), p.id.clone(), edited, false).unwrap();
        let p = studio_generate(h.clone(), p.id.clone(), "rest".into()).unwrap();
        let g = p.generated.as_ref().unwrap();
        assert_eq!(g.paragraphs.iter().map(|q| q.status).collect::<Vec<_>>(), [ParagraphStatus::Ready, ParagraphStatus::Ready, ParagraphStatus::Pending]);
        assert_eq!(g.paragraphs[0].file, before[0]);
        let p = wait(&|p| !p.generated.as_ref().unwrap().running);
        let g = p.generated.as_ref().unwrap();
        assert!(g.paragraphs.iter().all(|q| q.status == ParagraphStatus::Ready));
        assert_ne!(g.paragraphs[2].file, before[2]);
        assert_eq!(g.paragraphs[2].text, "Third one, now edited.");
        assert!(!root.join(&p.id).join(before[2].as_ref().unwrap()).exists(), "old audio of the edited paragraph removed");
        assert!(root.join(&p.id).join(before[0].as_ref().unwrap()).exists(), "unchanged audio kept");

        // A project saved before paragraphs stored their text (an older build)
        // still regenerates only what changed.
        let legacy_path = root.join(&p.id).join("project.json");
        let mut legacy: serde_json::Value = serde_json::from_slice(&std::fs::read(&legacy_path).unwrap()).unwrap();
        for q in legacy["generated"]["paragraphs"].as_array_mut().unwrap() {
            q.as_object_mut().unwrap().remove("text");
        }
        std::fs::write(&legacy_path, serde_json::to_vec(&legacy).unwrap()).unwrap();
        let p = studio_get(h.clone(), p.id.clone()).unwrap();
        assert!(p.generated.as_ref().unwrap().paragraphs.iter().all(|q| !q.text.is_empty()), "text restored from the script");
        let before: Vec<Option<String>> = p.generated.as_ref().unwrap().paragraphs.iter().map(|q| q.file.clone()).collect();
        let edited2 = script.replace("Third one.", "Third one, edited twice.").replace("Hello there.", "Hi there.");
        studio_save_script(h.clone(), p.id.clone(), edited2, false).unwrap();
        // Regenerating the first part redoes all of it and asks for acceptance
        // again; a changed paragraph after it waits for that acceptance.
        let p = studio_generate(h.clone(), p.id.clone(), "preview".into()).unwrap();
        let g = p.generated.as_ref().unwrap();
        assert_eq!(g.paragraphs.iter().map(|q| q.status).collect::<Vec<_>>(), [ParagraphStatus::Pending; 3]);
        assert!(!g.accepted);
        let p = wait(&|p| !p.generated.as_ref().unwrap().running);
        let g = p.generated.as_ref().unwrap();
        assert_eq!(g.paragraphs.iter().map(|q| q.status).collect::<Vec<_>>(), [ParagraphStatus::Ready, ParagraphStatus::Ready, ParagraphStatus::Pending]);
        assert_ne!(g.paragraphs[0].file, before[0]);
        // Accepting generates only what is missing after the first part.
        let p = studio_generate(h.clone(), p.id.clone(), "rest".into()).unwrap();
        assert!(p.generated.as_ref().unwrap().accepted);
        let p = wait(&|p| !p.generated.as_ref().unwrap().running);
        let g = p.generated.as_ref().unwrap();
        assert!(g.paragraphs.iter().all(|q| q.status == ParagraphStatus::Ready));
        assert_eq!(g.paragraphs[2].text, "Third one, edited twice.");
        assert_ne!(g.paragraphs[2].file, before[2]);

        // A new iteration makes the audio stale; generating again starts fresh.
        let p = studio_save_script(h.clone(), p.id.clone(), "Only one line now.".into(), true).unwrap();
        assert_eq!((p.versions.len(), p.current), (2, 2));
        let p = studio_generate(h.clone(), p.id.clone(), "all".into()).unwrap();
        assert_eq!(p.generated.as_ref().unwrap().paragraphs.len(), 1);
        let p = wait(&|p| !p.generated.as_ref().unwrap().running);
        assert_eq!(p.generated.as_ref().unwrap().paragraphs[0].status, ParagraphStatus::Ready);
        let files: Vec<String> = std::fs::read_dir(root.join(&p.id)).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert!(files.iter().all(|f| !f.starts_with("v1-")), "old audio removed: {files:?}");

        studio_delete(h.clone(), p.id.clone()).unwrap();
        assert!(studio_list(h.clone()).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn splits_paragraphs_on_blank_lines_or_lines() {
        assert_eq!(split_paragraphs("One.\nstill one.\n\nTwo.\n\n\n  Three. "), ["One. still one.", "Two.", "Three."]);
        assert_eq!(split_paragraphs("Line one.\nLine two.\nLine three."), ["Line one.", "Line two.", "Line three."]);
        assert_eq!(split_paragraphs("  \n\n"), Vec::<String>::new());
        assert_eq!(split_paragraphs("Just one."), ["Just one."]);
    }

    #[test]
    fn first_part_grows_with_short_paragraphs() {
        let short = |n: usize| "x".repeat(n);
        assert_eq!(preview_count(&[short(300), short(300)]), 1);
        assert_eq!(preview_count(&[short(50), short(50), short(300)]), 3);
        assert_eq!(preview_count(&[short(10), short(10), short(10), short(10)]), 3, "never more than three");
        assert_eq!(preview_count(&[short(10)]), 1);
        assert_eq!(preview_count(&[short(10), short(250)]), 2);
    }

    #[test]
    fn markers_become_pauses_or_are_dropped() {
        let allowed = vec!["laugh".to_string()];
        let (pieces, skipped) = parse_markers("Hi [pause] there [Pause 1.5s] you [laugh] [whisper] end [note", &allowed);
        let shape: Vec<String> = pieces
            .iter()
            .map(|p| match p {
                Piece::Text(t) => format!("T({t})"),
                Piece::Pause(ms) => format!("P({ms})"),
            })
            .collect();
        assert_eq!(shape, ["T(Hi )", "P(600)", "T( there )", "P(1500)", "T( you [laugh] end [note)"]);
        assert_eq!(skipped, 1);
        assert_eq!(parse_duration_ms("800ms"), 800);
        assert_eq!(parse_duration_ms("2"), 2000);
        assert_eq!(parse_duration_ms("junk"), SHORT_PAUSE_MS);
        let (p, _) = parse_markers("[pause 99s]", &[]);
        assert!(matches!(p[0], Piece::Pause(MAX_PAUSE_MS)));
    }

    #[test]
    fn expressions_are_spoken_unless_the_model_has_the_tag() {
        let text = |s: &str, allowed: &[String]| match &parse_markers(s, allowed).0[..] {
            [Piece::Text(t)] => t.clone(),
            other => panic!("{} pieces", other.len()),
        };
        assert_eq!(text("So funny.[laughs] Anyway.", &[]), "So funny. Ha ha ha! Anyway.");
        assert_eq!(text("[Clears throat] Hello.", &[]), "Ahem. Hello.");
        assert_eq!(text("Wait [clear-throat] what", &[]), "Wait Ahem. what");
        assert_eq!(text("Oh [gasp] no", &[]), "Oh Oh! no");
        // The model's own tag is passed through instead.
        assert_eq!(text("So funny.[laugh]", &["laugh".to_string()]), "So funny.[laugh]");
        assert_eq!(parse_markers("[laughing]", &[]).1, 1, "unknown forms are dropped");
        assert_eq!(expression("sighs").map(|e| e.0), Some("sigh"));
        assert_eq!(expression("clears throat").map(|e| e.0), Some("clear throat"));

        let feats = expression_features(&model_features("kokoro-82m-v1.0"));
        assert!(feats.iter().all(|f| f.approximate && f.source == "app"));
        assert!(feats.iter().any(|f| f.example == "[clear throat]"));
    }

    #[test]
    fn writes_and_joins_wav_files() {
        let d = std::env::temp_dir().join(format!("sovirae-studio-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let a = d.join("a.wav");
        let b = d.join("b.wav");
        write_wav(&a, &vec![0.5; 2400]).unwrap();
        write_wav(&b, &vec![-0.5; 2400]).unwrap();
        let out = d.join("out.wav");
        write_joined(&[a, b], &out).unwrap();
        let r = hound::WavReader::open(&out).unwrap();
        assert_eq!(r.spec().sample_rate, SAMPLE_RATE);
        assert_eq!(r.len() as u64, 2400 + PARAGRAPH_GAP_MS * SAMPLE_RATE as u64 / 1000 + 2400);
        let _ = std::fs::remove_dir_all(&d);
    }
}
