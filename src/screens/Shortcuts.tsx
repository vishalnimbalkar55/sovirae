import { useEffect, useRef, useState } from "react";
import { api } from "../lib/api";
import { keyParts } from "../lib/format";
import { Alert } from "../lib/icons";
import type { ShortcutStatus } from "../lib/types";
import type { AppModel } from "../App";

const LABELS: Record<string, string> = {
  speakClipboard: "Speak clipboard",
  readSelection: "Read selected text",
  playPause: "Play/pause",
  skipBack: "Skip back 10 seconds",
  skipForward: "Skip forward 10 seconds",
  speedUp: "Speed up",
  slowDown: "Slow down",
  stop: "Stop and hide player",
};

const MODIFIER_CODES = new Set([
  "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight",
]);

/** Builds a binding from a keydown, using physical key codes so Option/AltGr
 *  characters do not change the result. */
function bindingFrom(e: KeyboardEvent): string | null {
  if (MODIFIER_CODES.has(e.code)) return null;
  const mods: string[] = [];
  if (e.metaKey) mods.push("Command");
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  const key = e.code.replace(/^Key([A-Z])$/, "$1").replace(/^Digit(\d)$/, "$1");
  return [...mods, key].join("+");
}

export default function ShortcutsScreen({ app }: { app: AppModel }) {
  const [rows, setRows] = useState<ShortcutStatus[]>([]);
  const [recording, setRecording] = useState<string | null>(null);
  const [error, setError] = useState<{ action: string; message: string } | null>(null);
  const recordRef = useRef<string | null>(null);

  useEffect(() => {
    api.getShortcuts().then(setRows);
  }, [app.settings?.hotkeysPaused, app.snapshot?.status]);

  const stopRecording = async () => {
    recordRef.current = null;
    setRecording(null);
    await api.suspendShortcuts(false);
    setRows(await api.getShortcuts());
  };

  useEffect(() => {
    if (!recording) return;
    const onKey = async (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape" && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey) {
        await stopRecording();
        return;
      }
      const binding = bindingFrom(e);
      if (!binding) return;
      const action = recordRef.current!;
      try {
        setRows(await api.setShortcut(action, binding));
        setError(null);
      } catch (err) {
        setError({ action, message: String(err) });
      }
      await stopRecording();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording]);

  const record = async (action: string) => {
    setError(null);
    // Release global bindings so the recorder can hear the chord.
    await api.suspendShortcuts(true);
    recordRef.current = action;
    setRecording(action);
  };

  const reset = async (action: string) => {
    setError(null);
    try {
      setRows(await api.setShortcut(action, null));
    } catch (err) {
      setError({ action, message: String(err) });
    }
  };

  const paused = app.settings?.hotkeysPaused;

  return (
    <div className="screen" aria-labelledby="shortcuts-title">
      <header className="screen-head">
        <h1 id="shortcuts-title">Shortcuts</h1>
        <p>
          Read shortcuts work in every app. Playback shortcuts switch on only while something is being read, so the
          same keys keep their usual meaning the rest of the time.
        </p>
      </header>

      {paused && (
        <div className="banner" role="status">
          <Alert />
          <span>Global shortcuts are paused.</span>
          <button className="btn small" onClick={() => app.update({ hotkeysPaused: false })}>Resume</button>
        </div>
      )}

      <div className="group">
        {rows.map((r) => {
          const isRec = recording === r.action;
          const state = paused
            ? "Paused"
            : r.error
              ? null
              : r.registered
                ? "Active"
                : r.playbackOnly
                  ? "During reading"
                  : "Not registered";
          return (
            <div className="row shortcut-row" key={r.action}>
              <div className="row-label">
                <span>{LABELS[r.action] ?? r.action}</span>
                {r.conflict && <span className="hint">Also used by: {r.conflict}</span>}
                {r.error && <span className="hint error-text">{r.error}</span>}
                {error?.action === r.action && <span className="hint error-text">{error.message}</span>}
              </div>
              <div className="row-control">
                {isRec ? (
                  <span className="recording" aria-live="assertive">Press the new shortcut… Esc to cancel</span>
                ) : (
                  <span className="keys" aria-label={r.binding}>
                    {keyParts(r.binding).map((k, i) => (
                      <kbd key={i}>{k}</kbd>
                    ))}
                  </span>
                )}
                {state && <span className={`pill ${state === "Active" ? "ok" : ""}`}>{state === "Active" && <span className="led" />}{state}</span>}
                {isRec ? (
                  <button className="btn small" onClick={stopRecording}>Cancel</button>
                ) : (
                  <button className="btn small" onClick={() => record(r.action)} disabled={!!recording}>Record</button>
                )}
                <button className="btn ghost small" onClick={() => reset(r.action)} disabled={!!recording}>Reset</button>
              </div>
            </div>
          );
        })}
      </div>
      <p className="footnote">
        Press Esc while the player has focus to stop reading. Sovirae can only report conflicts the system tells it
        about; it cannot see every shortcut other apps use.
      </p>
    </div>
  );
}
