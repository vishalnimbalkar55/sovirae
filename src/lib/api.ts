import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DocumentView,
  Envelope,
  InitialState,
  ModelView,
  Notice,
  Settings,
  ShortcutStatus,
  Snapshot,
  Voice,
} from "./types";

export const api = {
  getState: () => invoke<InitialState>("get_state"),
  speakText: (text: string, truncate = false) => invoke<void>("speak_text", { text, truncate }),
  speakClipboard: (truncate = false) => invoke<void>("speak_clipboard", { truncate }),
  readClipboard: () => invoke<string>("read_clipboard_text"),
  control: (action: string, value?: number) => invoke<void>("control", { action, value }),
  listVoices: () => invoke<Voice[]>("list_voices"),
  previewVoice: (voice: string) => invoke<void>("preview_voice", { voice }),
  getDocument: () => invoke<DocumentView | null>("get_document"),
  updateSettings: (patch: Partial<Settings>) => invoke<Settings>("update_settings", { patch }),
  getShortcuts: () => invoke<ShortcutStatus[]>("get_shortcuts"),
  setShortcut: (action: string, binding: string | null) =>
    invoke<ShortcutStatus[]>("set_shortcut", { action, binding }),
  suspendShortcuts: (suspended: boolean) => invoke<void>("suspend_shortcuts", { suspended }),
  playerExpand: (expanded: boolean) => invoke<void>("player_expand", { expanded }),
  dismissPlayer: () => invoke<void>("dismiss_player"),
  openMain: (screen?: string) => invoke<void>("open_main", { screen }),
  exit: () => invoke<void>("exit_app"),
  listModels: () => invoke<ModelView[]>("list_models"),
  downloadModel: (model: string, artifact: string) => invoke<void>("download_model", { model, artifact }),
  cancelDownload: (model: string) => invoke<void>("cancel_download", { model }),
  removeModel: (model: string) => invoke<void>("remove_model", { model }),
};

type Events = {
  playback: Snapshot;
  notice: Notice;
  document: DocumentView;
  envelope: Envelope;
  settings: Settings;
  navigate: string;
  "player-collapse": null;
  model: ModelView;
};

export function on<K extends keyof Events>(event: K, handler: (payload: Events[K]) => void): () => void {
  let unlisten: UnlistenFn | null = null;
  let cancelled = false;
  listen<Events[K]>(event, (e) => handler(e.payload)).then((u) => {
    if (cancelled) u();
    else unlisten = u;
  });
  return () => {
    cancelled = true;
    unlisten?.();
  };
}
