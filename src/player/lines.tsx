// Player line styles other than the waveform (Settings › Player line):
// word ticker, sentence steps, live meter, and breathing line.

import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { formatTime } from "../lib/format";
import { levelAt, spanIndexAt, textFraction, type LineSentence, type Span } from "./timeline";

export interface LineProps {
  timeline: Span[];
  positionMs: number;
  durationMs: number;
  durationIsFinal: boolean;
  playing: boolean;
  rate: number;
  onSeek: (fraction: number) => void;
}

const reducedMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

function token(name: string) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

/** Interpolates the reported position between controller updates. */
function usePlayhead(positionMs: number, playing: boolean, rate: number, durationMs: number) {
  const anchor = useRef({ pos: positionMs, at: performance.now() });
  useEffect(() => {
    anchor.current = { pos: positionMs, at: performance.now() };
  }, [positionMs]);
  const at = useCallback(
    (now: number) => {
      const elapsed = playing && !reducedMotion() ? (now - anchor.current.at) * rate : 0;
      return Math.min(anchor.current.pos + elapsed, Math.max(durationMs, 1));
    },
    [playing, rate, durationMs],
  );
  const pin = useCallback((pos: number) => {
    anchor.current = { pos, at: performance.now() };
  }, []);
  return useMemo(() => ({ at, pin }), [at, pin]);
}

/** Calls `tick` at `fps` while `active` and the window is visible. */
function useFrames(active: boolean, fps: number, tick: (now: number) => void) {
  const cb = useRef(tick);
  cb.current = tick;
  useEffect(() => {
    if (!active || reducedMotion()) return;
    let raf = 0;
    let last = 0;
    const loop = (now: number) => {
      if (now - last >= 1000 / fps) {
        last = now;
        cb.current(now);
      }
      raf = requestAnimationFrame(loop);
    };
    const start = () => {
      cancelAnimationFrame(raf);
      if (!document.hidden) raf = requestAnimationFrame(loop);
    };
    start();
    document.addEventListener("visibilitychange", start);
    return () => {
      cancelAnimationFrame(raf);
      document.removeEventListener("visibilitychange", start);
    };
  }, [active, fps]);
}

/** Invisible range over a line: dragging, clicking, and arrow keys seek. */
function SeekInput({ shown, durationMs, durationIsFinal, onScrub, onCommit }: {
  shown: number;
  durationMs: number;
  durationIsFinal: boolean;
  onScrub: (ms: number) => void;
  onCommit: () => void;
}) {
  const max = Math.max(durationMs, 1);
  return (
    <input
      type="range"
      className="wave-input"
      min={0}
      max={max}
      step={1000}
      value={Math.min(shown, max)}
      aria-label="Reading position"
      aria-valuetext={`${formatTime(shown)} of ${durationIsFinal ? "" : "about "}${formatTime(durationMs)}`}
      onChange={(e) => onScrub(Number(e.target.value))}
      onPointerUp={onCommit}
      onKeyUp={onCommit}
      onBlur={onCommit}
    />
  );
}

/** Scrub state shared by every style: the line follows the drag, then seeks once. */
function useScrub(props: LineProps) {
  const { positionMs, playing, rate, durationMs, onSeek } = props;
  const head = usePlayhead(positionMs, playing, rate, durationMs);
  const [scrub, setScrub] = useState<number | null>(null);
  const commit = () => {
    if (scrub === null) return;
    onSeek(scrub / Math.max(durationMs, 1));
    head.pin(scrub);
    setScrub(null);
  };
  return { head, scrub, setScrub, commit };
}

// ---- Canvas styles --------------------------------------------------------------

type Draw = (ctx: CanvasRenderingContext2D, w: number, h: number, pos: number, now: number) => void;

function CanvasLine({ draw, ...props }: LineProps & { draw: Draw }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const { head, scrub, setScrub, commit } = useScrub(props);

  const paint = useCallback(
    (now: number) => {
      const c = canvas.current;
      const ctx = c?.getContext("2d");
      if (!c || !ctx) return;
      const dpr = window.devicePixelRatio || 1;
      const w = c.clientWidth;
      const h = c.clientHeight;
      if (c.width !== Math.round(w * dpr) || c.height !== Math.round(h * dpr)) {
        c.width = Math.round(w * dpr);
        c.height = Math.round(h * dpr);
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, w, h);
      draw(ctx, w, h, scrub ?? head.at(now), now);
    },
    [draw, head, scrub],
  );

  useEffect(() => {
    paint(performance.now());
    const c = canvas.current;
    if (!c) return;
    const ro = new ResizeObserver(() => paint(performance.now()));
    ro.observe(c);
    return () => ro.disconnect();
  }, [paint, props.timeline, props.positionMs]);
  useFrames(props.playing && scrub === null, 30, paint);

  return (
    <div className="wave">
      <canvas ref={canvas} aria-hidden />
      <SeekInput
        shown={scrub ?? props.positionMs}
        durationMs={props.durationMs}
        durationIsFinal={props.durationIsFinal}
        onScrub={setScrub}
        onCommit={commit}
      />
    </div>
  );
}

