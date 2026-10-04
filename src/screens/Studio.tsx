import { useEffect, useMemo, useRef, useState } from "react";
import { api, on } from "../lib/api";
import { confirmLeave, setLeaveGuard } from "../lib/navGuard";
import { paragraphsOf } from "../lib/studioText";
import { displayVoiceName, formatTime, languageName } from "../lib/format";
import { Check, Chevron, Copy, Download, Pause, Play, Plus, Stop, Trash } from "../lib/icons";
import type { StudioFeature, StudioProject, StudioSummary, Voice } from "../lib/types";
import type { AppModel } from "../App";

// The open project survives switching screens.
let openId: string | null = null;

const MAX_SCRIPT = 200_000;

/** Prompt for ChatGPT or Claude: enhance a script using only markers this voice honours. */
export function buildPrompt(features: StudioFeature[], voiceName: string, script: string): string {
  const markers = features
    .filter((f) => f.source === "model" || f.id.startsWith("pause") || f.approximate)
    .map((f) => `- ${f.example} — ${f.approximate ? `${f.label.toLowerCase()} (approximate: ${f.description.replace(/^Approximate: /, "").replace(/\.$/, "")})` : f.description}`)
    .join("\n");
  const cues = features
    .filter((f) => f.source === "app" && !f.id.startsWith("pause") && !f.approximate)
    .map((f) => `- ${f.label}: ${f.description} (${f.example})`)
    .join("\n");
  return [
    `You are preparing a script to be read aloud by a text-to-speech voice (${voiceName}).`,
    "Rewrite the script below for spoken delivery: keep every fact and the author's meaning, make sentences easy to say aloud, spell out abbreviations and awkward numbers, and split long passages into short paragraphs separated by one blank line.",
    "",
    "You may insert ONLY these markers, written exactly as shown:",
    markers || "- (none)",
    "",
    "The voice also responds to punctuation:",
    cues,
    "",
    "Use expression markers sparingly, only where a listener would expect them. Do not add any other bracketed tags, stage directions, sound effects, or emojis. Do not add a title or commentary. Output only the finished script.",
    "",
    "SCRIPT:",
    script.trim() || "(paste your script here)",
  ].join("\n");
}

const dateLabel = (secs: number) =>
  new Date(secs * 1000).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });

