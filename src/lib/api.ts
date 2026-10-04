import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DocumentView,
  Envelope,
  InitialState,
  ModelView,
  BridgeStatus,
  ProcessorStatus,
  Notice,
  Settings,
  ShortcutStatus,
  Snapshot,
  StudioFeature,
  StudioProject,
  StudioSummary,
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
  processorStatus: () => invoke<ProcessorStatus>("processor_status"),
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
  bridgeStatus: () => invoke<BridgeStatus>("bridge_status"),
  bridgeTest: () => invoke<{ ok: boolean; message: string }>("bridge_test"),
  bridgeRevoke: (id: string) => invoke<void>("bridge_revoke", { id }),
  bridgeAllowAgain: (id: string) => invoke<void>("bridge_allow_again", { id }),
  openExtensionFolder: () => invoke<void>("open_extension_folder"),
  studioList: () => invoke<StudioSummary[]>("studio_list"),
  studioCreate: (name: string) => invoke<StudioProject>("studio_create", { name }),
  studioGet: (id: string) => invoke<StudioProject>("studio_get", { id }),
  studioRename: (id: string, name: string) => invoke<StudioProject>("studio_rename", { id, name }),
  studioDelete: (id: string) => invoke<void>("studio_delete", { id }),
  studioSaveScript: (id: string, text: string, newVersion: boolean) =>
    invoke<StudioProject>("studio_save_script", { id, text, newVersion }),
  studioSelectVersion: (id: string, version: number) => invoke<StudioProject>("studio_select_version", { id, version }),
  studioDeleteVersion: (id: string, version: number) => invoke<StudioProject>("studio_delete_version", { id, version }),
  studioSetVoice: (id: string, voice: string) => invoke<StudioProject>("studio_set_voice", { id, voice }),
  studioFeatures: (voice: string | null) => invoke<StudioFeature[]>("studio_features", { voice }),
  studioGenerate: (id: string, scope: "preview" | "rest" | "all") => invoke<StudioProject>("studio_generate", { id, scope }),
  studioCancel: (id: string) => invoke<void>("studio_cancel", { id }),
  studioAudio: (id: string, paragraph: number) => invoke<ArrayBuffer>("studio_audio", { id, paragraph }),
  studioExport: (id: string, scope: "preview" | "all") => invoke<string | null>("studio_export", { id, scope }),
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
  bridge: BridgeStatus;
  studio: StudioProject;
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
