# 5. Architecture and platform boundaries

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This topic is synchronized with the full specification; original numbering is preserved.

### 5.1 Chosen stack and boundaries

- **Desktop shell:** Tauri 2 with a Rust backend and React + TypeScript UI. Use Vite for the frontend build, Tauri commands/events for narrow UI ↔ Rust messages, and explicit capability permissions. Tauri uses the platform webview; verify rendering on WebView2, WKWebView, and WebKitGTK. [Tauri architecture](https://v2.tauri.app/start/)
- **Product core:** Rust crates for `SpeechDocument`, deterministic text rules, playback state, synthesis scheduling, model/catalog management, resource policies, and settings. The UI never owns playback state or runs inference in JavaScript.
- **TTS runtime:** a packaged Rust/native worker process, with ONNX Runtime for the verified Kokoro artifact. Piper is conditional on integration and licensing checks. No production Python, PyTorch, `llama.cpp`, GGUF, LLM, STT, microphone, or local LLM HTTP server.
- **Audio:** use `cpal` as the initial Rust output library on Windows, macOS, and Linux, with a bounded native PCM ring. Verify its default-device changes, clocks, Linux backend dependencies, and package behavior in a spike. Choose a pitch-preserving time-stretch library/binding only after a quality, portability, and license test; NAudio/WASAPI cannot be the shared implementation. [CPAL platform support](https://github.com/RustAudio/cpal)
- **OS services:** a small Rust adapter layer for global shortcuts, foreground selection capture, clipboard, tray/menu, startup, focus-safe floating window, output-device changes, and media controls. Each adapter reports capabilities instead of promising unsupported OS behavior.
- **Chrome:** MV3 extension → Chrome Native Messaging host → authenticated local IPC → the running Tauri desktop process. The host is a small native Rust executable with shared protocol definitions. Use a per-user named pipe on Windows and a user-owned Unix-domain socket on macOS/Linux, not an open localhost server.
- **Worker packaging:** package per-target worker binaries as Tauri sidecars; scope launch permissions to exact executables and fixed arguments. [Tauri sidecars](https://v2.tauri.app/develop/sidecar/)
- **Process ownership:** Tauri is the single session owner. It supervises inference and bridge workers, owns cancellation/session IDs, and shuts them down on Exit. A model worker can crash or be restarted without losing the UI.

The original conversation discussed both LLM and TTS. Only the TTS runtime and hardware-selection ideas are adopted. Its example Python/FastAPI service, `llama-server`, SSE token stream, and generic GPU mapping are not dependencies of SpeakIt.

### 5.2 Proposed repository structure

```text
spec/                              Requirements and step files
src/                               React + TypeScript UI and player
src-tauri/src/                     Tauri startup, commands, events, tray
crates/speakit-core/               SpeechDocument, text rules, session state
crates/speakit-models/             Catalog, download, capability checks
crates/speakit-audio/              CPAL output, native PCM ring, tested time stretcher
crates/speakit-platform/           Windows/macOS/Linux service adapters
crates/speakit-protocol/           Versioned IPC and native message schemas
workers/speakit-tts/               ONNX inference worker executable
workers/speakit-native-host/       Chrome stdio ↔ local IPC executable
ext/                               MV3 worker, popup, options, picker
benchmarks/                        Listening corpus and resource reports
packaging/                         Per-platform app and host registration
```

These are future paths. Only `spec/` exists during this planning task. Package each worker binary for the target OS/architecture; model weights are separately downloaded and hash verified. A worker may use a shared Rust crate, but it has its own lifecycle and message boundary.

### 5.3 Data flow and responsibilities

```text
Manual text / clipboard ──┐
OS selection capture ──────┼──> Tauri Rust commands ──> one PlaybackSession
Chrome extension ── host ──┘                │                   │
                                 SpeechDocument + text rules  │
                                            │                   ▼
                                    bounded scheduler ──> TTS worker
                                                             │ PCM
React settings/player <── state events <── native audio <── cache
```

- The extension and OS adapters submit plain text plus bounded source metadata. No adapter has an audio player.
- `SpeechDocument` keeps source mapping and sentence state. The scheduler requests one segment at a time up to a bounded lookahead; the worker returns PCM and measured duration.
- Rust owns the audio clock, output device, session state, cache, and cancellation. React renders snapshots and sends explicit commands.
- Do not stream bulk PCM as high-frequency Tauri events to React. The UI receives low-rate status only; native audio consumes native buffers.
- TypeScript, Rust, native host, and extension share a versioned protocol contract and validation fixtures.
- A Tauri plugin is a candidate implementation, not an automatic solution. Validate shortcuts, autostart, tray, positioning, and capabilities on each platform. [Tauri plugins](https://v2.tauri.app/start/)
- No LLM dependency or content-processing service belongs in this architecture.

### 5.4 Native capability gates

| Capability | Windows | macOS | Linux |
|---|---|---|---|
| Global shortcuts | Registered chords, with narrow native fallback only if required | Registered chords; permissions depend on chosen method | X11 or GlobalShortcuts portal where available; desktop may assign final chord |
| Foreground selection | UI Automation with safe copy fallback | Accessibility selected-text APIs with consent, then safe fallback | AT-SPI where exposed; Wayland often needs explicit copy/extension |
| Background/tray | User-session process and tray | Menu bar user-session process | StatusNotifier/AppIndicator where supported; retain a launcher route where absent |
| Floating player | Native no-activate window behavior | Non-activating panel/window behavior | X11 window-manager behavior; Wayland compositor decides placement and focus limits |
| Audio | Native backend selected by tested Rust adapter | Native backend selected by tested Rust adapter | PulseAudio/PipeWire/ALSA path selected by tested adapter |
| Browser IPC | Per-user pipe | User-owned Unix socket | User-owned Unix socket |

**Release gate:** test the requested hotkey, background, overlay, capture, and audio flows on each declared OS/session type. If a Wayland compositor prevents global shortcut registration or arbitrary window placement, report that limitation and provide extension/focused Paste controls. Do not claim pixel-identical or always-on-top parity where the compositor does not permit it. [Global Shortcuts portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html), [Tauri Global Shortcut plugin](https://v2.tauri.app/plugin/global-shortcut/)

---

[Previous](04-screens-and-settings.md) · [Next](06-reading-entry-paths.md)
