// Browser-only Studio mock: in-memory projects and a timed generation so the
// screen can be reviewed without the Rust side.
import { emit } from "@tauri-apps/api/event";
import { paragraphsOf } from "./studioText";
import type { StudioFeature, StudioParagraph, StudioProject, StudioSummary } from "./types";

const SCRIPT = `Welcome to the show. Today we look at how a small voice model fits on an ordinary laptop. [pause]

The hard part is not the voice itself. It is starting quickly and leaving the machine free for real work.

So let's begin with the numbers, and then listen to a few examples. [pause 1s] Ready?`;

const projects: StudioProject[] = [
  {
    id: "demo-1", name: "Podcast intro", created: 1, updated: 3, voice: "Samantha",
    versions: [{ id: 1, created: 1, text: SCRIPT.replace(" [pause]", "") }, { id: 2, created: 2, text: SCRIPT }], current: 2,
    generated: null,
  },
  { id: "demo-2", name: "Product tour", created: 1, updated: 2, voice: null, versions: [{ id: 1, created: 1, text: "" }], current: 1, generated: null },
];
let running: string | null = null;

const FEATURES: StudioFeature[] = [
  { id: "pause", label: "Short pause", description: "A beat of silence, about half a second.", example: "[pause]", source: "app", approximate: false },
  { id: "pause-length", label: "Timed pause", description: "Silence of a given length, up to 10 seconds.", example: "[pause 2s]", source: "app", approximate: false },
  { id: "paragraph", label: "New paragraph", description: "A blank line starts a new paragraph, generated and downloadable on its own.", example: "(blank line)", source: "app", approximate: false },
  { id: "question", label: "Question", description: "A question mark lifts the end of the sentence.", example: "Really?", source: "app", approximate: false },
  { id: "emphasis", label: "Emphasis", description: "An exclamation mark adds energy to the sentence.", example: "That's it!", source: "app", approximate: false },
  { id: "trail", label: "Trailing off", description: "An ellipsis slows the ending.", example: "I wonder…", source: "app", approximate: false },
  ...[["laugh", "Laugh", "Ha ha ha!"], ["chuckle", "Chuckle", "Heh heh."], ["giggle", "Giggle", "Hee hee hee!"], ["sigh", "Sigh", "Ahh…"],
    ["clear throat", "Clear throat", "Ahem."], ["gasp", "Gasp", "Oh!"], ["hmm", "Thinking sound", "Hmm."]].map(([id, label, spoken]) => ({
    id, label, description: `Approximate: the voice says “${spoken}”.`, example: `[${id}]`, source: "app" as const, approximate: true,
  })),
];

const previewCount = (ps: string[]) => {
  let chars = 0, n = 0;
  for (const p of ps) { n++; chars += p.length; if (chars >= 200 || n >= 3) break; }
  return Math.max(1, Math.min(n, ps.length));
};
const find = (id: unknown) => {
  const p = projects.find((x) => x.id === id);
  if (!p) throw "This project no longer exists.";
  return p;
};
const touch = (p: StudioProject) => { p.updated = Date.now() / 1000; setTimeout(() => emit("studio", { ...p }), 0); return { ...p }; };

/** A short WAV tone so the in-app player has something to play. */
function tone(seconds: number, hz: number): ArrayBuffer {
  const rate = 24000, n = Math.round(seconds * rate);
  const buf = new ArrayBuffer(44 + n * 2), v = new DataView(buf);
  const str = (o: number, s: string) => [...s].forEach((c, i) => v.setUint8(o + i, c.charCodeAt(0)));
  str(0, "RIFF"); v.setUint32(4, 36 + n * 2, true); str(8, "WAVE"); str(12, "fmt "); v.setUint32(16, 16, true);
  v.setUint16(20, 1, true); v.setUint16(22, 1, true); v.setUint32(24, rate, true); v.setUint32(28, rate * 2, true);
  v.setUint16(32, 2, true); v.setUint16(34, 16, true); str(36, "data"); v.setUint32(40, n * 2, true);
  for (let i = 0; i < n; i++) v.setInt16(44 + i * 2, Math.round(6000 * Math.sin((i / rate) * hz * 2 * Math.PI) * Math.min(1, (n - i) / 2000)), true);
  return buf;
}

