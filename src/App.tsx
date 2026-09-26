import { useEffect, useState } from "react";
import { api, on } from "./lib/api";
import { voiceLabel } from "./lib/format";
import { useAppState, useTheme } from "./lib/useApp";
import { GearIcon, KeyIcon, Mark, Pause, Play, PuzzleIcon, ReadIcon, VoiceIcon } from "./lib/icons";
import { ACTIVE } from "./lib/types";
import ReadScreen from "./screens/Read";
import VoicesScreen from "./screens/Voices";
import ShortcutsScreen from "./screens/Shortcuts";
import ExtensionScreen from "./screens/Extension";
import SettingsScreen from "./screens/Settings";

const SCREENS = [
  { id: "read", label: "Read", Icon: ReadIcon },
  { id: "voices", label: "Voices", Icon: VoiceIcon },
  { id: "shortcuts", label: "Shortcuts", Icon: KeyIcon },
  { id: "extension", label: "Extension", Icon: PuzzleIcon },
  { id: "settings", label: "Settings", Icon: GearIcon },
] as const;
type ScreenId = (typeof SCREENS)[number]["id"];

export default function App() {
  const app = useAppState();
  const [screen, setScreen] = useState<ScreenId>("read");
  useTheme(app.settings?.theme);

  useEffect(
    () =>
      on("navigate", (s) => {
        if (SCREENS.some((x) => x.id === s)) setScreen(s as ScreenId);
      }),
    [],
  );

  const snap = app.snapshot;
  const reading = snap ? ACTIVE.includes(snap.status) : false;
  const paused = snap?.status === "paused";

  return (
    <div className="shell">
      <nav className="sidebar" aria-label="Sections">
        <div className="traffic" data-tauri-drag-region />
        <div className="brand" data-tauri-drag-region>
          <Mark className="brand-mark" />
          <span className="wordmark">Sovirae</span>
        </div>
        <ul>
          {SCREENS.map(({ id, label, Icon }) => (
            <li key={id}>
              <button
                className="nav-item"
                aria-current={screen === id ? "page" : undefined}
                onClick={() => setScreen(id)}
              >
                <Icon />
                <span>{label}</span>
              </button>
            </li>
          ))}
        </ul>

        <div className={`now ${reading ? "live" : ""}`}>
          {reading ? (
            <>
              <div className="now-text">
                <span className="now-title">{paused ? "Paused" : "Reading aloud"}</span>
                <span className="now-sub">{snap?.voice ?? voiceLabel(app.settings?.voice)}</span>
              </div>
              <button
                className="now-btn"
                onClick={() => api.control("toggle")}
                aria-label={paused ? "Resume reading" : "Pause reading"}
              >
                {paused ? <Play /> : <Pause />}
              </button>
            </>
          ) : (
            <div className="now-text">
              <span className="now-title">Voice ready offline</span>
              <span className="now-sub">{voiceLabel(app.settings?.voice)}</span>
            </div>
          )}
        </div>
      </nav>

      <main className="content">
        <div className="titlebar-space" data-tauri-drag-region />
        {!app.settings || !app.snapshot || !app.init ? null : screen === "read" ? (
          <ReadScreen app={app} onChangeVoice={() => setScreen("voices")} />
        ) : screen === "voices" ? (
          <VoicesScreen app={app} />
        ) : screen === "shortcuts" ? (
          <ShortcutsScreen app={app} />
        ) : screen === "extension" ? (
          <ExtensionScreen app={app} />
        ) : (
          <SettingsScreen app={app} />
        )}
      </main>
    </div>
  );
}

export type AppModel = ReturnType<typeof useAppState>;
