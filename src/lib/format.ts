export function formatTime(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const mm = h > 0 ? String(m).padStart(2, "0") : String(m).padStart(2, "0");
  return h > 0 ? `${h}:${mm}:${String(s).padStart(2, "0")}` : `${mm}:${String(s).padStart(2, "0")}`;
}

export function wordCount(text: string): number {
  return text.split(/\s+/).filter((w) => /[\p{L}\p{N}]/u.test(w)).length;
}

/** Approximate listening time, spec §10.4: words / (180 × rate) minutes. */
export function estimateLabel(words: number, rate: number): string {
  if (words === 0) return "";
  const minutes = words / (180 * (rate > 0 ? rate : 1));
  if (minutes < 1) return "Under 1 min";
  if (minutes < 60) return `About ${Math.round(minutes)} min`;
  const h = Math.floor(minutes / 60);
  const m = Math.round(minutes % 60);
  return `About ${h} h ${m} min`;
}

export const MAX_TEXT_UTF16 = 200_000;

export function rateLabel(rate: number): string {
  return `${rate.toFixed(1)}×`;
}

const isMac = navigator.platform.toLowerCase().includes("mac");

/** Readable label for a binding, e.g. ⌘⌥R on macOS or Ctrl+Shift+R. */
export function keyParts(binding: string): string[] {
  return binding.split("+").map((p) => {
    const k = p.toLowerCase();
    if (["command", "cmd", "super"].includes(k)) return isMac ? "⌘" : "Super";
    if (["alt", "option"].includes(k)) return isMac ? "⌥" : "Alt";
    if (["control", "ctrl"].includes(k)) return isMac ? "⌃" : "Ctrl";
    if (k === "shift") return isMac ? "⇧" : "Shift";
    if (k === "arrowleft") return "←";
    if (k === "arrowright") return "→";
    if (k === "arrowup") return "↑";
    if (k === "arrowdown") return "↓";
    const bare = p.replace(/^Key/, "").replace(/^Digit/, "");
    return bare.length === 1 ? bare.toUpperCase() : bare;
  });
}

export function sourceLabel(kind: string | undefined, name: string | null | undefined): string {
  if (name) return name;
  switch (kind) {
    case "clipboard":
      return "Clipboard";
    case "selection":
      return "Selection";
    case "chrome":
      return "Chrome";
    default:
      return "Text";
  }
}

/** Friendly name for a stored voice ID, e.g. "kokoro:af_heart" → "Heart". */
export function voiceLabel(id: string | null | undefined): string {
  if (!id) return "System voice";
  const [, voice] = id.includes(":") ? id.split(":", 2) : [null, null];
  if (voice) {
    const name = voice.includes("_") ? voice.split("_").slice(1).join(" ") : voice;
    return name.charAt(0).toUpperCase() + name.slice(1);
  }
  return id.replace(/ \(English \((US|UK)\)\)$/, "");
}

const displayNames = (() => {
  try {
    return new Intl.DisplayNames(["en"], { type: "language" });
  } catch {
    return null;
  }
})();

/** Readable name for any language tag, e.g. "ja-JP" → "Japanese (Japan)".
 *  Tags that cannot be named are grouped as "Other". */
export function languageName(tag: string): string {
  if (!tag) return "Other";
  try {
    const name = displayNames?.of(tag);
    if (name && name !== tag) return name.replace("American English", "English (US)").replace("British English", "English (UK)");
  } catch {
    /* invalid tag */
  }
  return "Other";
}

/** Strips macOS's "(English (US))" suffix from a voice name. */
export const displayVoiceName = (name: string) => name.replace(/ \(English \((US|UK)\)\)$/, "");

export const megabytes = (bytes: number) => `${Math.round(bytes / 1_000_000)} MB`;

/** "12.4 MB/s", or "850 KB/s" on slow connections. */
export function speedLabel(bytesPerSecond: number): string {
  if (bytesPerSecond >= 1_000_000) return `${(bytesPerSecond / 1_000_000).toFixed(1)} MB/s`;
  return `${Math.max(1, Math.round(bytesPerSecond / 1_000))} KB/s`;
}

/** "About 20 s left", "About 3 min left". */
export function timeLeftLabel(seconds: number): string {
  if (seconds < 60) return `About ${Math.max(1, Math.round(seconds / 5) * 5 || 1)} s left`;
  if (seconds < 3600) return `About ${Math.round(seconds / 60)} min left`;
  return `About ${Math.floor(seconds / 3600)} h ${Math.round((seconds % 3600) / 60)} min left`;
}
