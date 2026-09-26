import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import type { AppModel } from "../App";
import { api } from "../lib/api";
import type { PlayerLine, ProcessorStatus, ResourceProfile, Settings } from "../lib/types";
import { Row, Section, Segmented, Slider, Switch } from "../lib/ui";

const PROFILE_HINT: Record<ResourceProfile, string> = {
  eco: "Prepares 8 seconds ahead with up to 32 MB of audio. Lightest on battery.",
  balanced: "Prepares 15 seconds ahead with up to 64 MB of audio.",
  performance: "Prepares 30 seconds ahead with up to 128 MB of audio.",
};

export default function SettingsScreen({ app }: { app: AppModel }) {
  const s = app.settings!;
  const set = (patch: Partial<Settings>) => app.update(patch);
  const [processor, setProcessor] = useState<ProcessorStatus | null>(null);
  useEffect(() => {
    api.processorStatus().then(setProcessor).catch(() => setProcessor(null));
  }, [s.processor]);

  return (
    <div className="screen" aria-labelledby="settings-title">
      <header className="screen-head">
        <h1 id="settings-title">Settings</h1>
        <p>Changes apply immediately and are saved on this device.</p>
      </header>

      {app.notice?.code === "SETTINGS" && (
        <div className="banner" role="alert"><span>{app.notice.message}</span></div>
      )}

      <Section title="Appearance">
        <div className="group">
          <Row label="Theme">
            <Segmented label="Theme" value={s.theme} onChange={(theme) => set({ theme })}
              options={[["system", "System"], ["light", "Light"], ["dark", "Dark"]]} />
          </Row>
          <Row label="Reading text size" hint="Read screen and expanded player" htmlFor="font-size">
            <Slider id="font-size" min={14} max={28} step={1} value={s.readingFontSize}
              onChange={(readingFontSize) => set({ readingFontSize })} />
            <span className="value">{s.readingFontSize} pt</span>
          </Row>
          <Row label="Reading font">
            <Segmented label="Reading font" value={s.readingFont} onChange={(readingFont) => set({ readingFont })}
              options={[["serif", "Serif"], ["sans", "Sans-serif"]]} />
          </Row>
          <div className="row stacked">
            <div className="row-label">
              <span id="player-line-label">Player line</span>
              <span className="hint">What the line under the player controls shows while reading. Word ticker and Sentence steps make the player a little taller.</span>
            </div>
            <LinePicker value={s.playerLine} onChange={(playerLine) => set({ playerLine })} />
          </div>
        </div>
      </Section>

      <Section title="Playback">
        <div className="group">
          <Row label="Speed" hint="Keeps the voice's natural pitch" htmlFor="rate">
            <Slider id="rate" min={0.5} max={3} step={0.1} value={s.rate} valueText={`${s.rate.toFixed(1)} times`}
              onChange={(rate) => set({ rate })} />
            <span className="value">{s.rate.toFixed(1)}×</span>
          </Row>
          <Row label="Volume" hint="Separate from the system volume" htmlFor="volume">
            <Slider id="volume" min={0} max={1} step={0.05} value={s.volume} valueText={`${Math.round(s.volume * 100)}%`}
              onChange={(volume) => set({ volume })} />
            <span className="value">{Math.round(s.volume * 100)}%</span>
          </Row>
          <Row label="Snap skips to the start of a sentence" htmlFor="snap">
            <Switch id="snap" checked={s.sentenceSnap} onChange={(sentenceSnap) => set({ sentenceSnap })} />
          </Row>
          <Row label="Scroll with the reading in the expanded player" htmlFor="follow">
            <Switch id="follow" checked={s.followReading} onChange={(followReading) => set({ followReading })} />
          </Row>
          <Row label="Keep the player above other windows" htmlFor="topmost">
            <Switch id="topmost" checked={s.playerTopmost} onChange={(playerTopmost) => set({ playerTopmost })} />
          </Row>
        </div>
      </Section>

      <Section title="Performance" description="How much work Sovirae does ahead of the voice.">
        <div className="group">
          <Row label="Resource use" hint={PROFILE_HINT[s.resourceProfile]}>
            <Segmented label="Resource use" value={s.resourceProfile}
              onChange={(resourceProfile) => set({ resourceProfile })}
              options={[["eco", "Eco"], ["balanced", "Balanced"], ["performance", "Performance"]]} />
          </Row>
          <Row label="Use GPU" hint={gpuHint(s, processor)} htmlFor="gpu">
            <Switch id="gpu" checked={s.processor === "gpu"} disabled={!processor?.gpuAvailable}
              onChange={(on) => set({ processor: on ? "gpu" : "cpu" })} />
          </Row>
        </div>
      </Section>

      <Section title="Background">
        <div className="group">
          <Row label="Open at login" htmlFor="login">
            <Switch id="login" checked={s.startAtLogin}
              onChange={(startAtLogin) => set({ startAtLogin, ...(startAtLogin ? {} : { startMinimized: false }) })} />
          </Row>
          <Row label="Start in the menu bar only" hint="Requires Open at login" htmlFor="minimized">
            <Switch id="minimized" checked={s.startMinimized} disabled={!s.startAtLogin}
              onChange={(startMinimized) => set({ startMinimized })} />
          </Row>
          <Row label="Keep running when the window closes" hint="Shortcuts and the menu bar icon stay available" htmlFor="keep">
            <Switch id="keep" checked={s.keepRunning} onChange={(keepRunning) => set({ keepRunning })} />
          </Row>
          <Row label="Pause global shortcuts" htmlFor="pause-hk">
            <Switch id="pause-hk" checked={s.hotkeysPaused} onChange={(hotkeysPaused) => set({ hotkeysPaused })} />
          </Row>
        </div>
      </Section>

      <Section title="Privacy">
        <div className="group">
          <Row label="Reading history" hint="Text you read is never written to disk">
            <span className="pill">Off</span>
          </Row>
          <Row label="Audio cache" hint="Held in memory for the current reading only">
            <span className="pill">Memory only</span>
          </Row>
          <Row label="Diagnostic log" hint="Errors only, and never the text you read">
            <span className="pill">Errors</span>
          </Row>
        </div>
      </Section>
    </div>
  );
}

