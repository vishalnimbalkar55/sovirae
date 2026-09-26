// Browser-only design preview: when the UI runs outside Tauri during
// development, answer commands with realistic sample data so screens and
// states can be reviewed. Never included in a Tauri build path at runtime.
import { mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";

const SAMPLE = `Local speech synthesis has improved quickly. A compact model can now run on an ordinary laptop and still sound natural enough for long reading sessions.

The difficult part is not only the voice. It is starting quickly, staying responsive, and leaving the rest of the machine free for the work you are actually doing.`;

export function installDevMock() {
  if ("__TAURI_INTERNALS__" in window || !import.meta.env.DEV) return;
  const params = new URLSearchParams(location.search);
  const state = params.get("state") ?? "playing";
  const settings = {
    theme: params.get("theme") ?? "system", readingFontSize: 18, readingFont: "serif", rate: 1.2, volume: 0.8,
    voice: "Samantha" as string | null, resourceProfile: "balanced", startAtLogin: false, keepRunning: true, startMinimized: false,
    hotkeysPaused: false, sentenceSnap: false, followReading: true, playerTopmost: true,
    playerLine: params.get("line") ?? "wave",
    chromeBridge: true, pairedExtensions: [] as string[], matchPageLanguage: true,
  };
  const segments: [number, number, number, number, boolean][] = [];
  const bytes = new TextEncoder();
  let offset = 0;
  SAMPLE.split("\n\n").forEach((para, pi) => {
    const sentences = para.match(/[^.]+\./g) ?? [];
    sentences.forEach((s, si) => {
      const t = s.trim();
      const start = bytes.encode(SAMPLE.slice(0, SAMPLE.indexOf(t, offset))).length;
      offset = SAMPLE.indexOf(t, offset) + t.length;
      segments.push([segments.length, segments.length, start, start + bytes.encode(t).length, si === 0 && pi >= 0]);
    });
  });
  const idle = state === "idle" || state === "notice";
  const snapshot = {
    sessionId: idle ? 0 : 3, status: idle ? "idle" : state, source: idle ? null : { kind: "clipboard", displayName: "Clipboard" },
    segmentId: 1, sentenceId: 1, positionMs: 7400, durationMs: 28600, durationIsFinal: false, rate: 1.2, volume: 0.8,
    voice: idle ? null : "Samantha", device: "CPU", message: null,
  };
  const modelState = params.get("model") ?? "missing";
  if (params.get("voice")) settings.voice = params.get("voice")!;
  mockIPC(
    (cmd, args) => {
      switch (cmd) {
        case "get_state": return { settings, snapshot, platform: "macos", engine: "macos-system" };
        case "get_document": return { sessionId: 3, text: SAMPLE, segments };
        case "list_voices": {
          const sys = [
            ["Samantha", "en-US", "female", true], ["Daniel", "en-GB", "male", true], ["Moira", "en-IE", "female", true],
            ["Eddy (English (US))", "en-US", "male", false], ["Flo (English (UK))", "en-GB", "female", false],
            ["Reed (English (US))", "en-US", "male", false], ["Zarvox", "en-US", null, false],
            ["Kyoko", "ja-JP", "female", false], ["Thomas", "fr-FR", "male", false], ["Tingting", "zh-CN", "female", false],
          ].map(([id, language, gender, recommended]) => ({
            id, name: id, language, gender, model: "system", engine: "System voice", recommended,
            approximatePronunciation: false,
          }));
          const kokoro = modelState === "installed" || modelState === "update" ? [
            ["af_heart", "Heart", "en-US", "female"], ["af_bella", "Bella", "en-US", "female"], ["am_michael", "Michael", "en-US", "male"],
            ["am_adam", "Adam", "en-US", "male"], ["am_echo", "Echo", "en-US", "male"], ["am_eric", "Eric", "en-US", "male"],
            ["bf_emma", "Emma", "en-GB", "female"], ["bm_george", "George", "en-GB", "male"],
            ["ef_dora", "Dora", "es-ES", "female"], ["jf_alpha", "Alpha", "ja-JP", "female"], ["zm_yunxi", "Yunxi", "zh-CN", "male"],
          ].map(([id, name, language, gender]) => ({
            id: `kokoro:${id}`, name, language, gender, model: "kokoro-82m-v1.0", engine: "Kokoro 82M", recommended: false,
            approximatePronunciation: id.startsWith("j") || id.startsWith("z"),
          })) : [];
          return [...sys, ...kokoro];
        }
        case "get_shortcuts": return [
          ["speakClipboard", "Command+Alt+R", false, true], ["readSelection", "Command+Alt+S", false, true],
          ["playPause", "Command+Alt+Space", true, false], ["skipBack", "Command+Alt+ArrowLeft", true, false],
          ["skipForward", "Command+Alt+ArrowRight", true, false], ["speedUp", "Command+Alt+ArrowUp", true, false],
          ["slowDown", "Command+Alt+ArrowDown", true, false], ["stop", "Command+Alt+X", true, false],
        ].map(([action, binding, playbackOnly, registered]) => ({
          action, binding, playbackOnly, registered, error: null,
          conflict: binding === "Command+Alt+Space" ? "Opens a Finder search window by default."
            : String(binding).endsWith("ArrowLeft") || String(binding).endsWith("ArrowRight") ? "Switches tabs in Chrome, Safari, and many editors." : null,
        }));
        case "bridge_status": return {
          enabled: true, listening: true, extensionId: "jhbbmlhbjhjdepmaebpniefhaoljfgoe",
          extensionFolder: "/Users/you/Projects/sovirae/ext", hostInstalled: true,
          browsers: [{ name: "Google Chrome", registered: true }, { name: "Arc", registered: false }],
          paired: params.get("paired") ? ["jhbbmlhbjhjdepmaebpniefhaoljfgoe"] : [], connected: params.get("paired") ? 1 : 0,
          lastConnectionSecs: params.get("paired") ? 5 : null, lastExtension: null,
        };
        case "bridge_test": return { ok: true, message: "Google Chrome can reach Sovirae. The extension is allowed." };
        case "update_settings":
          Object.assign(settings, (args as { patch: object }).patch);
          setTimeout(() => emit("settings", { ...settings }), 0);
          return { ...settings };
        case "list_models": return [
          {
            id: "system", name: "System voices", description: "Built into macOS. No download needed.", builtIn: true,
            voicePrefix: "", license: null, licenseUrl: null, cardUrl: null, artifacts: [], voicesBytes: 0, voiceCount: 176,
            voicesMissing: 0, voicesMissingBytes: 0,
            installedArtifact: "built-in", downloading: null, progress: null, bytesPerSecond: null, secondsLeft: null,
            error: null, missing: [],
          },
          {
            id: "kokoro-82m-v1.0", name: "Kokoro 82M", description: "A compact neural voice with natural rhythm. Runs on the processor.",
            builtIn: false, voicePrefix: "kokoro", license: "Apache-2.0",
            licenseUrl: "https://huggingface.co/hexgrad/Kokoro-82M/blob/main/LICENSE", cardUrl: "https://huggingface.co/hexgrad/Kokoro-82M",
            artifacts: [
              { id: "fp32", label: "Full precision", description: "Reference quality.", bytes: 325532232 },
              { id: "q8", label: "Compact", description: "8-bit weights, about a third of the size.", bytes: 92361116 },
            ],
            voicesBytes: 28_734_464, voiceCount: 55,
            voicesMissing: modelState === "update" ? 27 : 0, voicesMissingBytes: modelState === "update" ? 14_110_000 : 0,
            installedArtifact: modelState === "installed" || modelState === "update" ? "fp32" : null,
            downloading: modelState === "downloading" ? "fp32" : null,
            progress: modelState === "downloading" ? { doneBytes: 131_000_000, totalBytes: 340_155_464, verifying: false } : null,
            bytesPerSecond: modelState === "downloading" ? 12_400_000 : null,
            secondsLeft: modelState === "downloading" ? 17 : null,
            error: modelState === "failed" ? "The download could not be verified. Download again." : null,
            missing: [],
          },
        ];
        default: return null;
      }
    },
    { shouldMockEvents: true },
  );
  if (!idle) {
    // Waveform envelopes for the first two segments only: the rest shows as
    // not-yet-synthesized placeholder.
    setTimeout(() => {
      [0, 1, 2].forEach((segment) => {
        const durationMs = [6200, 5100, 4300][segment];
        const levels = Array.from({ length: Math.round(durationMs / 20) }, (_, i) =>
          Math.round(255 * Math.max(0.05, Math.abs(Math.sin(i / 7 + segment)) * (0.55 + 0.45 * Math.sin(i / 31)) ** 2)));
        emit("envelope", { sessionId: 3, segment, durationMs, levels });
      });
    }, 50);
  }
  if (state === "notice") {
    setTimeout(() => emit("notice", { code: "NO_TEXT", message: "Clipboard is empty. Copy some text and try again.", action: null }), 50);
  }
  if (state === "playing" && location.pathname.endsWith("player.html")) setTimeout(() => simulatePlayback(segments, snapshot), 300);
}

/** Speech-like envelopes and a moving position, so player styles can be reviewed live. */
function simulatePlayback(segments: [number, number, number, number, boolean][], snapshot: { positionMs: number; durationMs: number; durationIsFinal: boolean; segmentId: number | null; sentenceId: number | null; rate: number }) {
  let seed = 11;
  const rand = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  const bytes = new TextEncoder().encode(SAMPLE);
  const dec = new TextDecoder();
  let t = 0;
  const spans = segments.map(([id, , start, end]) => {
    const levels: number[] = [];
    for (const word of dec.decode(bytes.subarray(start, end)).split(/\s+/)) {
      const letters = word.replace(/[^a-z]/gi, "").length;
      const frames = Math.round((110 + 56 * letters) / 20);
      const syl = Math.max(1, Math.round(letters / 3));
      const loud = 0.55 + 0.45 * rand();
      for (let i = 0; i < frames; i++) {
        const ph = i / frames;
        levels.push(Math.round(255 * Math.min(1, loud * Math.pow(Math.sin(Math.PI * ((ph * syl) % 1)), 0.7) + 0.05 * rand())));
      }
      const gap = /[.!?]$/.test(word) ? 16 : /,$/.test(word) ? 10 : 3;
      for (let i = 0; i < gap; i++) levels.push(Math.round(6 * rand()));
    }
    const span = { id, start: t, durationMs: levels.length * 20, levels };
    t += span.durationMs;
    return span;
  });
  snapshot.durationMs = t;
  snapshot.durationIsFinal = true;
  snapshot.positionMs = 2000;
  for (const s of spans) emit("envelope", { sessionId: 3, segment: s.id, durationMs: s.durationMs, levels: s.levels });
  setInterval(() => {
    snapshot.positionMs = (snapshot.positionMs + 250 * snapshot.rate) % t;
    const cur = spans.find((s) => snapshot.positionMs < s.start + s.durationMs) ?? spans[0];
    snapshot.segmentId = cur.id;
    snapshot.sentenceId = segments[cur.id][1];
    emit("playback", { ...snapshot });
  }, 250);
}
