import { useEffect, useMemo, useState } from "react";
import { api, on } from "../lib/api";
import { displayVoiceName, languageName, megabytes, speedLabel, timeLeftLabel } from "../lib/format";
import { Check, Download, Play } from "../lib/icons";
import type { ModelView, Voice } from "../lib/types";
import { Section } from "../lib/ui";
import type { AppModel } from "../App";

type Gender = "any" | "female" | "male" | "other";

/** Voices with no stated gender are grouped as "other", never dropped. */
const genderOf = (v: Voice): Exclude<Gender, "any"> => v.gender ?? "other";
const GENDER_LABEL = { female: "Female", male: "Male", other: "Other" } as const;

const voiceOption = (v: Voice) =>
  `${displayVoiceName(v.name)}${v.approximatePronunciation ? " (approximate pronunciation)" : ""}`;

/** Picks the best voice in `pool`, keeping language and gender if possible. */
function pickVoice(pool: Voice[], language?: string, gender?: Gender): Voice | undefined {
  const byLang = language ? pool.filter((v) => languageName(v.language) === language) : pool;
  const langPool = byLang.length ? byLang : pool;
  const byGender = gender && gender !== "any" ? langPool.filter((v) => genderOf(v) === gender) : langPool;
  const final = byGender.length ? byGender : langPool;
  return final.find((v) => v.recommended) ?? final[0];
}

