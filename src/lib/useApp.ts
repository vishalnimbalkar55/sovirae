import { useEffect, useState } from "react";
import { api, on } from "./api";
import type { InitialState, Notice, Settings, Snapshot } from "./types";

/** Live settings + playback snapshot shared by the main window's screens. */
export function useAppState() {
  const [init, setInit] = useState<InitialState | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);

  useEffect(() => {
    api.getState().then((s) => {
      setInit(s);
      setSettings(s.settings);
      setSnapshot(s.snapshot);
    });
    const offs = [
      on("playback", setSnapshot),
      on("settings", setSettings),
      on("notice", setNotice),
    ];
    return () => offs.forEach((off) => off());
  }, []);

  const update = async (patch: Partial<Settings>) => {
    setSettings((s) => (s ? { ...s, ...patch } : s));
    try {
      setSettings(await api.updateSettings(patch));
    } catch (e) {
      setNotice({ code: "SETTINGS", message: String(e), action: null });
      api.getState().then((s) => setSettings(s.settings));
    }
  };

  return { init, settings, snapshot, notice, setNotice, update };
}

export function useTheme(theme: Settings["theme"] | undefined) {
  useEffect(() => {
    const root = document.documentElement;
    if (!theme || theme === "system") root.removeAttribute("data-theme");
    else root.setAttribute("data-theme", theme);
  }, [theme]);
}
