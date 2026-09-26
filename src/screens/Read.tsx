import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../lib/api";
import { estimateLabel, keyParts, MAX_TEXT_UTF16, voiceLabel, wordCount } from "../lib/format";
import { Alert, Chevron, Clipboard, Play, VoiceIcon } from "../lib/icons";
import { ACTIVE } from "../lib/types";
import type { AppModel } from "../App";

// The draft lives only while the app is open (spec §4.1).
let draft = "";

export default function ReadScreen({ app, onChangeVoice }: { app: AppModel; onChangeVoice: () => void }) {
  const [text, setText] = useState(draft);
  const [pasteError, setPasteError] = useState<string | null>(null);
  const [tooLong, setTooLong] = useState(false);
  const [clipboardChord, setClipboardChord] = useState<string | null>(null);
  const area = useRef<HTMLTextAreaElement>(null);
  const settings = app.settings!;
  const snapshot = app.snapshot!;

  useEffect(() => {
    draft = text;
  }, [text]);

  // Name of the model behind the chosen voice, for the voice chip.
  const [engine, setEngine] = useState("System voice");
  useEffect(() => {
    api.listVoices().then((vs) => {
      const v = vs.find((x) => x.id === settings.voice) ?? vs.find((x) => x.recommended);
      if (v) setEngine(v.engine);
    }).catch(() => {});
  }, [settings.voice]);

  useEffect(() => {
    api.getShortcuts().then((rows) => setClipboardChord(rows.find((r) => r.action === "speakClipboard")?.binding ?? null));
  }, []);

  const words = useMemo(() => wordCount(text), [text]);
  const empty = text.trim().length === 0;
  const reading = ACTIVE.includes(snapshot.status);
  const over = text.length > MAX_TEXT_UTF16;

  const listen = (truncate = false) => {
    if (empty) return;
    if (over && !truncate) {
      setTooLong(true);
      return;
    }
    setTooLong(false);
    api.speakText(text, truncate);
  };

  const paste = async () => {
    setPasteError(null);
    try {
      setText(await api.readClipboard());
      area.current?.focus();
    } catch (e) {
      setPasteError(String(e));
    }
  };

  const notice = app.notice && !["TEXT_TOO_LONG", "SETTINGS"].includes(app.notice.code) ? app.notice : null;
  const voice = reading && snapshot.voice ? snapshot.voice : voiceLabel(settings.voice);

  return (
    <section className="screen read" aria-labelledby="read-title">
      <header className="screen-head">
        <h1 id="read-title">Read</h1>
        <p>Choose text to read aloud. Speech is generated on this {app.init?.platform === "macos" ? "Mac" : "computer"}.</p>
      </header>

      {notice && (
        <div className="banner" role="alert">
          <Alert />
          <span>{notice.message}</span>
          <button className="btn ghost small" onClick={() => app.setNotice(null)}>Dismiss</button>
        </div>
      )}
      {tooLong && (
        <div className="banner" role="alert">
          <Alert />
          <span>This text is longer than 200,000 characters. Sovirae can read the first part, ending at a sentence.</span>
          <button className="btn small" onClick={() => listen(true)}>Read first part</button>
          <button className="btn ghost small" onClick={() => setTooLong(false)}>Cancel</button>
        </div>
      )}

      <div className={`sheet ${empty ? "is-empty" : ""}`}>
        <label htmlFor="read-text" className="visually-hidden">Text to read aloud</label>
        <textarea
          id="read-text"
          ref={area}
          value={text}
          onChange={(e) => setText(e.target.value)}
          spellCheck={false}
          style={{
            fontSize: settings.readingFontSize,
            fontFamily: settings.readingFont === "serif" ? "var(--font-read-serif)" : "var(--font-read-sans)",
          }}
          onKeyDown={(e) => {
            if ((e.metaKey || e.ctrlKey) && e.key === "Enter") listen();
          }}
        />
        {empty && (
          <div className="sheet-empty">
            <p className="sheet-empty-title">Paste text here to hear it read aloud</p>
            {clipboardChord && (
              <p className="sheet-empty-hint">
                Or copy text in any app and press
                <span className="keys-inline">
                  {keyParts(clipboardChord).map((k, i) => <kbd key={i}>{k}</kbd>)}
                </span>
              </p>
            )}
            <button className="btn" onClick={paste}>
              <Clipboard /> Paste from clipboard
            </button>
            {pasteError && <p className="field-error" role="status">{pasteError}</p>}
          </div>
        )}

        <div className="sheet-bar">
          <div className="counts" aria-live="polite">
            {!empty && (
              <>
                <span className="count-strong">{words.toLocaleString()} {words === 1 ? "word" : "words"}</span>
                <span>{estimateLabel(words, settings.rate)} at {settings.rate.toFixed(1)}×</span>
              </>
            )}
          </div>
          <div className="bar-actions">
            {!empty && (
              <>
                <button className="btn ghost" onClick={() => setText("")}>Clear</button>
                <button className="btn" onClick={paste}>
                  <Clipboard /> Paste
                </button>
              </>
            )}
            <button className="btn primary" onClick={() => listen()} disabled={empty} title="⌘ Return">
              <Play /> {reading ? "Read this instead" : "Listen"}
            </button>
          </div>
        </div>
      </div>
      {!empty && pasteError && <p className="field-error" role="status">{pasteError}</p>}

      <button className="voice-chip" onClick={onChangeVoice}>
        <span className="voice-chip-icon"><VoiceIcon /></span>
        <span className="voice-chip-text">
          <span className="voice-chip-name">{voice}</span>
          <span className="voice-chip-meta">{engine} on {snapshot.device}</span>
        </span>
        <span className="voice-chip-action">Change voice <Chevron /></span>
      </button>
    </section>
  );
}