function pill(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number) {
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, Math.min(w, h) / 2);
  ctx.fill();
}

/** Loudness around a moment, so thin gaps between frames do not flicker. */
function liveLevel(timeline: Span[], ms: number) {
  return Math.max(levelAt(timeline, ms - 20) ?? 0, levelAt(timeline, ms) ?? 0, levelAt(timeline, ms + 20) ?? 0);
}

const METER_PROFILE = [0.55, 0.85, 1, 0.8, 0.6];

/** A small equalizer that moves with the voice, beside a plain progress track. */
export function LiveMeter(props: LineProps) {
  const { timeline, durationMs, playing } = props;
  const draw = useCallback<Draw>(
    (ctx, w, h, pos) => {
      const violet = token("--wave-played");
      const mid = h / 2;
      const bw = 3;
      const gap = 2.5;
      METER_PROFILE.forEach((p, i) => {
        const lv = playing ? Math.max(0.14, liveLevel(timeline, pos - i * 45) * p) : 0.14;
        ctx.fillStyle = violet;
        pill(ctx, i * (bw + gap), mid - (lv * (h - 4)) / 2, bw, lv * (h - 4));
      });
      const x0 = METER_PROFILE.length * (bw + gap) + 10;
      const tw = w - x0 - 7;
      const px = x0 + (pos / Math.max(durationMs, 1)) * tw;
      ctx.fillStyle = token("--track");
      pill(ctx, x0, mid - 2, tw, 4);
      ctx.fillStyle = violet;
      pill(ctx, x0, mid - 2, Math.max(4, px - x0), 4);
      ctx.save();
      ctx.shadowColor = "rgba(0, 0, 0, 0.28)";
      ctx.shadowBlur = 3;
      ctx.shadowOffsetY = 1;
      ctx.fillStyle = "#fff";
      ctx.beginPath();
      ctx.arc(px, mid, 6, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
      ctx.fillStyle = violet;
      ctx.beginPath();
      ctx.arc(px, mid, 2.5, 0, Math.PI * 2);
      ctx.fill();
    },
    [timeline, durationMs, playing],
  );
  return <CanvasLine {...props} draw={draw} />;
}

/** A thin line that ripples only where the voice is. */
export function BreathingLine(props: LineProps) {
  const { timeline, durationMs, playing } = props;
  const draw = useCallback<Draw>(
    (ctx, w, h, pos, now) => {
      const mid = h / 2;
      const px = (pos / Math.max(durationMs, 1)) * w;
      const lv = playing && !reducedMotion() ? liveLevel(timeline, pos) : 0;
      const y = (x: number) => mid + lv * (h / 2 - 2) * Math.exp(-(((x - px) / 26) ** 2)) * Math.sin((x - px) * 0.42 - now * 0.018);
      ctx.lineWidth = 2;
      ctx.lineCap = "round";
      for (const [a, b, color] of [[px, w, token("--track")], [0, px, token("--wave-played")]] as const) {
        ctx.strokeStyle = color;
        ctx.beginPath();
        for (let x = a; x <= b; x += 1) {
          if (x === a) ctx.moveTo(x, y(x));
          else ctx.lineTo(x, y(x));
        }
        ctx.stroke();
      }
      ctx.fillStyle = token("--wave-played");
      ctx.beginPath();
      ctx.arc(px, y(px), 3, 0, Math.PI * 2);
      ctx.fill();
    },
    [timeline, durationMs, playing],
  );
  return <CanvasLine {...props} draw={draw} />;
}

// ---- Text styles ----------------------------------------------------------------

interface TextProps extends LineProps {
  sentences: LineSentence[];
}

interface Spot {
  sentence: number;
  word: number;
}

/** Interpolated position, re-rendered often enough for word changes. */
function useTextPosition(props: LineProps) {
  const scrubbing = useScrub(props);
  const { head, scrub } = scrubbing;
  const [pos, setPos] = useState(props.positionMs);
  useEffect(() => setPos(scrub ?? head.at(performance.now())), [props.positionMs, scrub, head]);
  useFrames(props.playing && scrub === null, 15, (now) => setPos(head.at(now)));
  return { ...scrubbing, pos };
}

function useLookup(timeline: Span[], sentences: LineSentence[]) {
  return useMemo(() => {
    const spans = new Map<number, Span>();
    timeline.forEach((s) => spans.set(s.segment, s));
    const sentenceOf = new Map<number, number>();
    sentences.forEach((s, i) => s.segments.forEach((seg) => sentenceOf.set(seg, i)));
    const range = (s: LineSentence) => {
      const first = spans.get(s.segments[0]);
      const last = spans.get(s.segments[s.segments.length - 1]);
      return first && last ? [first.startMs, last.startMs + last.durationMs] : [0, 0];
    };
    return { spans, sentenceOf, range };
  }, [timeline, sentences]);
}

/** The sentence and word being spoken (word -1 before the sentence starts). */
function spotAt(timeline: Span[], sentences: LineSentence[], sentenceOf: Map<number, number>, pos: number): Spot {
  const i = spanIndexAt(timeline, pos);
  if (i < 0) return { sentence: 0, word: -1 };
  const span = timeline[i];
  const sentence = sentenceOf.get(span.segment) ?? 0;
  const frac = textFraction(span, pos);
  const words = sentences[sentence]?.words ?? [];
  let word = -1;
  for (let w = 0; w < words.length; w++) {
    const wd = words[w];
    const segIndex = sentences[sentence].segments.indexOf(wd.segment);
    const curIndex = sentences[sentence].segments.indexOf(span.segment);
    if (segIndex < curIndex || (segIndex === curIndex && wd.at <= frac + 0.001)) word = w;
  }
  return { sentence, word };
}

/** The sentence being read on one sliding line, with the spoken word marked. */
export function WordTicker(props: TextProps) {
  const { timeline, sentences, durationMs, onSeek } = props;
  const { pos, scrub, setScrub, commit } = useTextPosition(props);
  const { spans, sentenceOf } = useLookup(timeline, sentences);
  const spot = spotAt(timeline, sentences, sentenceOf, pos);
  const sentence = sentences[spot.sentence];
  const mask = useRef<HTMLDivElement>(null);
  const line = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const m = mask.current;
    const l = line.current;
    if (!m || !l) return;
    const target = l.children[Math.max(0, spot.word)] as HTMLElement | undefined;
    const keep = Math.min(0, m.clientWidth - l.scrollWidth - 8);
    const x = target ? Math.min(0, m.clientWidth * 0.36 - target.offsetLeft) : 0;
    l.style.transform = `translateX(${Math.max(keep, x)}px)`;
  }, [spot.sentence, spot.word]);

  const seekWord = (segment: number, at: number) => {
    const span = spans.get(segment);
    if (span) onSeek((span.startMs + at * span.durationMs) / Math.max(durationMs, 1));
  };

  return (
    <div className="line-ticker">
      <div className="ticker-mask" ref={mask}>
        <div className="ticker-text" ref={line} key={spot.sentence}>
          {sentence?.words.map((w, i) => (
            <span
              key={i}
              className={`tw ${i < spot.word ? "done" : i === spot.word ? "now" : ""}`}
              onClick={() => seekWord(w.segment, w.at)}
            >
              {w.text}
            </span>
          ))}
        </div>
      </div>
      <div className="seek-zone">
        <div className="hair"><i style={{ width: `${(pos / Math.max(durationMs, 1)) * 100}%` }} /></div>
        <SeekInput shown={scrub ?? pos} durationMs={durationMs} durationIsFinal={props.durationIsFinal} onScrub={setScrub} onCommit={commit} />
      </div>
    </div>
  );
}