export default function StudioScreen(_: { app: AppModel }) {
  const [list, setList] = useState<StudioSummary[] | null>(null);
  const [project, setProject] = useState<StudioProject | null>(null);
  const [voices, setVoices] = useState<Voice[]>([]);
  const [features, setFeatures] = useState<StudioFeature[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [script, setScript] = useState("");
  const [saved, setSaved] = useState(true);
  const [showPrompt, setShowPrompt] = useState(false);
  const [copied, setCopied] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  // Set while the "save or discard?" dialog waits for an answer.
  const [leave, setLeave] = useState<{ resolve: (ok: boolean) => void } | null>(null);
  const player = usePlayer(project?.id ?? null);

  const refreshList = () => api.studioList().then(setList).catch((e) => setError(String(e)));

  useEffect(() => {
    refreshList();
    api.listVoices().then(setVoices).catch((e) => setError(String(e)));
    if (openId) open(openId);
    const off = on("studio", (p) => {
      setProject((cur) => (cur && cur.id === p.id ? p : cur));
      refreshList();
    });
    return () => {
      off();
      openId = null;
      setLeaveGuard(null);
    };
  }, []);

  const current = project?.versions.find((v) => v.id === project.current) ?? null;
  // The editor follows the stored script when the project or iteration changes.
  useEffect(() => {
    setScript(current?.text ?? "");
    setSaved(true);
  }, [project?.id, current?.id]);

  useEffect(() => {
    if (project) api.studioFeatures(project.voice).then(setFeatures).catch(() => setFeatures([]));
  }, [project?.id, project?.voice]);

  const open = (id: string) => {
    openId = id;
    setError(null);
    setNotice(null);
    api.studioGet(id).then(setProject).catch((e) => {
      setError(String(e));
      openId = null;
      setProject(null);
    });
  };

  const back = () => {
    openId = null;
    setProject(null);
    setError(null);
    setNotice(null);
    refreshList();
  };

  const run = async (f: () => Promise<unknown>) => {
    setError(null);
    try {
      await f();
    } catch (e) {
      setError(String(e));
    }
  };

  const create = () =>
    run(async () => {
      const p = await api.studioCreate(`Project ${(list?.length ?? 0) + 1}`);
      openId = p.id;
      setProject(p);
      refreshList();
    });


  const dirty = !saved && !!current && script !== current.text;

  /** Saves the editor into the current iteration. */
  const save = async () => {
    if (!project) return;
    const p = await api.studioSaveScript(project.id, script, false);
    setProject(p);
    setSaved(true);
  };

  // While the script has unsaved changes, leaving (another screen, the
  // project list, another iteration) goes through the save-or-discard dialog.
  useEffect(() => {
    if (!dirty) {
      setLeaveGuard(null);
      return;
    }
    setLeaveGuard(() => new Promise<boolean>((resolve) => setLeave({ resolve })));
    return () => setLeaveGuard(null);
  }, [dirty]);

  const answerLeave = async (choice: "save" | "discard" | "cancel") => {
    const pending = leave;
    setLeave(null);
    if (!pending) return;
    if (choice === "cancel") return pending.resolve(false);
    if (choice === "save") {
      try {
        await save();
      } catch (e) {
        setError(String(e));
        return pending.resolve(false);
      }
    }
    pending.resolve(true);
  };

  // Cmd/Ctrl+S saves while a project is open.
  useEffect(() => {
    if (!project) return;
    const onKey = (e: globalThis.KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        if (dirty) run(save);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [project?.id, dirty, script]);

  const newIteration = () => project && run(async () => setProject(await api.studioSaveScript(project.id, script, true)));

  const deleteProject = (id: string) =>
    run(async () => {
      await api.studioDelete(id);
      setConfirmDelete(null);
      if (project?.id === id) {
        openId = null;
        setProject(null);
      }
      refreshList();
    });

  const voiceOf = (id: string | null) => voices.find((v) => v.id === id);
  const voice = voiceOf(project?.voice ?? null);
  const models = useMemo(() => {
    const seen = new Map<string, string>();
    for (const v of voices) if (!seen.has(v.model)) seen.set(v.model, v.engine);
    return [...seen.entries()];
  }, [voices]);
  const modelId = voice?.model ?? models[0]?.[0] ?? "system";
  const modelVoices = voices.filter((v) => v.model === modelId);
  const chooseVoice = (id: string) => project && run(async () => setProject(await api.studioSetVoice(project.id, id)));
  const chooseModel = (m: string) => {
    const pool = voices.filter((v) => v.model === m);
    const pick = pool.find((v) => v.recommended) ?? pool[0];
    if (pick) chooseVoice(pick.id);
  };

  const g = project?.generated ?? null;
  // Which paragraphs of the editor differ from the generated ones.
  const scriptParagraphs = useMemo(() => paragraphsOf(script), [script]);
  const changed = useMemo(() => {
    if (!g) return new Set<number>();
    const set = new Set<number>();
    g.paragraphs.forEach((q, i) => { if (q.text !== scriptParagraphs[i]) set.add(i); });
    return set;
  }, [g, scriptParagraphs]);
  const textChanged = !!g && (changed.size > 0 || scriptParagraphs.length !== g.paragraphs.length);
  const otherSource = !!g && project !== null && (g.version !== project.current || g.voice !== project.voice);
  const previewReady = !!g && g.paragraphs.slice(0, g.previewCount).every((q) => q.status === "ready");
  const allReady = !!g && g.paragraphs.every((q) => q.status === "ready");
  const readyCount = g?.paragraphs.filter((q) => q.status === "ready").length ?? 0;
  const generating = g?.paragraphs.find((q) => q.status === "generating");
  const busy = !!g?.running;

  const generate = (scope: "preview" | "rest" | "all") =>
    project &&
    run(async () => {
      if (dirty) await save();
      player.reset();
      setProject(await api.studioGenerate(project.id, scope));
    });

  const exportAudio = (scope: "preview" | "all") =>
    project &&
    run(async () => {
      const path = await api.studioExport(project.id, scope);
      setNotice(path ? `Saved to ${path}` : null);
    });

  const prompt = buildPrompt(features, voice ? displayVoiceName(voice.name) : "a neural voice", script);
  const copyPrompt = async () => {
    try {
      await navigator.clipboard.writeText(prompt);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      setShowPrompt(true);
    }
  };

  // ── Project list ──────────────────────────────────────────────────
  if (!project) {
    return (
      <div className="screen studio-list" aria-labelledby="studio-title">
        <header className="screen-head studio-list-head">
          <div>
            <h1 id="studio-title">Studio</h1>
            <p>Turn a script into a recording. Pick a voice, generate the first part, accept it, and download the result.</p>
          </div>
          <button className="btn primary" onClick={create}><Plus /> New project</button>
        </header>
        {error && <div className="banner" role="alert"><span>{error}</span></div>}
        {list && list.length === 0 && (
          <div className="studio-welcome">
            <h2>No projects yet</h2>
            <p>A project keeps a script with its iterations, its own voice, and the audio generated from it.</p>
            <button className="btn primary" onClick={create}><Plus /> Create the first project</button>
          </div>
        )}
        <ul className="studio-cards" aria-label="Projects">
          {(list ?? []).map((p) => (
            <li key={p.id} className={`studio-card-wrap ${confirmDelete === p.id ? "confirming" : ""}`}>
              <button className="studio-card" onClick={() => open(p.id)} disabled={confirmDelete === p.id}>
                <span className="studio-card-name">{p.name}</span>
                <span className="studio-card-meta">
                  {p.voice ? displayVoiceName(voiceOf(p.voice)?.name ?? p.voice.split(":").pop() ?? p.voice) : "No voice chosen"}
                </span>
                <span className="studio-card-foot">
                  <span className={`pill ${p.running ? "accent" : p.total && p.ready === p.total ? "ok" : ""}`}>
                    {p.running && <span className="led" />}
                    {p.running ? "Generating" : p.total ? `${p.ready} of ${p.total} paragraphs` : "No audio yet"}
                  </span>
                  <span className="hint">{dateLabel(p.updated)}</span>
                </span>
              </button>
              {confirmDelete === p.id ? (
                <div className="studio-card-confirm" role="alertdialog" aria-label={`Delete ${p.name}?`}>
                  <span className="confirm-text">Delete this project and its audio?</span>
                  <div className="studio-actions">
                    <button className="btn danger small" onClick={() => deleteProject(p.id)}>Delete</button>
                    <button className="btn ghost small" onClick={() => setConfirmDelete(null)}>Keep</button>
                  </div>
                </div>
              ) : (
                <button
                  className="btn ghost small icon studio-card-delete"
                  aria-label={`Delete ${p.name}`}
                  title="Delete project"
                  disabled={p.running}
                  onClick={() => setConfirmDelete(p.id)}
                >
                  <Trash />
                </button>
              )}
            </li>
          ))}
        </ul>
      </div>
    );
  }

  // ── Project view: fixed left column, scrolling main area ──────────
  return (
    <div className="studio-project" aria-labelledby="studio-title">
      <aside className="studio-side">
        <button className="btn ghost small studio-back" onClick={() => confirmLeave().then((ok) => ok && back())}><Chevron className="flip" /> All projects</button>

        <div className="studio-block">
          <h2>Voice for this project</h2>
          <label className="field">
            <span>Model</span>
            <select className="select compact" value={modelId} onChange={(e) => chooseModel(e.target.value)} disabled={busy}>
              {models.map(([id, name]) => <option key={id} value={id}>{name}</option>)}
            </select>
          </label>
          <label className="field">
            <span>Voice</span>
            <select className="select compact" value={project.voice ?? ""} onChange={(e) => chooseVoice(e.target.value)} disabled={busy}>
              {!project.voice && <option value="">Choose a voice…</option>}
              {modelVoices.map((v) => (
                <option key={v.id} value={v.id}>{displayVoiceName(v.name)} · {languageName(v.language)}</option>
              ))}
            </select>
          </label>
          {voice && (
            <button className="btn ghost small" onClick={() => api.previewVoice(voice.id)}><Play /> Preview voice</button>
          )}
        </div>

        <div className="studio-block studio-block-features">
          <h2>What this voice supports</h2>
          <div className="studio-features-scroll">
            <ul className="studio-features">
              {features.map((f) => (
                <li key={f.id}>
                  <code>{f.example}</code>
                  <span className="studio-feature-text">
                    <span className="studio-feature-label">
                      {f.label}
                      {f.source === "model" && <span className="pill accent">model</span>}
                      {f.approximate && <span className="pill">approximate</span>}
                    </span>
                    <span className="hint">{f.description}</span>
                  </span>
                </li>
              ))}
            </ul>
            {!features.some((f) => f.source === "model") && (
              <p className="hint studio-features-note">This voice reads plain text, so expressions are approximate: it says a sound such as “Ha ha ha!” in its own tone. Markers it does not know are left out of the audio.</p>
            )}
          </div>
        </div>

        <div className="studio-block">
          <h2>Enhance with an assistant</h2>
          <p className="hint">Copy a prompt that tells ChatGPT or Claude which markers this voice supports. Paste the improved script back as a new iteration.</p>
          <div className="studio-actions">
            <button className="btn small" onClick={copyPrompt}>{copied ? <Check /> : <Copy />}{copied ? "Copied" : "Copy prompt"}</button>
            <button className="btn ghost small" onClick={() => setShowPrompt((s) => !s)}>{showPrompt ? "Hide" : "Show"}</button>
          </div>
          {showPrompt && <textarea className="studio-prompt" readOnly value={prompt} aria-label="Prompt" onFocus={(e) => e.target.select()} />}
        </div>
      </aside>

      <section className="studio-main">
        <header className="studio-head">
          <input
            className="studio-name"
            aria-label="Project name"
            defaultValue={project.name}
            key={project.id + project.name}
            onBlur={(e) => e.target.value.trim() !== project.name && run(async () => setProject(await api.studioRename(project.id, e.target.value)))}
            onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
          />
          {confirmDelete === project.id ? (
            <div className="studio-actions">
              <span className="confirm-text">Delete this project and its audio?</span>
              <button className="btn danger small" onClick={() => deleteProject(project.id)}>Delete</button>
              <button className="btn ghost small" onClick={() => setConfirmDelete(null)}>Keep</button>
            </div>
          ) : (
            <button className="btn ghost small" onClick={() => setConfirmDelete(project.id)} disabled={busy}><Trash /> Delete</button>
          )}
        </header>

        {error && <div className="banner" role="alert"><span>{error}</span></div>}
        {notice && <div className="banner ok" role="status"><span>{notice}</span><button className="btn ghost small" onClick={() => setNotice(null)}>Dismiss</button></div>}

        <div className="studio-iterations" role="tablist" aria-label="Script iterations">
          {project.versions.map((v, i) => (
            <button
              key={v.id}
              role="tab"
              aria-selected={v.id === project.current}
              className="studio-tab"
              disabled={busy}
              onClick={() => { if (v.id !== project.current) confirmLeave().then((ok) => { if (ok) run(async () => setProject(await api.studioSelectVersion(project.id, v.id))); }); }}
            >
              Iteration {i + 1}
              {g?.version === v.id && <span className="pill ok" title="Audio was generated from this iteration">audio</span>}
            </button>
          ))}
          <button className="btn ghost small" onClick={newIteration} disabled={busy} title="Copies the current script into a new iteration">
            <Plus /> New iteration
          </button>
          {project.versions.length > 1 && current && (
            <button
              className="btn ghost small"
              disabled={busy}
              onClick={() => confirm("Delete this iteration? Its audio is removed too.") && run(async () => setProject(await api.studioDeleteVersion(project.id, current.id)))}
            >
              <Trash /> Delete iteration
            </button>
          )}
        </div>

        <div className="studio-editor">
          <textarea
            aria-label="Script"
            value={script}
            spellCheck={false}
            disabled={busy}
            placeholder={"Paste or write your script here.\n\nSeparate paragraphs with a blank line. Use [pause], [pause 2s], or [laugh] where you want them."}
            onChange={(e) => {
              if (e.target.value.length <= MAX_SCRIPT) {
                setScript(e.target.value);
                setSaved(false);
              }
            }}
          />
          <div className="studio-editor-foot">
            <span className="hint">
              {script.trim() ? `${scriptParagraphs.length} paragraph${scriptParagraphs.length === 1 ? "" : "s"} · ${script.length.toLocaleString()} characters` : "Empty script"}
              {" · "}
              {dirty ? "Unsaved changes" : "Saved"}
            </span>
            <span className="studio-actions">
              {otherSource && !busy && <span className="pill warn">Audio is from another iteration or voice</span>}
              {!otherSource && textChanged && !busy && (
                <span className="pill warn">
                  {scriptParagraphs.length !== g!.paragraphs.length ? "Paragraphs added or removed" : `${changed.size} paragraph${changed.size > 1 ? "s" : ""} changed`}
                </span>
              )}
              <button className={`btn small ${dirty ? "primary" : ""}`} onClick={() => run(save)} disabled={!dirty || busy} title="⌘S / Ctrl+S">
                Save
              </button>
            </span>
          </div>
        </div>

        <div className="studio-generate">
          {busy ? (
            <>
              <span className="studio-progress">
                <span className="led live" />
                {generating ? `Generating paragraph ${generating.index + 1} of ${g!.paragraphs.length}…` : "Starting…"}
              </span>
              <button className="btn small" onClick={() => api.studioCancel(project.id)}><Stop /> Stop</button>
            </>
          ) : !g || otherSource ? (
            <>
              <button className="btn primary" onClick={() => generate("preview")} disabled={!project.voice || !script.trim()}>
                Generate first part
              </button>
              <button className="btn" onClick={() => generate("all")} disabled={!project.voice || !script.trim()}>
                Generate everything
              </button>
              <span className="hint">The first part is the opening paragraph, or two or three short ones. Listen, then accept to continue.</span>
            </>
          ) : textChanged ? (
            <>
              <button className="btn primary" onClick={() => generate("preview")} disabled={!script.trim()}>
                Regenerate first part
              </button>
              <button className="btn" onClick={() => generate("all")}>Regenerate all</button>
              <span className="hint">The first part is made again from the edited script. Accept it to continue with the rest.</span>
            </>
          ) : !g.accepted ? (
            <>
              <button className="btn primary" onClick={() => generate("rest")} disabled={!previewReady}>
                <Check /> Accept and generate the rest
              </button>
              <button className="btn" onClick={() => generate("all")}>Regenerate first part</button>
              {g.previewCount >= g.paragraphs.length && <span className="hint">The script fits in the first part.</span>}
            </>
          ) : (
            <>
              <span className="studio-progress"><Check /> {allReady ? "All paragraphs generated." : `${readyCount} of ${g.paragraphs.length} paragraphs ready.`}</span>
              {!allReady && <button className="btn primary" onClick={() => generate("rest")}>Retry missing paragraphs</button>}
              <button className="btn" onClick={() => generate("all")}>Regenerate all</button>
            </>
          )}
        </div>
        {g?.error && !busy && <p className="field-error" role="alert">{g.error}</p>}

        {g && (
          <div className="studio-output">
            <div className="studio-output-head">
              <h2>Audio</h2>
              <div className="studio-actions">
                <button className="btn small" onClick={() => player.playAll(g, allReady ? g.paragraphs.length : g.previewCount)} disabled={readyCount === 0}>
                  {player.playingAll ? <Pause /> : <Play />} {player.playingAll ? "Pause" : "Play"}
                </button>
                <button className="btn small" onClick={() => exportAudio("preview")} disabled={!previewReady}><Download /> First part</button>
                <button className="btn small" onClick={() => exportAudio("all")} disabled={readyCount === 0 || busy}>
                  <Download /> {allReady ? "All audio" : "Ready paragraphs"}
                </button>
              </div>
            </div>
            <ol className="studio-paragraphs">
              {g.paragraphs.map((q) => (
                <li key={q.index} className={`studio-paragraph ${q.status} ${player.current === q.index ? "playing" : ""}`}>
                  {q.index === 0 && g.previewCount < g.paragraphs.length && <div className="studio-divider"><span>First part</span></div>}
                  {q.index === g.previewCount && g.previewCount < g.paragraphs.length && (
                    <div className="studio-divider"><span>Rest of the script</span></div>
                  )}
                  <div className="studio-paragraph-row">
                    <button
                      className="studio-play"
                      aria-label={player.current === q.index && player.playing ? `Pause paragraph ${q.index + 1}` : `Play paragraph ${q.index + 1}`}
                      disabled={q.status !== "ready"}
                      onClick={() => player.toggle(q.index)}
                    >
                      {player.current === q.index && player.playing ? <Pause /> : <Play />}
                    </button>
                    <span className="studio-paragraph-text">
                      <span className="studio-paragraph-preview">{q.preview}{q.chars > q.preview.length ? "…" : ""}</span>
                      <span className="hint">
                        {q.status === "ready" ? formatTime(q.durationMs) : q.status === "generating" ? "Generating…" : q.status === "failed" ? q.error ?? "Failed" : "Waiting"}
                        {q.skippedMarkers > 0 && ` · ${q.skippedMarkers} unsupported marker${q.skippedMarkers > 1 ? "s" : ""} left out`}
                      </span>
                    </span>
                    <span className={`pill ${changed.has(q.index) && q.status === "ready" ? "warn" : q.status === "ready" ? "ok" : q.status === "failed" ? "warn" : ""}`}>
                      {q.status === "generating" && <span className="led" />}
                      {changed.has(q.index) && q.status === "ready" ? "Changed" : q.status === "ready" ? "Ready" : q.status === "generating" ? "Working" : q.status === "failed" ? "Failed" : "Queued"}
                    </span>
                  </div>
                </li>
              ))}
            </ol>
          </div>
        )}
      </section>

      {leave && (
        <div className="modal-backdrop" onClick={() => answerLeave("cancel")}>
          <div className="modal" role="dialog" aria-modal="true" aria-labelledby="leave-title" onClick={(e) => e.stopPropagation()}>
            <h2 id="leave-title">Save changes to this script?</h2>
            <p>The script has changes that are not saved to this iteration.</p>
            <div className="studio-actions modal-actions">
              <button className="btn primary" autoFocus onClick={() => answerLeave("save")}>Save</button>
              <button className="btn danger" onClick={() => answerLeave("discard")}>Discard</button>
              <button className="btn ghost" onClick={() => answerLeave("cancel")}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

/** One hidden audio element; paragraph WAVs are fetched on demand and cached per project. */
function usePlayer(projectId: string | null) {
  const audio = useRef<HTMLAudioElement | null>(null);
  const urls = useRef(new Map<string, string>());
  const queue = useRef<number[]>([]);
  const [current, setCurrent] = useState<number | null>(null);
  const [playing, setPlaying] = useState(false);
  const [playingAll, setPlayingAll] = useState(false);

  useEffect(() => {
    const a = new Audio();
    audio.current = a;
    a.onplay = () => setPlaying(true);
    a.onpause = () => setPlaying(false);
    a.onended = () => {
      const next = queue.current.shift();
      if (next === undefined) {
        setPlayingAll(false);
        setCurrent(null);
      } else {
        load(next).catch(() => setPlayingAll(false));
      }
    };
    return () => {
      a.pause();
      for (const u of urls.current.values()) URL.revokeObjectURL(u);
      urls.current.clear();
    };
  }, [projectId]);

  const load = async (i: number) => {
    if (!projectId || !audio.current) return;
    const key = `${projectId}:${i}`;
    let url = urls.current.get(key);
    if (!url) {
      const bytes = await api.studioAudio(projectId, i);
      url = URL.createObjectURL(new Blob([bytes], { type: "audio/wav" }));
      urls.current.set(key, url);
    }
    setCurrent(i);
    audio.current.src = url;
    await audio.current.play();
  };

  const toggle = (i: number) => {
    queue.current = [];
    setPlayingAll(false);
    if (current === i && audio.current) {
      if (audio.current.paused) audio.current.play();
      else audio.current.pause();
      return;
    }
    load(i).catch(() => {});
  };

  const playAll = (g: StudioProject["generated"], upto: number) => {
    if (!g || !audio.current) return;
    if (playingAll) {
      audio.current.pause();
      queue.current = [];
      setPlayingAll(false);
      return;
    }
    const ready = g.paragraphs.filter((q) => q.status === "ready" && q.index < upto).map((q) => q.index);
    if (!ready.length) return;
    queue.current = ready.slice(1);
    setPlayingAll(true);
    load(ready[0]).catch(() => setPlayingAll(false));
  };

  /** Forgets cached audio, e.g. before paragraphs are regenerated. */
  const reset = () => {
    audio.current?.pause();
    queue.current = [];
    setPlayingAll(false);
    setCurrent(null);
    for (const u of urls.current.values()) URL.revokeObjectURL(u);
    urls.current.clear();
  };

  return { current, playing, playingAll, toggle, playAll, reset };
}
