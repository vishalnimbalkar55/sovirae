import type { DocumentView, Envelope } from "../lib/types";

const DEFAULT_MS_PER_CHAR = 62;

/** How far the playhead may run ahead of the last reported position. The
 *  controller reports every 250 ms while audio plays; when reports stop
 *  (no audio), the playhead stops instead of moving over silence. */
const MAX_LEAD_MS = 500;

/** Position `now` ms after `anchor` was reported, while playing. */
export function extrapolate(anchor: { pos: number; at: number }, now: number, rate: number): number {
  return anchor.pos + Math.min(Math.max(now - anchor.at, 0), MAX_LEAD_MS) * rate;
}

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

/** Index of the span playing at a source time, or -1. */
export function spanIndexAt(timeline: Span[], ms: number): number {
  let lo = 0;
  let hi = timeline.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const s = timeline[mid];
    if (ms < s.startMs) hi = mid - 1;
    else if (ms >= s.startMs + s.durationMs) lo = mid + 1;
    else return mid;
  }
  return ms >= 0 && timeline.length ? timeline.length - 1 : -1;
}

/**
 * How much of a span's text has been spoken by `ms` (0–1). Engines report
 * no word timings, so this spreads the text over the voiced frames of the
 * envelope: pauses between phrases do not move the words along. Without an
 * envelope it falls back to elapsed time.
 */
export function textFraction(span: Span, ms: number): number {
  const f = Math.min(1, Math.max(0, (ms - span.startMs) / Math.max(span.durationMs, 1)));
  const levels = span.levels;
  if (!levels || levels.length < 4) return f;
  const upto = Math.floor(f * levels.length);
  let voiced = 0;
  let done = 0;
  for (let i = 0; i < levels.length; i++) {
    if (levels[i] > 18) {
      voiced++;
      if (i < upto) done++;
    }
  }
  return voiced ? done / voiced : f;
}

export interface LineWord {
  text: string;
  segment: number;
  /** Position of the word inside its segment's text, 0–1. */
  at: number;
}

export interface LineSentence {
  id: number;
  segments: number[];
  words: LineWord[];
}

/** Sentences of the document with their words, for the text player styles. */
export function buildSentences(doc: DocumentView | null): LineSentence[] {
  if (!doc) return [];
  const bytes = new TextEncoder().encode(doc.text);
  const dec = new TextDecoder();
  const out: LineSentence[] = [];
  for (const [seg, sentence, start, end] of doc.segments) {
    const text = dec.decode(bytes.subarray(start, end));
    let s = out[out.length - 1];
    if (!s || s.id !== sentence) {
      s = { id: sentence, segments: [], words: [] };
      out.push(s);
    }
    s.segments.push(seg);
    const len = Math.max(text.length, 1);
    for (const m of text.matchAll(/\S+/g)) s.words.push({ text: m[0], segment: seg, at: (m.index ?? 0) / len });
  }
  return out;
}
