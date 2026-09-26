export type Status =
  | "idle"
  | "loadingVoice"
  | "preparing"
  | "buffering"
  | "playing"
  | "paused"
  | "recovering"
  | "completed"
  | "error";

export interface SourceReference {
  kind: "manual" | "clipboard" | "selection" | "chrome";
  displayName: string | null;
}

export interface Snapshot {
  sessionId: number;
  status: Status;
  source: SourceReference | null;
  segmentId: number | null;
  sentenceId: number | null;
  positionMs: number;
  durationMs: number;
  durationIsFinal: boolean;
  rate: number;
  volume: number;
  voice: string | null;
  device: string;
  message: string | null;
}

export interface Notice {
  code: string;
  message: string;
  action: "readFirstPart" | null;
}

export interface DocumentView {
  sessionId: number;
  text: string;
  /** [segment id, sentence id, start byte, end byte, paragraph start] */
  segments: [number, number, number, number, boolean][];
}

export interface Envelope {
  sessionId: number;
  segment: number;
  durationMs: number;
  levels: number[];
}

export interface Voice {
  id: string;
  name: string;
  language: string;
  gender: "female" | "male" | null;
  /** Owning model: a catalog model ID, or "system". */
  model: string;
  engine: string;
  recommended: boolean;
  approximatePronunciation: boolean;
}

/** What the line under the player controls shows. */
export type PlayerLine = "wave" | "ticker" | "steps" | "meter" | "breathing";

export type ResourceProfile = "eco" | "balanced" | "performance";

/** Where downloaded voices run. System voices are always run by the OS. */
export type Processor = "cpu" | "gpu";

export interface ProcessorStatus {
  gpuAvailable: boolean;
  /** Why the GPU was requested but the CPU is in use. */
  fallback: string | null;
}

export interface Settings {
  theme: "system" | "light" | "dark";
  readingFontSize: number;
  readingFont: "serif" | "sans";
  rate: number;
  volume: number;
  voice: string | null;
  resourceProfile: ResourceProfile;
  processor: Processor;
  startAtLogin: boolean;
  keepRunning: boolean;
  startMinimized: boolean;
  hotkeysPaused: boolean;
  sentenceSnap: boolean;
  followReading: boolean;
  playerLine: PlayerLine;
  playerTopmost: boolean;
  chromeBridge: boolean;
  pairedExtensions: string[];
  matchPageLanguage: boolean;
}

export interface BridgeStatus {
  enabled: boolean;
  listening: boolean;
  extensionId: string;
  extensionFolder: string | null;
  hostInstalled: boolean;
  browsers: { name: string; registered: boolean }[];
  paired: string[];
  connected: number;
  lastConnectionSecs: number | null;
  lastExtension: string | null;
}

export interface ShortcutStatus {
  action: string;
  binding: string;
  playbackOnly: boolean;
  registered: boolean;
  error: string | null;
  conflict: string | null;
}

export interface InitialState {
  settings: Settings;
  snapshot: Snapshot;
  platform: string;
  engine: string;
}

export const ACTIVE: Status[] = ["loadingVoice", "preparing", "buffering", "playing", "paused", "recovering"];

export interface ModelArtifact {
  id: string;
  label: string;
  description: string;
  bytes: number;
}

export interface ModelView {
  id: string;
  name: string;
  description: string;
  builtIn: boolean;
  voicePrefix: string;
  license: string | null;
  licenseUrl: string | null;
  cardUrl: string | null;
  artifacts: ModelArtifact[];
  voicesBytes: number;
  voiceCount: number;
  voicesMissing: number;
  voicesMissingBytes: number;
  installedArtifact: string | null;
  downloading: string | null;
  progress: { doneBytes: number; totalBytes: number; verifying: boolean } | null;
  bytesPerSecond: number | null;
  secondsLeft: number | null;
  error: string | null;
  missing: string[];
}
