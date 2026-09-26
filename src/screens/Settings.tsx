import type { AppModel } from "../App";
import type { ResourceProfile, Settings } from "../lib/types";
import { Row, Section, Segmented, Slider, Switch } from "../lib/ui";

const PROFILE_HINT: Record<ResourceProfile, string> = {
  eco: "Prepares 8 seconds ahead with up to 32 MB of audio. Lightest on battery.",
  balanced: "Prepares 15 seconds ahead with up to 64 MB of audio.",
  performance: "Prepares 30 seconds ahead with up to 128 MB of audio.",
};

export default function SettingsScreen({ app }: { app: AppModel }) {
  const s = app.settings!;
  const set = (patch: Partial<Settings>) => app.update(patch);

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
          <Row label="Processor" hint="GPU appears here only for voices verified on your hardware">
            <span className="pill">CPU</span>
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