export function mockStudio(cmd: string, args: Record<string, unknown>): unknown {
  switch (cmd) {
    case "studio_list":
      return projects.map<StudioSummary>((p) => ({
        id: p.id, name: p.name, updated: p.updated, voice: p.voice, running: running === p.id,
        ready: p.generated?.paragraphs.filter((q) => q.status === "ready").length ?? 0, total: p.generated?.paragraphs.length ?? 0,
      })).sort((a, b) => b.updated - a.updated);
    case "studio_create": {
      const p: StudioProject = { id: `p${Date.now()}`, name: String(args.name || "Untitled project"), created: Date.now() / 1000, updated: Date.now() / 1000, voice: null, versions: [{ id: 1, created: 1, text: "" }], current: 1, generated: null };
      projects.push(p);
      return touch(p);
    }
    case "studio_get": return { ...find(args.id) };
    case "studio_rename": { const p = find(args.id); p.name = String(args.name); return touch(p); }
    case "studio_delete": { projects.splice(projects.findIndex((p) => p.id === args.id), 1); return null; }
    case "studio_save_script": {
      const p = find(args.id);
      if (args.newVersion) { const id = Math.max(...p.versions.map((v) => v.id)) + 1; p.versions.push({ id, created: Date.now() / 1000, text: String(args.text) }); p.current = id; }
      else p.versions.find((v) => v.id === p.current)!.text = String(args.text);
      return touch(p);
    }
    case "studio_select_version": { const p = find(args.id); p.current = Number(args.version); return touch(p); }
    case "studio_delete_version": {
      const p = find(args.id);
      if (p.versions.length <= 1) throw "A project keeps at least one iteration.";
      p.versions = p.versions.filter((v) => v.id !== args.version);
      if (p.current === args.version) p.current = p.versions[p.versions.length - 1].id;
      return touch(p);
    }
    case "studio_set_voice": { const p = find(args.id); p.voice = String(args.voice); return touch(p); }
    case "studio_features": return FEATURES;
    case "studio_cancel": running = null; return null;
    case "studio_audio": {
      const p = find(args.id);
      const q = p.generated?.paragraphs[Number(args.paragraph)];
      if (!q?.file) throw "This paragraph has no audio yet.";
      return tone(q.durationMs / 1000, 220 + 40 * q.index);
    }
    case "studio_export": return `/Users/you/Downloads/${find(args.id).name}.wav`;
    case "studio_generate": {
      const p = find(args.id);
      if (running) throw "Wait for the other project to finish generating.";
      if (!p.voice) throw "Choose a voice for this project.";
      const text = p.versions.find((v) => v.id === p.current)!.text;
      const ps = paragraphsOf(text);
      if (!ps.length) throw "Write or paste a script first.";
      const scope = String(args.scope);
      const old = scope !== "all" && p.generated && p.generated.version === p.current && p.generated.voice === p.voice ? p.generated : null;
      const pool = old ? [...old.paragraphs] : [];
      const nPreview = previewCount(ps);
      const paragraphs = ps.map<StudioParagraph>((t, index) => {
        const at = scope === "preview" && index < nPreview ? -1 : pool.findIndex((q) => q.status === "ready" && q.text === t);
        if (at >= 0) return { ...pool.splice(at, 1)[0], index };
        return { index, text: t, preview: t.slice(0, 90), chars: t.length, status: "pending", file: null, durationMs: 0, skippedMarkers: 0, error: null };
      });
      p.generated = {
        version: p.current, voice: p.voice, previewCount: nPreview, accepted: scope !== "preview",
        running: true, error: null, paragraphs,
      };
      const g = p.generated;
      running = p.id;
      const todo = g.paragraphs.filter((q) => (scope === "preview" ? q.index < g.previewCount : true) && q.status !== "ready");
      let i = 0;
      const step = () => {
        if (running !== p.id || i >= todo.length) {
          g.running = false; if (running !== p.id && i < todo.length) g.error = "Generation stopped.";
          running = null; touch(p); return;
        }
        const q = todo[i]; q.status = "generating"; touch(p);
        setTimeout(() => {
          if (running !== p.id) { q.status = "pending"; step(); return; }
          q.status = "ready"; q.file = `v${g.version}-p${q.index}.wav`; q.durationMs = 400 + q.chars * 55;
          q.skippedMarkers = (ps[q.index].match(/\[(?!pause|laugh|chuckle|giggle|sigh|clear|gasp|hmm)[^\]]*\]/g) ?? []).length;
          i++; touch(p); step();
        }, 900);
      };
      setTimeout(step, 100);
      return touch(p);
    }
    default: return null;
  }
}
