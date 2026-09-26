import { useEffect, useRef, useState } from "react";
import { formatTime } from "../lib/format";
import { levelAt, type Span } from "./timeline";

interface Props {
  timeline: Span[];
  positionMs: number;
  durationMs: number;
  durationIsFinal: boolean;
  playing: boolean;
  rate: number;
  onSeek: (fraction: number) => void;
}

const BAR = 3;
const GAP = 2;
const FPS_INTERVAL = 1000 / 30;

function token(name: string) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

export default function Waveform({ timeline, positionMs, durationMs, durationIsFinal, playing, rate, onSeek }: Props) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const anchor = useRef({ pos: positionMs, at: performance.now() });
  const [scrub, setScrub] = useState<number | null>(null);

  // Re-anchor interpolation whenever the controller reports a position.
  useEffect(() => {
    anchor.current = { pos: positionMs, at: performance.now() };
  }, [positionMs]);

  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    let raf = 0;
    let last = 0;
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    const draw = (now: number) => {
      const dpr = window.devicePixelRatio || 1;
      const w = c.clientWidth;
      const h = c.clientHeight;
      if (c.width !== Math.round(w * dpr) || c.height !== Math.round(h * dpr)) {
        c.width = Math.round(w * dpr);
        c.height = Math.round(h * dpr);
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, w, h);
      const total = Math.max(durationMs, 1);
      const elapsed = playing && !reduce ? (now - anchor.current.at) * rate : 0;
      const pos = scrub ?? Math.min(anchor.current.pos + elapsed, total);
      const played = token("--wave-played");
      const ahead = token("--wave-ahead");
      const dim = token("--wave-dim");
      const bars = Math.floor((w + GAP) / (BAR + GAP));
      const mid = h / 2;
      for (let i = 0; i < bars; i++) {
        const t = ((i + 0.5) / bars) * total;
        const level = levelAt(timeline, t);
        const x = i * (BAR + GAP);
        if (level === null) {
          ctx.fillStyle = dim;
          ctx.fillRect(x, mid - 1.5, BAR, 3);
          continue;
        }
        const bh = Math.max(3, level * (h - 2));
        ctx.fillStyle = t <= pos ? played : ahead;
        ctx.beginPath();
        ctx.roundRect(x, mid - bh / 2, BAR, bh, 1.5);
        ctx.fill();
      }
      // Playhead
      const px = (pos / total) * w;
      ctx.fillStyle = played;
      ctx.fillRect(Math.min(Math.max(px - 1, 0), w - 2), 1, 2, h - 2);
    };

    const loop = (now: number) => {
      if (now - last >= FPS_INTERVAL) {
        last = now;
        draw(now);
      }
      raf = requestAnimationFrame(loop);
    };
    draw(performance.now());
    // Animate only while audibly playing and visible; otherwise stay static.
    const animate = playing && !reduce && !document.hidden && scrub === null;
    if (animate) raf = requestAnimationFrame(loop);
    const onVis = () => {
      cancelAnimationFrame(raf);
      if (!document.hidden && animate) raf = requestAnimationFrame(loop);
    };
    document.addEventListener("visibilitychange", onVis);
    const ro = new ResizeObserver(() => draw(performance.now()));
    ro.observe(c);
    return () => {
      cancelAnimationFrame(raf);
      document.removeEventListener("visibilitychange", onVis);
      ro.disconnect();
    };
  }, [timeline, positionMs, durationMs, playing, rate, scrub]);

  const commit = () => {
    if (scrub !== null) {
      onSeek(scrub / Math.max(durationMs, 1));
      anchor.current = { pos: scrub, at: performance.now() };
      setScrub(null);
    }
  };

  const shown = scrub ?? positionMs;
  const approx = durationIsFinal ? "" : "about ";
  return (
    <div className="wave">
      <canvas ref={canvas} aria-hidden />
      <input
        type="range"
        className="wave-input"
        min={0}
        max={Math.max(durationMs, 1)}
        step={1000}
        value={Math.min(shown, Math.max(durationMs, 1))}
        aria-label="Reading position"
        aria-valuetext={`${formatTime(shown)} of ${approx}${formatTime(durationMs)}`}
        onChange={(e) => setScrub(Number(e.target.value))}
        onPointerUp={commit}
        onKeyUp={commit}
        onBlur={commit}
      />
    </div>
  );
}