export default function VoicesScreen({ app }: { app: AppModel }) {
  const [voices, setVoices] = useState<Voice[] | null>(null);
  const [models, setModels] = useState<ModelView[]>([]);
  const [error, setError] = useState<string | null>(null);
  const settings = app.settings!;
  // Refetch voices whenever a model is installed, removed, or gains voices.
  const installedKey = models.map((m) => `${m.id}=${m.installedArtifact}/${m.voicesMissing}`).join();

  useEffect(() => {
    api.listModels().then(setModels).catch((e) => setError(String(e)));
    return on("model", (m) => setModels((all) => all.map((x) => (x.id === m.id ? m : x))));
  }, []);

  useEffect(() => {
    api.listVoices().then(setVoices).catch((e) => setError(String(e)));
  }, [installedKey]);

  // The chosen voice, or the default the engine would use.
  const all = voices ?? [];
  const current =
    all.find((v) => v.id === settings.voice) ?? all.find((v) => v.model === "system" && v.recommended) ?? all[0];
  const activeModel = current?.model ?? "system";
  const pool = all.filter((v) => v.model === activeModel);

  // Filters narrow the Voice list; "any" shows every voice of the model.
  const [language, setLanguage] = useState<string>("any");
  const [gender, setGender] = useState<Gender>("any");

  // Filter by readable language name, so unnamed tags share one "Other".
  const languages = useMemo(
    () => [...new Set(pool.map((v) => languageName(v.language)))].sort((a, b) =>
      a === "Other" ? 1 : b === "Other" ? -1 : a.localeCompare(b)),
    [pool],
  );
  useEffect(() => {
    // A language the newly used model lacks falls back to all languages.
    if (language !== "any" && voices && !languages.includes(language)) setLanguage("any");
  }, [activeModel, languages.join()]);

  const inLanguage = pool.filter((v) => language === "any" || languageName(v.language) === language);
  const countGender = (g: Gender) => inLanguage.filter((v) => g === "any" || genderOf(v) === g).length;
  const genders = (["female", "male", "other"] as const).filter((g) => countGender(g) > 0);
  const choices = inLanguage.filter((v) => gender === "any" || genderOf(v) === gender);

  // Voice options grouped by language, then gender.
  const groups = useMemo(() => {
    const map = new Map<string, Voice[]>();
    for (const v of [...choices].sort((a, b) =>
      languageName(a.language).localeCompare(languageName(b.language)) ||
      genderOf(a).localeCompare(genderOf(b)) ||
      a.name.localeCompare(b.name))) {
      const key = `${languageName(v.language)}, ${GENDER_LABEL[genderOf(v)].toLowerCase()}`;
      map.set(key, [...(map.get(key) ?? []), v]);
    }
    return [...map.entries()];
  }, [choices]);

  const select = (v: Voice | undefined) => v && app.update({ voice: v.id });

  const useModel = (modelId: string) => {
    const target = all.filter((v) => v.model === modelId);
    const lang = language === "any" ? (current ? languageName(current.language) : undefined) : language;
    select(pickVoice(target, lang, gender) ?? target[0]);
  };

  // Changing a filter keeps the current voice if it still matches.
  const applyFilters = (lang: string, g: Gender) => {
    const matches = (v: Voice) => (lang === "any" || languageName(v.language) === lang) && (g === "any" || genderOf(v) === g);
    if (current && !matches(current)) select(pickVoice(pool.filter(matches), undefined, g));
  };
  const plural = (n: number) => `${n} ${n === 1 ? "voice" : "voices"}`;

  const activeName = models.find((m) => m.id === activeModel)?.name ?? "System voices";

  return (
    <div className="screen" aria-labelledby="voices-title">
      <header className="screen-head">
        <h1 id="voices-title">Voices</h1>
        <p>Choose how Sovirae sounds. The voice is used from the next reading, and every voice runs on this device.</p>
      </header>

      {error && <p className="field-error" role="alert">{error}</p>}

      <Section title="Voice">
        <div className="voice-setup">
          <div className="voice-setup-model">
            <span className="muted">Using</span>
            <span className="pill accent">{activeName}</span>
          </div>
          <div className="voice-setup-fields">
            <label className="field">
              <span>Language</span>
              <select
                className="select"
                aria-label="Language"
                value={language}
                onChange={(e) => {
                  setLanguage(e.target.value);
                  applyFilters(e.target.value, gender);
                }}
              >
                <option value="any">All languages ({pool.length})</option>
                {languages.map((l) => (
                  <option key={l} value={l}>{l} ({pool.filter((v) => languageName(v.language) === l).length})</option>
                ))}
              </select>
            </label>
            <label className="field">
              <span>Gender</span>
              <select
                className="select"
                aria-label="Gender"
                value={gender}
                onChange={(e) => {
                  const g = e.target.value as Gender;
                  setGender(g);
                  applyFilters(language, g);
                }}
              >
                <option value="any">Any ({countGender("any")})</option>
                {genders.map((g) => (
                  <option key={g} value={g}>{GENDER_LABEL[g]} ({countGender(g)})</option>
                ))}
              </select>
            </label>
            <label className="field grow">
              <span>Voice</span>
              <select
                className="select"
                aria-label="Voice"
                value={current?.id ?? ""}
                disabled={choices.length === 0}
                onChange={(e) => select(pool.find((v) => v.id === e.target.value))}
              >
                {groups.length > 1
                  ? groups.map(([label, vs]) => (
                      <optgroup key={label} label={label}>
                        {vs.map((v) => <option key={v.id} value={v.id}>{voiceOption(v)}</option>)}
                      </optgroup>
                    ))
                  : choices.map((v) => <option key={v.id} value={v.id}>{voiceOption(v)}</option>)}
              </select>
            </label>
            <button
              className="btn preview-btn"
              disabled={!current}
              onClick={() => current && api.previewVoice(current.id)}
            >
              <Play /> Preview
            </button>
          </div>
          <p className="voice-count" aria-live="polite">
            {choices.length === pool.length
              ? `${plural(pool.length)} in ${activeName}.`
              : `Showing ${choices.length} of ${pool.length} ${activeName} voices.`}
            {choices.length !== pool.length && (
              <button className="link" onClick={() => { setLanguage("any"); setGender("any"); }}>Show all</button>
            )}
          </p>
        </div>
      </Section>

      <Section title="Models" description="Download a model once, then use it offline. Only one model is used at a time.">
        <ul className="model-list">
          {models.map((m) => (
            <ModelRow
              key={m.id}
              model={m}
              active={m.id === activeModel}
              onUse={() => useModel(m.id)}
            />
          ))}
        </ul>
      </Section>
    </div>
  );
}

