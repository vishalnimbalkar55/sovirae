import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, on } from "../lib/api";
import { formatTime, rateLabel, sourceLabel } from "../lib/format";
import { Back10, Close, Collapse, Expand, Forward10, Mark, Mute, Pause, Play, Volume } from "../lib/icons";
import { Slider } from "../lib/ui";
import { useTheme } from "../lib/useApp";
import type { DocumentView, Envelope, Notice, PlayerLine as PlayerLineStyle, Settings, Snapshot, Status } from "../lib/types";
import { BreathingLine, LiveMeter, SentenceSteps, WordTicker, type LineProps } from "./lines";
import { buildSentences, buildTimeline, type LineSentence } from "./timeline";
import Waveform from "./Waveform";

const STATUS_LABEL: Partial<Record<Status, string>> = {
  loadingVoice: "Loading voice…",
  preparing: "Preparing text…",
  buffering: "Buffering audio…",
  paused: "Paused",
  recovering: "Reconnecting audio…",
  completed: "Finished",
  error: "Stopped",
};

const RATES = Array.from({ length: 26 }, (_, i) => Math.round((0.5 + i * 0.1) * 10) / 10);

export default function Player() {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [doc, setDoc] = useState<DocumentView | null>(null);
  const [envelopes, setEnvelopes] = useState<Map<number, Envelope>>(new Map());
  const [notice, setNotice] = useState<Notice | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [expanded, setExpanded] = useState(false);
  const lastVolume = useRef(0.8);
  useTheme(settings?.theme);

  useEffect(() => {
    api.getState().then((s) => {
      setSnap(s.snapshot);
      setSettings(s.settings);
    });
    api.getDocument().then(setDoc);
    const offs = [
      on("playback", (s) => {
        setSnap(s);
        if (s.sessionId !== 0) setNotice((n) => (n?.code === "TEXT_TOO_LONG" ? n : null));
      }),
      on("document", (d) => {
        setDoc(d);
        setEnvelopes(new Map());
      }),
      on("envelope", (e) =>
        setEnvelopes((m) => {
          const next = new Map(m);
          next.set(e.segment, e);
          return next;
        }),
      ),
      on("notice", setNotice),
      on("settings", setSettings),
      on("player-collapse", () => setExpanded(false)),
    ];
    return () => offs.forEach((off) => off());
  }, []);

  // Envelopes belong to one session; drop any from a different document.
  const sessionEnvelopes = useMemo(() => {
    if (!doc) return envelopes;
    const m = new Map<number, Envelope>();
    envelopes.forEach((e, k) => e.sessionId === doc.sessionId && m.set(k, e));
    return m;
  }, [envelopes, doc]);
  const timeline = useMemo(() => buildTimeline(doc, sessionEnvelopes), [doc, sessionEnvelopes]);
  const sentences = useMemo(() => buildSentences(doc), [doc]);
  const line = settings?.playerLine ?? "wave";

  // The text styles need a taller collapsed player; Rust knows the height.
  useEffect(() => {
    if (settings && !expanded) api.playerExpand(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [line]);

  const idle = !snap || snap.sessionId === 0;

  // A notice with no reading behind it hides itself after a while.
  const hovered = useRef(false);
  useEffect(() => {
    if (!notice || !idle) return;
    const t = window.setInterval(() => {
      if (!hovered.current) {
        setNotice(null);
        api.dismissPlayer();
      }
    }, 10_000);
    return () => clearInterval(t);
  }, [notice, idle]);

  const toggleExpand = useCallback(() => {
    const next = !expanded;
    setExpanded(next);
    api.playerExpand(next);
  }, [expanded]);

  const stop = useCallback(() => {
    setNotice(null);
    if (idle) api.dismissPlayer();
    else api.control("stop");
  }, [idle]);

  // Escape stops and hides only while the player itself has focus.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") stop();
      else if (e.key === " " && (e.target as HTMLElement)?.tagName !== "BUTTON" && (e.target as HTMLElement)?.tagName !== "INPUT") {
        e.preventDefault();
        api.control("toggle");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [stop]);

  const status = snap?.status ?? "idle";
  const playing = status === "playing";
  const showPause = playing || status === "buffering" || status === "preparing";
  const volume = snap?.volume ?? 0.8;
  const rate = snap?.rate ?? 1;
  const label = STATUS_LABEL[status];
  const remainingMin = snap ? Math.max(0, (snap.durationMs - snap.positionMs) / 60000 / rate) : 0;

  return (
    <div
      className={`player ${expanded ? "expanded" : ""}`}
      onMouseEnter={() => (hovered.current = true)}
      onMouseLeave={() => (hovered.current = false)}
    >
      <div className="p-head" data-tauri-drag-region>
        <span className="source-chip" title={snap?.source?.displayName ?? undefined}>
          <Mark className="source-mark" />
          {idle ? "Sovirae" : sourceLabel(snap?.source?.kind, snap?.source?.displayName)}
        </span>
        {!idle && snap?.voice && <span className="meta" data-tauri-drag-region>{snap.voice.replace(/ \(English \((US|UK)\)\)$/, "")}</span>}
        {!idle && <span className="meta" data-tauri-drag-region>{snap?.device}</span>}
        {label && !idle && <span className={`status ${status}`} role="status">{label}</span>}
        <span className="spacer" data-tauri-drag-region />
        {!idle && (
          <button className="icon-btn sm" onClick={toggleExpand} aria-label={expanded ? "Collapse player" : "Expand player"} title={expanded ? "Collapse" : "Expand"}>
            {expanded ? <Collapse /> : <Expand />}
          </button>
        )}
        <button className="icon-btn sm" onClick={stop} aria-label="Stop and close player" title="Stop and close">
          <Close />
        </button>
      </div>

      {notice && (idle || notice.code !== "NO_TEXT") ? (
        <div className="p-notice" role="alert">
          <span>{notice.message}</span>
          {notice.action === "readFirstPart" && (
            <button className="btn small" onClick={() => { setNotice(null); api.speakClipboard(true); }}>Read first part</button>
          )}
          <button className="btn ghost small" onClick={() => { setNotice(null); if (idle) api.dismissPlayer(); }}>Dismiss</button>
        </div>
      ) : null}

      {!idle && !(notice && idle) && (
        <>
          <div className="p-controls">
            <button className="icon-btn" onClick={() => api.control("skip", -10000)} aria-label="Back 10 seconds" title="Back 10 seconds">
              <Back10 />
            </button>
            <button className="icon-btn play" onClick={() => api.control("toggle")} aria-label={showPause ? "Pause" : "Play"}>
              {showPause ? <Pause /> : <Play />}
            </button>
            <button className="icon-btn" onClick={() => api.control("skip", 10000)} aria-label="Forward 10 seconds" title="Forward 10 seconds">
              <Forward10 />
            </button>
            <label className="rate">
              <span className="visually-hidden">Playback speed</span>
              <select value={rate} onChange={(e) => api.control("rate", Number(e.target.value))}>
                {RATES.map((r) => (
                  <option key={r} value={r}>{rateLabel(r)}</option>
                ))}
              </select>
            </label>
            <div className="vol">
              <button
                className="icon-btn sm"
                aria-label={volume === 0 ? "Unmute" : "Mute"}
                onClick={() => {
                  if (volume > 0) {
                    lastVolume.current = volume;
                    api.control("volume", 0);
                  } else api.control("volume", lastVolume.current || 0.8);
                }}
              >
                {volume === 0 ? <Mute /> : <Volume />}
              </button>
              <Slider
                min={0}
                max={1}
                step={0.05}
                value={volume}
                label="Volume"
                valueText={`${Math.round(volume * 100)}%`}
                onChange={(v) => api.control("volume", v)}
              />
            </div>
            <span className="spacer" />
            <span className="time" title={`About ${Math.ceil(remainingMin)} min left at ${rateLabel(rate)}`}>
              {formatTime(snap!.positionMs)}
              <span className="muted"> / {snap!.durationIsFinal ? "" : "~"}{formatTime(snap!.durationMs)}</span>
            </span>
          </div>
          <PlayerLine
            style={line}
            sentences={sentences}
            timeline={timeline}
            positionMs={snap!.positionMs}
            durationMs={snap!.durationMs}
            durationIsFinal={snap!.durationIsFinal}
            playing={playing}
            rate={rate}
            onSeek={(f) => api.control("seekFraction", f)}
          />
          {snap?.message && <p className="p-message" role="status">{snap.message}</p>}
          {expanded && doc && (
            <ReadingPane
              doc={doc}
              sentenceId={snap?.sentenceId ?? null}
              follow={settings?.followReading ?? true}
              fontSize={settings?.readingFontSize ?? 18}
              serif={(settings?.readingFont ?? "serif") === "serif"}
            />
          )}
        </>
      )}
    </div>
  );
}

function PlayerLine({ style, sentences, ...props }: LineProps & { style: PlayerLineStyle; sentences: LineSentence[] }) {
  switch (style) {
    case "ticker":
      return <WordTicker {...props} sentences={sentences} />;
    case "steps":
      return <SentenceSteps {...props} sentences={sentences} />;
    case "meter":
      return <LiveMeter {...props} />;
    case "breathing":
      return <BreathingLine {...props} />;
    default:
      return <Waveform {...props} />;
  }
}

function ReadingPane({ doc, sentenceId, follow, fontSize, serif }: {
  doc: DocumentView;
  sentenceId: number | null;
  follow: boolean;
  fontSize: number;
  serif: boolean;
}) {
  const pane = useRef<HTMLDivElement>(null);
  const [following, setFollowing] = useState(follow);
  const programmatic = useRef(false);

  // Byte offsets from Rust index UTF-8; slice through an encoder.
  const paragraphs = useMemo(() => {
    const bytes = new TextEncoder().encode(doc.text);
    const dec = new TextDecoder();
    const sentences = new Map<number, { firstSegment: number; text: string; para: boolean }>();
    for (const [seg, sentence, start, end, paraStart] of doc.segments) {
      const text = dec.decode(bytes.subarray(start, end));
      const s = sentences.get(sentence);
      if (s) s.text += " " + text;
      else sentences.set(sentence, { firstSegment: seg, text, para: paraStart });
    }
    const out: { id: number; firstSegment: number; text: string }[][] = [];
    for (const [id, s] of sentences) {
      if (s.para || out.length === 0) out.push([]);
      out[out.length - 1].push({ id, firstSegment: s.firstSegment, text: s.text });
    }
    return out;
  }, [doc]);

  useEffect(() => setFollowing(follow), [follow]);

  useEffect(() => {
    if (!following || sentenceId === null) return;
    const el = pane.current?.querySelector<HTMLElement>(`[data-sentence="${sentenceId}"]`);
    if (!el) return;
    programmatic.current = true;
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    el.scrollIntoView({ block: "center", behavior: reduce ? "auto" : "smooth" });
    window.setTimeout(() => (programmatic.current = false), 600);
  }, [sentenceId, following]);

  return (
    <div className="p-reading-wrap">
      <div
        className="p-reading"
        ref={pane}
        style={{ fontSize, fontFamily: serif ? "var(--font-read-serif)" : "var(--font-read-sans)" }}
        onWheel={() => setFollowing(false)}
        onTouchMove={() => setFollowing(false)}
        tabIndex={0}
        aria-label="Reading text"
      >
        {paragraphs.map((p, i) => (
          <p key={i}>
            {p.map((s) => (
              <span
                key={s.id}
                data-sentence={s.id}
                className={`sentence ${s.id === sentenceId ? "current" : ""}`}
                role="button"
                tabIndex={0}
                aria-current={s.id === sentenceId ? "true" : undefined}
                onClick={() => api.control("seekSegment", s.firstSegment)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") api.control("seekSegment", s.firstSegment);
                }}
              >
                {s.text}{" "}
              </span>
            ))}
          </p>
        ))}
      </div>
      {!following && (
        <button className="btn small return" onClick={() => setFollowing(true)}>Return to current sentence</button>
      )}
    </div>
  );
}