function gpuHint(s: Settings, status: ProcessorStatus | null): string {
  if (!status?.gpuAvailable) return "Not available on this computer yet. Voices run on the CPU.";
  if (s.processor === "gpu" && status.fallback) {
    return `The GPU could not run downloaded voices, so they use the CPU. ${status.fallback}`;
  }
  return "Downloaded voices read faster and leave the CPU free. System voices are run by macOS.";
}

const BARS = [8, 14, 20, 12, 24, 16, 10, 22, 18, 9, 15, 26, 13, 19, 11, 21, 16, 8, 14, 23, 12, 17];
const WORDS = [14, 9, 17, 7, 12, 19, 10, 15];

/** Small still pictures of each style, drawn with the player's own colours. */
const PREVIEW: Record<PlayerLine, ReactNode> = {
  wave: (
    <>
      {BARS.map((h, i) => (
        <rect key={i} x={4 + i * 5.2} y={16 - h / 2} width={3} height={h} rx={1.5} className={i < 9 ? "pv-played" : "pv-ahead"} />
      ))}
      <rect x={50} y={2} width={1.6} height={28} className="pv-played" />
    </>
  ),
  ticker: (
    <>
      {WORDS.map((w, i) => {
        const x = 4 + WORDS.slice(0, i).reduce((a, b) => a + b + 4, 0);
        return i === 3 ? (
          <g key={i}><rect x={x - 3} y={6} width={w + 6} height={11} rx={3} className="pv-soft" /><rect x={x} y={9.5} width={w} height={4} rx={2} className="pv-played" /></g>
        ) : (
          <rect key={i} x={x} y={9.5} width={w} height={4} rx={2} className={i < 3 ? "pv-ink" : "pv-mute"} />
        );
      })}
      <rect x={4} y={25} width={112} height={2} rx={1} className="pv-track" />
      <rect x={4} y={25} width={44} height={2} rx={1} className="pv-played" />
    </>
  ),
  steps: (
    <>
      {WORDS.slice(0, 6).map((w, i) => {
        const x = 4 + WORDS.slice(0, i).reduce((a, b) => a + b + 4, 0);
        return <rect key={i} x={x} y={i === 2 ? 7.5 : 8.5} width={w} height={i === 2 ? 5 : 3} rx={1.5} className={i === 2 ? "pv-ink" : "pv-mute"} />;
      })}
      {[0, 1, 2, 3, 4, 5].map((i) => (
        <g key={i}>
          <rect x={4 + i * 19} y={23} width={16} height={4} rx={2} className="pv-track" />
          {i < 3 && <rect x={4 + i * 19} y={23} width={i < 2 ? 16 : 7} height={4} rx={2} className="pv-played" />}
        </g>
      ))}
    </>
  ),
  meter: (
    <>
      {[0.5, 0.8, 1, 0.7, 0.45].map((l, i) => (
        <rect key={i} x={4 + i * 5.5} y={16 - l * 10} width={3} height={l * 20} rx={1.5} className="pv-played" />
      ))}
      <rect x={38} y={14} width={78} height={4} rx={2} className="pv-track" />
      <rect x={38} y={14} width={30} height={4} rx={2} className="pv-played" />
      <circle cx={68} cy={16} r={5.5} className="pv-knob" />
      <circle cx={68} cy={16} r={2.2} className="pv-played" />
    </>
  ),
  breathing: (
    <>
      <path d="M62 16 H116" className="pv-line pv-line-track" />
      <path d="M4 16 H44 C47 16 48 9 51 9 S54 23 57 23 S60 12 62 16" className="pv-line pv-line-played" />
      <circle cx={62} cy={16} r={2.8} className="pv-played" />
    </>
  ),
};

const LINE_OPTIONS: [PlayerLine, string][] = [
  ["wave", "Waveform"],
  ["ticker", "Word ticker"],
  ["steps", "Sentence steps"],
  ["meter", "Live meter"],
  ["breathing", "Breathing line"],
];

function LinePicker({ value, onChange }: { value: PlayerLine; onChange: (v: PlayerLine) => void }) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  // Arrow keys move the choice, like a native radio group.
  const onKey = (e: KeyboardEvent, i: number) => {
    const d = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const next = (i + d + LINE_OPTIONS.length) % LINE_OPTIONS.length;
    onChange(LINE_OPTIONS[next][0]);
    refs.current[next]?.focus();
  };
  return (
    <div className="line-picker" role="radiogroup" aria-labelledby="player-line-label">
      {LINE_OPTIONS.map(([id, name], i) => (
        <button
          key={id}
          ref={(el) => {
            refs.current[i] = el;
          }}
          role="radio"
          aria-checked={value === id}
          tabIndex={value === id ? 0 : -1}
          className="line-option"
          onClick={() => onChange(id)}
          onKeyDown={(e) => onKey(e, i)}
        >
          <svg viewBox="0 0 120 32" aria-hidden>{PREVIEW[id]}</svg>
          <span>{name}</span>
        </button>
      ))}
    </div>
  );
}