function ModelRow({ model, active, onUse }: { model: ModelView; active: boolean; onUse: () => void }) {
  const [artifact, setArtifact] = useState(model.artifacts[0]?.id ?? "");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const installed = model.installedArtifact !== null;
  const busy = model.downloading !== null;
  const chosen = model.artifacts.find((a) => a.id === artifact) ?? model.artifacts[0];
  const installedArtifact = model.artifacts.find((a) => a.id === model.installedArtifact);
  const pct = model.progress ? Math.min(100, (model.progress.doneBytes / Math.max(1, model.progress.totalBytes)) * 100) : 0;
  const blocked = model.missing.length > 0;
  const run = (p: Promise<unknown>) => p.catch((e) => setError(String(e)));

  const meta = model.builtIn
    ? `${model.voiceCount} voices`
    : installedArtifact
      ? `${model.voiceCount} voices, ${megabytes(installedArtifact.bytes + model.voicesBytes)} on disk`
      : `${model.voiceCount} voices`;

  return (
    <li className={`model-row ${active ? "active" : ""}`}>
      <div className="model-info">
        <div className="model-title">
          <span className="model-name">{model.name}</span>
          {active && <span className="pill accent"><Check /> In use</span>}
          {!model.builtIn && installed && !active && <span className="pill ok"><span className="led" />Downloaded</span>}
        </div>
        <p className="model-desc">{model.description}</p>
        <p className="model-meta">
          <span>{meta}</span>
          {model.license && model.licenseUrl && (
            <a className="link" href={model.licenseUrl} target="_blank" rel="noreferrer">{model.license} license</a>
          )}
        </p>
        {blocked && (
          <p className="model-warn">
            {model.missing.includes("espeak-ng") ? (
              document.documentElement.dataset.platform === "windows" ? (
                <>
                  Needs espeak-ng for pronunciation. Install it from the{" "}
                  <a className="link" href="https://github.com/espeak-ng/espeak-ng/releases" target="_blank" rel="noreferrer">eSpeak NG releases page</a>
                  {" "}(the <code>.msi</code>), then reopen Sovirae.
                </>
              ) : (
                <>Needs espeak-ng for pronunciation. Install it with <code>brew install espeak-ng</code>, then reopen Sovirae.</>
              )
            ) : (
              <>The voice engine for this model is missing from this build.</>
            )}
          </p>
        )}
        {installed && !busy && model.voicesMissing > 0 && (
          <div className="model-update">
            <span>
              {model.voicesMissing} more {model.voicesMissing === 1 ? "voice is" : "voices are"} available for this model.
            </span>
            <button
              className="btn small primary"
              disabled={blocked}
              onClick={() => { setError(null); run(api.downloadModel(model.id, model.installedArtifact!)); }}
            >
              <Download /> Download {megabytes(model.voicesMissingBytes)}
            </button>
          </div>
        )}
        {busy && (
          <div className="model-progress" aria-live="polite">
            <div className="bar" role="progressbar" aria-label={`Downloading ${model.name}`} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(pct)}>
              <span style={{ width: `${pct}%` }} />
            </div>
            <span className="model-progress-text">
              {model.progress?.verifying
                ? "Verifying…"
                : model.progress
                  ? `${megabytes(model.progress.doneBytes)} of ${megabytes(model.progress.totalBytes)}`
                  : "Starting…"}
            </span>
            {!model.progress?.verifying && model.bytesPerSecond !== null && (
              <span className="model-progress-rate">
                <span>{speedLabel(model.bytesPerSecond)}</span>
                {model.secondsLeft !== null && <span className="muted">{timeLeftLabel(model.secondsLeft)}</span>}
              </span>
            )}
          </div>
        )}
        {(model.error || error) && (
          <p className="field-error" role="alert">{model.error ?? error}</p>
        )}
      </div>

      <div className="model-buttons">
        {model.builtIn ? (
          !active && <button className="btn" onClick={onUse}>Use it</button>
        ) : busy ? (
          <button className="btn" onClick={() => run(api.cancelDownload(model.id))}>Cancel</button>
        ) : installed ? (
          confirmDelete ? (
            <>
              <span className="confirm-text">Delete {megabytes((installedArtifact?.bytes ?? 0) + model.voicesBytes)}?</span>
              <button className="btn danger" onClick={() => { setConfirmDelete(false); run(api.removeModel(model.id)); }}>Delete</button>
              <button className="btn ghost" onClick={() => setConfirmDelete(false)}>Keep</button>
            </>
          ) : (
            <>
              {!active && <button className="btn" disabled={blocked} onClick={onUse}>Use it</button>}
              <button className="btn ghost" onClick={() => setConfirmDelete(true)}>Delete</button>
            </>
          )
        ) : (
          <>
            {model.artifacts.length > 1 && (
              <select
                className="select compact"
                aria-label={`${model.name} version`}
                value={artifact}
                onChange={(e) => setArtifact(e.target.value)}
                title={chosen?.description}
              >
                {model.artifacts.map((a) => (
                  <option key={a.id} value={a.id}>{a.label}, {megabytes(a.bytes + model.voicesBytes)}</option>
                ))}
              </select>
            )}
            <button
              className="btn primary"
              disabled={blocked}
              onClick={() => { setError(null); run(api.downloadModel(model.id, chosen.id)); }}
            >
              <Download /> Download
            </button>
          </>
        )}
      </div>
    </li>
  );
}
