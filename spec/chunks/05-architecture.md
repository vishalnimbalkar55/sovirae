# 5. Architecture and platform boundaries

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This file is a topic-sized extract of the full specification. Original requirement numbering is preserved.

### 5.1 Proposed technology choices

- Desktop: WPF, MVVM, a supported .NET LTS, dependency injection, structured local logging.
- Baseline recommendation: .NET 10, replacing the pasted .NET 8 baseline. Microsoft's published support dates place .NET 8 near end of support in November 2026; .NET 10 has a longer runway. Recheck supported SDK/package combinations when implementation begins. [Source](https://dotnet.microsoft.com/en-us/platform/support/policy/dotnet-core)
- Core: headless C# library, with no WPF or `System.Windows` reference.
- Local neural inference: ONNX Runtime adapters where the exact exported model is compatible.
- Audio: WASAPI output, a tested SoundTouch binding or equivalent pitch-preserving time stretcher, and bounded PCM buffers.
- Windows built-in TTS: select one concrete Windows API adapter and enumerate its compatible voices; do not mix WinRT and `System.Speech` voice APIs accidentally.
- Extension: Manifest V3, JavaScript modules, HTML/CSS in shadow UI, no required bundler for the first version.
- Bridge: Chrome Native Messaging to a small host, then a per-user named pipe to the desktop process.
- Inference isolation: a worker process owned by the desktop app, so a failed native inference call can be terminated without killing the UI.

These are specification choices; no dependencies are installed by this document.

### 5.2 Proposed repository structure

```text
spec/SpeakIt-Product-Spec.md
src/SpeakIt.App/              WPF views, view models, tray, composition
src/SpeakIt.Core/             Text, sessions, scheduling, contracts
src/SpeakIt.Audio/            Playback, time stretching, device recovery
src/SpeakIt.Models/           Catalog, download verification, engine adapters
src/SpeakIt.InferenceHost/    Isolated model execution and PCM streaming
src/SpeakIt.Interop/          Windows-specific native interop
src/SpeakIt.NativeHost/       Chrome framing and desktop bridge
ext/                         Manifest, worker, popup, options, picker
tests/                       Core, bridge, integration, extension fixtures
benchmarks/                  Corpus, runner, machine metadata, reports
installer/                   Per-user installer and bridge registration
```

Only `spec/` is created during this planning task. The other paths describe future work.

### 5.3 Responsibilities

```text
Manual input ───┐
Clipboard ──────┼──> PlaybackSession ──> SpeechDocument ──> Scheduler
Auto-capture ───┤                                           │
Chrome bridge ──┘                                           ▼
                  Native player <── State       Local inference worker
                         ▲                                 │
                         └── Audio clock <── WASAPI <── PCM cache

Chrome picker -> Extension worker -> Native host -> Desktop named pipe
```

- Adapters collect text and source metadata; they do not create playback engines.
- Core validates and normalizes all text from all sources.
- One session owns playback, cancellation, segment ordering, and session identity.
- Audio consumes prepared PCM; it never performs synthesis or disk/network I/O on its callback.
- Model code exposes capabilities and synthesized audio, not UI controls.
- Interop calls live in the Windows-specific project.
- UI observes immutable snapshots or dispatcher-safe events.
- No LLM dependency or content-processing service belongs in this architecture.

---

[Previous](04-screens-and-settings.md) · [Next](06-reading-entry-paths.md)
