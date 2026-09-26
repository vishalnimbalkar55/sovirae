import type { DocumentView, Envelope } from "../lib/types";

const DEFAULT_MS_PER_CHAR = 62;

export interface Span {
  segment: number;
  startMs: number;
  durationMs: number;
  levels: number[] | null;
}

/** Lays segments out on the source-time axis: real durations where audio
 *  exists, char-based estimates elsewhere (spec §9.3). */
export function buildTimeline(doc: DocumentView | null, envelopes: Map<number, Envelope>): Span[] {
  if (!doc) return [];
  let knownMs = 0;
  let knownChars = 0;
  for (const [id, , start, end] of doc.segments) {
    const env = envelopes.get(id);
    if (env) {
      knownMs += env.durationMs;
      knownChars += end - start;
    }
  }
  const perChar = knownChars >= 80 ? knownMs / knownChars : DEFAULT_MS_PER_CHAR;
  let t = 0;
  return doc.segments.map(([id, , start, end]) => {
    const env = envelopes.get(id);
    const durationMs = env ? env.durationMs : (end - start) * perChar;
    const span = { segment: id, startMs: t, durationMs, levels: env ? env.levels : null };
    t += durationMs;
    return span;
  });
}

/** Level (0–1) at a source time, or null where audio is not synthesized. */
export function levelAt(timeline: Span[], ms: number, hz = 50): number | null {
  let lo = 0;
  let hi = timeline.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const s = timeline[mid];
    if (ms < s.startMs) hi = mid - 1;
    else if (ms >= s.startMs + s.durationMs) lo = mid + 1;
    else {
      if (!s.levels) return null;
      const i = Math.floor(((ms - s.startMs) / 1000) * hz);
      return (s.levels[Math.min(i, s.levels.length - 1)] ?? 0) / 255;
    }
  }
  return null;
}