const MAX_STEPS = 40;

/** Progress split per sentence, with the current sentence above it. */
export function SentenceSteps(props: TextProps) {
  const { timeline, sentences, durationMs } = props;
  const { pos, scrub, setScrub, commit } = useTextPosition(props);
  const { sentenceOf, range } = useLookup(timeline, sentences);
  const spot = spotAt(timeline, sentences, sentenceOf, pos);
  const words = sentences[spot.sentence]?.words ?? [];
  // Long sentences start a few words before the spoken one so it stays visible.
  const from = Math.max(0, spot.word - 5);

  // Long readings group sentences so every step stays wide enough to see.
  const steps = useMemo(() => {
    const size = Math.max(1, Math.ceil(sentences.length / MAX_STEPS));
    const out: { start: number; end: number }[] = [];
    for (let i = 0; i < sentences.length; i += size) {
      const [start] = range(sentences[i]);
      const [, end] = range(sentences[Math.min(sentences.length, i + size) - 1]);
      out.push({ start: out.length === 0 ? 0 : start, end });
    }
    return out;
  }, [sentences, range]);

  return (
    <div className="line-steps">
      <div className="steps-caption">
        <span className="steps-text">
          {from > 0 && "… "}
          {words.slice(from).map((w, i) => (
            <span key={from + i}>{from + i === spot.word ? <b>{w.text}</b> : w.text} </span>
          ))}
        </span>
        <span className="steps-count">{sentences.length ? `${spot.sentence + 1} of ${sentences.length}` : ""}</span>
      </div>
      <div className="seek-zone">
        <div className="steps-track">
          {steps.map((s, i) => {
            const f = Math.min(1, Math.max(0, (pos - s.start) / Math.max(s.end - s.start, 1)));
            return (
              <span key={i} className="step" style={{ flexGrow: Math.max(s.end - s.start, 1) }}>
                <i style={{ width: `${f * 100}%` }} />
              </span>
            );
          })}
        </div>
        <SeekInput shown={scrub ?? pos} durationMs={durationMs} durationIsFinal={props.durationIsFinal} onScrub={setScrub} onCommit={commit} />
      </div>
    </div>
  );
}
