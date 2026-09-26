# SpeakIt — Detailed Product and Implementation Specification

Version: 4.0 • Prepared: 2026-09-26 • Status: planning only

**Do not implement this document until the user explicitly asks to start development.**

> **Architecture decision:** The [shared conversation](https://chatgpt.com/share/6ab777c9-6118-83e8-8ed4-fe510c699ce2) was read. SpeakIt targets Windows, macOS, and Linux with Tauri 2, React/TypeScript, and Rust. Only its TTS path applies; the conversation’s LLM and STT design is outside this release. The [Windows prompt review](Windows-Prompt-Feature-Review.md) supplies selected behavior and preferred shortcuts.

## 1. Purpose and source of truth

Build a beautiful, local text-to-speech application with a floating player and Chrome extension. It should read selected or copied text in a crisp, pleasant voice while leaving enough CPU, GPU, and memory for the user's other work.

This is a self-contained specification. It combines the shared Tauri conversation with selected behavior from the pasted Windows v1/v2 documents. Requirements are defined here rather than inherited from those documents.

Inputs reviewed:

- The user's request for attractive UI, efficient local CPU/GPU speech, a Chrome extension, granular steps, and no LLM for now.
- The complete pasted document, “SpeakIt — Spec v2: Model Config UI, Chrome Extension, Floating Player.”
- The original private [ChatGPT conversation](https://chatgpt.com/c/6ab76a7f-19f4-83ee-a304-248787073002) could not be fetched, but its [shared copy](https://chatgpt.com/share/6ab777c9-6118-83e8-8ed4-fe510c699ce2) was read. It explicitly proposes Tauri 2 + React/TypeScript + Rust and local native ONNX TTS. No referenced screenshot was available.
- Primary technical sources listed in §24 were checked while preparing the spec. Proposed budgets and UX decisions are requirements, not measured results.

### 1.1 Interpretation and assumptions

| Topic | Working decision |
|---|---|
| Product name | SpeakIt |
| Desktop platforms | Windows 11 x64, macOS arm64/x64, and named Linux desktop targets; test each release separately |
| Desktop framework | Tauri 2 + React/TypeScript + Rust, following the shared conversation |
| Current development workspace | macOS; Windows and Linux require separate native test environments |
| “No LLM” | No chat model, summarization, rewriting, translation, agent, or cloud language-model dependency; dedicated local neural TTS remains in scope |
| “Long press control” | Support long-press Ctrl explicitly; also allow Alt and shortcut-only activation |
| Initial language validation | English US/UK; other catalog languages only marked supported after testing |
| Default hardware policy | Balanced CPU; optional verified GPU acceleration |
| New read during playback | Replace current reading; no queue in this release |
| Distribution | Private native packages for validated target OSes plus unpacked extension first; public distribution later |

Platform intent and the Tauri stack come from the user and shared conversation. Exact OS versions, architectures, hardware targets, and packaging remain to be verified before implementation.

### 1.2 Priority definitions

- **P0:** Required for the useful desktop core and local TTS.
- **P1:** Required for the full initial product, including the Chrome picker and polished player.
- **P2:** Subsequent enhancement; not a dependency of initial release.
- A checked task in this document would mean verified completion. All tasks below are intentionally unchecked.

## 2. Scope and success criteria

### 2.1 Required outcomes

| ID | Priority | Requirement | Evidence of completion |
|---|---|---|---|
| UI-01 | P0 | Cohesive desktop UI with light/dark themes | Reviewed screenshots at supported sizes and DPI |
| TTS-01 | P0 | Speak manually entered or copied text locally | Works after disconnecting the network |
| TTS-02 | P0 | At least one high-quality downloadable local voice | Listening evaluation and reproducible benchmark |
| PERF-01 | P0 | Bounded resource use | CPU, RAM, VRAM, responsiveness, and buffer report |
| PLAY-01 | P0 | One playback session across all entry paths | Replacement/cancellation integration tests |
| EXT-01 | P1 | Chrome selection and block picking | Site compatibility matrix |
| EXT-02 | P1 | Configurable long-press Ctrl/Alt and scope control | Interaction tests, including ordinary shortcuts |
| BRIDGE-01 | P1 | Local native messaging transport | Reconnect, authorization, and malformed-message tests |
| CAP-01 | P1 | Best-effort auto-capture from desktop applications | Documented supported/unsupported cases |
| PRIV-01 | P0 | No text leaves the machine for speech | Network inspection and offline run |
| ACCESS-01 | P1 | Keyboard and screen-reader access | Manual accessibility review |

### 2.2 Explicitly out of scope

- LLM chat, summaries, semantic rewriting, translation, and AI agents.
- Cloud speech APIs, accounts, billing, telemetry, or analytics.
- Voice cloning, training, celebrity voice imitation, and microphone input.
- OCR, image reading, scanned PDF recognition, and DRM bypass.
- A second audio engine or player inside the extension.
- Mobile clients and multi-device synchronization. Windows, macOS, and Linux desktop are in scope, with platform-specific capability gates.
- Playback queues, batch audiobook production, and audio-file export in the first release.
- A website or marketing landing page; the UI design skill is used for product design guidance.

## 3. UI design direction and skill usage

### 3.1 Selected skill

The `frontend-design` skill from [Anthropic's skills repository](https://github.com/anthropics/skills/tree/main/skills/frontend-design) was installed and read for this specification. Local installation: `/Users/zee/.codex/skills/frontend-design/SKILL.md`.

It is a suitable design-direction skill, not a claim that one objectively “best” UI skill exists. Its principles are adapted to a Tauri desktop product: intentional typography, meaningful visual hierarchy, restrained animation, realistic content, and one memorable visual element. No UI code or prototype has been implemented.

### 3.2 Concept: a quiet reading desk

The text being read is the center of the experience. The waveform is the signature visual. Navigation, model management, and hardware settings should remain calm and easy to scan.

Keep the warm surfaces and violet accent requested in the pasted design. Avoid filling every screen with identical cards, decorative statistics, or gradients. Use an uninterrupted reading area, grouped settings rows, and cards only for independently actionable models or readings.

### 3.3 Design tokens

All sizes are device-independent pixels for desktop and CSS pixels for extension UI.

| Token | Light | Dark | Purpose |
|---|---|---|---|
| Page | `#F7F6F3` | `#17181C` | Main background |
| Surface | `#FFFFFF` | `#222329` | Panels and model cards |
| Sidebar | `#F2F1EE` | `#1D1E23` | Navigation |
| Text primary | `#1A1A1A` | `#F1F0F4` | Main text |
| Text secondary | `#6B6862` | `#B8B6C2` | Supporting text |
| Accent | `#7C5CFF` | `#A28BFF` | Selection, waveform, focus |
| Accent button | `#6242D6` | `#B5A2FF` | Primary button; white/light or dark text as appropriate |
| Border | `#E5E3DE` | `#3B3D46` | Group separation |
| Success | `#2E7D5B` | `#7ACBA7` | Successful state |
| Error | `#B63229` | `#FF9C91` | Error text and icons |

Typography:

- UI: Segoe UI Variable, fallback Segoe UI; extension fallback `system-ui`.
- Reading text: Georgia, fallback serif, user-selectable sans-serif alternative.
- UI text 14; secondary text 12; section title 20; page title 28; reading text 18 by default.
- Reading text adjustable 14–28 with 1.55 line height; target 60–75 characters per line.
- Use sentence case. Do not rely on color, emoji, or a tiny icon to convey a state.

Geometry and motion:

- Four-pixel spacing grid; principal gaps 8/12/16/24/32.
- Panel radius 14; button radius 10; input radius 8; pills fully rounded.
- Main content padding 28; sidebar width approximately 220.
- One subtle shadow for floating surfaces; no permanent shadow on every settings row.
- Interaction transitions 120–180 ms. Waveform capped at 30 fps while playing.
- Reduced-motion mode stops breathing, gliding highlights, and decorative transitions.
- Paused player uses a static waveform and clear “Paused” label by default to save resources.

### 3.4 Layout and review criteria

Preferred settings window 1080 × 740; minimum 860 × 620. Smaller work areas scroll the content rather than clipping controls. Test 100%, 125%, 150%, and 200% DPI.

```text
┌───────────────────────────────────────────────────────────────────┐
│ SpeakIt                                            Window controls│
├──────────────────┬────────────────────────────────────────────────┤
│ Read             │ Read                                           │
│ Voices           │ Paste text and listen                          │
│ Shortcuts        │ ┌────────────────────────────────────────────┐ │
│ Extension        │ │ Editable text                              │ │
│ Settings         │ │                                            │ │
│                  │ └────────────────────────────────────────────┘ │
│                  │ 1,240 words    About 7 min     [Listen]         │
│                  │                                                │
│                  │ Voice: Heart    Device: CPU    [Change voice]  │
│ Local audio      │ Continue reading / recent items, if enabled    │
└──────────────────┴────────────────────────────────────────────────┘
```

Before implementation, produce static designs for Read, Voices, Settings, extension popup, picker confirmation, and both player sizes. Review real long labels, missing voices, empty state, download failure, and disconnected extension—not only the happy path.

Design critique already applied to this spec: removed the decorative streak/hero dashboard as a release dependency, retained the source palette, centered the actual reading task, and made the waveform the main expressive element. History and optional statistics remain P2.

## 4. Screen-by-screen behavior

### 4.1 Read screen

1. Show an editable multiline text area with placeholder “Paste text to read aloud.”
2. Provide Paste, Clear, and Listen actions; Paste reads the clipboard only when clicked.
3. Show word count and an explicitly approximate duration.
4. Disable Listen when text is empty or whitespace-only.
5. On Listen, validate length, create a session, and display the floating player.
6. Keep the draft available while the app remains open. Do not persist it across exits by default.
7. If already reading, the button label becomes “Read this instead.”
8. Show actual selected voice and execution device, with a link to change them.

### 4.2 Voices screen

1. Display model cards with name, description, languages, installed/download size, license link, and supported devices.
2. Clearly distinguish download size, disk size, and observed memory usage.
3. Mark recommended voices only after evaluation; no invented quality grades.
4. Allow Download, Cancel, Retry, Select, Preview, Update, and Delete according to state.
5. Expand configuration beneath the selected model only.
6. Show language selector only when multiple tested languages exist.
7. Filter voices by language; show speaker selector only for multi-speaker models.
8. Provide explicit Preview buttons. Never play audio automatically after downloading.
9. A preview pauses the current reading, uses the same audio output, and restores the prior session in paused state afterward.
10. Put synthesis parameters in Advanced, with a clear cache-invalidation explanation.
11. Put ordinary playback speed and volume in a separate playback section.
12. Do not let selecting a model discard a functioning engine until the replacement has loaded successfully.

Model status uses separate dimensions to avoid impossible combined states:

| Dimension | Values |
|---|---|
| Installation | Missing, Downloading, Verifying, Installed, Failed |
| Activation | Inactive, Loading, Active, LoadFailed |
| Update | Current, UpdateAvailable |

Exactly one engine is active after successful initialization; zero is valid while none are available. An active installed model can also have an update available. Failed updates leave the old version usable.

### 4.3 Shortcuts screen

Each row includes action, current chord, Record, and Reset. Recording provides Cancel and a visible listening state. Reject bare keys for global actions; identify duplicate app bindings; report registration failures from the OS. Do not claim to detect every shortcut used by every other application.

### 4.4 Extension screen

Show bridge installation health, last connection, authorized extension identity, connection status, Revoke, installation instructions, and Test connection. “Not connected” must not be reported as “not installed” without evidence. Display separate checks for extension connection, native host registration, and desktop readiness.

### 4.5 Settings screen

Sections: Appearance, Playback, Performance, Background behavior, Privacy and storage, Diagnostics.

| Setting | Default | Behavior |
|---|---|---|
| Theme | System | Light/dark manual override |
| Reading font size | 18 | Applies to expanded reading view |
| Playback speed | 1.0× | Range 0.5–3.0×, step 0.1 |
| Volume | 80% | Range 0–100%; independent of OS master volume |
| Execution device | CPU | Auto/GPU available after compatibility checks |
| Resource profile | Balanced | Eco/Balanced/Performance |
| Start at login | Off | Per-user startup mechanism on each supported OS |
| Keep running when windows close | On | Closing settings leaves tray/player available |
| Start minimized at login | Off | Enabled only when startup is enabled |
| Pause global hotkeys | Off | Also available in tray menu |
| Remember reading history | Off | Opt-in local persistence |
| Persistent audio cache | Off | Memory cache only initially |
| Diagnostic logging | Errors | No source text or page titles by default |

Explicit Exit always stops playback and terminates all owned workers. Closing the main window exits when background mode is off. The player close action stops and hides the current reading but does not independently terminate the application.

## 5. Architecture and platform boundaries

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

## 6. Entry paths and exact user journeys

### 6.1 Path A: clipboard hotkey — first delivery

1. User selects text in another app and copies it themselves.
2. User presses `Ctrl+Shift+R`.
3. SpeakIt reads Unicode text from the clipboard with a short bounded retry for clipboard contention.
4. Empty, non-text, or over-limit content produces a specific message.
5. Core starts a reading and the player appears without stealing typing focus.
6. SpeakIt does not modify the clipboard in this path.

The path depends on the source application permitting copy and the desktop allowing clipboard access when the shortcut fires. Offer a focused Paste action or Chrome extension route where background clipboard reads are restricted.

### 6.2 Path B: selected text without manual copy

1. User selects text and invokes the platform-mapped Read selection shortcut.
2. The platform adapter tries a supported accessibility selection API: Windows UI Automation, macOS Accessibility with consent, or Linux AT-SPI where exposed.
3. If the API fails, consider a synthetic Copy only when the OS permits it and all physical trigger modifiers have been released within a short timeout.
4. Preserve and restore clipboard contents only when the platform can do so safely; never overwrite a newer user copy. If formats or ownership cannot be preserved, skip this fallback.
5. If capture is unavailable, explain the manual-copy shortcut or Chrome extension route. Whole-document reading is always a separate explicit action.

Windows must verify that synthetic Copy does not open Chrome DevTools or leave modifiers stuck. macOS must handle Accessibility permission denial. Linux Wayland must not claim arbitrary cross-application selection access. Each result belongs in the compatibility matrix.

### 6.3 Path C: Chrome selection

1. User selects page text.
2. User chooses “Read selection with SpeakIt” from the context menu, a configured shortcut, or the popup.
3. Extension captures only the requested selection and minimal source metadata.
4. Worker establishes the native connection, validates the request, and sends it.
5. Desktop accepts or rejects; acceptance means the request is queued for preparation, not that sound already started.
6. Native floating player owns playback from that point onward.

### 6.4 Path D: Chrome element picker

1. User holds the configured Ctrl/Alt trigger on a permitted page.
2. After 450 ms, a visible cue arms the picker.
3. User releases the modifier; picker remains latched for selection.
4. Hover highlights a readable paragraph or block.
5. Mouse wheel or dedicated keyboard controls widen/narrow the candidate scope.
6. Clicking the candidate freezes selection and shows Listen, Copy text, Back, and Close.
7. Listen sends a snapshot to the app; Copy text writes exactly the previewed extraction to the clipboard after an explicit click.
8. Back returns to hover selection. Close or Escape exits completely.

Latching resolves the ambiguity in the pasted flow: a user must be able to release Ctrl/Alt and still click Listen. The Copy text action is included explicitly because the user referred to copy-text extension behavior.

## 7. Local TTS strategy

### 7.1 Model shortlist and selection policy

| Candidate | Intended role | CPU/GPU policy | Release condition |
|---|---|---|---|
| Kokoro-82M | Primary quality candidate | CPU baseline; GPU only for tested export/provider combinations | Pass pronunciation, startup, resource, and listening gates |
| Piper | Lightweight alternative | CPU first | Pass quality checks and engine/voice distribution-license review |
| Platform system voice, where available | Optional zero-download fallback | OS-managed local speech | Test whether each OS exposes offline voices and controllable PCM; do not promise on Linux |

Kokoro has 82 million parameters and Apache-2.0 model weights. That makes it a reasonable compact quality candidate, not proof of performance on the user's hardware. Its official example emits 24 kHz audio; use each artifact's actual declared format. [Model card](https://huggingface.co/hexgrad/Kokoro-82M)

The currently maintained Piper repository describes a local engine and carries GPL-3.0 licensing. Review the exact engine integration and each voice's license before packaging; do not assume all voice files share one license. Piper remains conditional until those decisions are recorded. [Repository](https://github.com/OHF-Voice/piper1-gpl)

### 7.2 Quality evaluation

Prepare a fixed 30-passage corpus covering:

- Short sentences and long paragraphs.
- Questions, dialogue, commas, abbreviations, dates, prices, and decimals.
- Personal names, technical terms, URLs, acronyms, and code-heavy prose.
- Long words, emojis, punctuation-only input, and mixed-script text.
- English accents separately; Hindi and other languages only if explicitly included in the supported catalog.

Compare at least three available Kokoro voices, one suitable Piper voice if included, and any verified OS built-in fallback. Review naturalness, crisp consonants, intelligibility, pronunciation, pauses, missing/repeated words, clipping, and fatigue over 15 minutes.

Proposed acceptance: average at least 4/5 for clarity and listening comfort from three listeners on the agreed corpus; no systematic omitted/repeated words or audible clipping. Record listeners, equipment, text, model hash, and settings. If only the owner evaluates a private build, label that limitation rather than calling it a broader study.

Quantized and full-precision artifacts must be compared directly. An int8 file may reduce storage without improving every hardware path; do not make it the recommendation solely because it is smaller.

### 7.3 CPU and GPU requirements

- CPU mode must work without CUDA or a discrete GPU.
- GPU selection is explicit and reversible; CPU remains available.
- Auto mode benchmarks a small local sample with consent before selecting a device.
- Record model export, runtime version, execution provider, device, and any CPU fallback nodes.
- Candidate GPU providers include CUDA for compatible NVIDIA hardware, DirectML for compatible Windows GPUs, CoreML on eligible Macs, and OpenVINO where the exact Intel/export/OS combination is tested. Provider availability is not model compatibility; validate actual execution and CPU fallback nodes. Do not import the shared conversation’s `llama.cpp` GPU mapping into ONNX TTS. [Provider documentation](https://onnxruntime.ai/docs/execution-providers/)
- Never label a session “GPU” merely because the machine has a GPU.
- If GPU loading fails, offer CPU fallback and show the reason. Do not silently download large runtimes.
- On battery, Auto prefers Eco CPU unless the measured GPU path is more efficient.
- No model training, background benchmarking, or speculative model preloading.

### 7.4 Model capabilities and catalog

Catalog entries must include:

| Group | Fields |
|---|---|
| Identity | Catalog schema version, model ID, artifact version, engine adapter ID |
| Files | HTTPS URL, exact bytes, SHA-256, relative installation path |
| Compatibility | Runtime requirements, tested providers, architecture, phonemizer identity |
| Audio | Sample rate, channels, output sample format |
| Text | Languages, tokenizer identity, exact model token/phoneme limit |
| Voices | Single/list/system mode, IDs, labels, language, sample text |
| Parameters | Typed ID, label, min/max/step or enum values, default, help, cache effect |
| Rights | Engine and artifact license URLs, required attribution |
| Evidence | Benchmark report ID and recommendation status |

Do not hardcode “54 voices,” a fixed language count, a token limit, or “one voice per Piper file” as a universal engine rule. Catalog the actual artifact, including multi-speaker files when present.

The UI renders existing parameter types from metadata: number → slider plus numeric input; boolean → switch; enum → dropdown. Adding a supported parameter requires no new UI control code, but the engine adapter must actually understand and map it. Unknown parameters are rejected or hidden with diagnostics; a JSON edit cannot create an unsupported engine capability.

### 7.5 Download and activation lifecycle

1. User explicitly chooses Download after seeing size and license information.
2. Validate free disk space including temporary-file overhead.
3. Download into a versioned temporary directory with cancellation and bounded concurrency.
4. Resume only when server validators and range support establish that the partial file still matches.
5. Verify every hash before activation; never execute code from the model catalog.
6. Atomically mark the version installed after all required files verify.
7. Preserve the prior working version through failed downloads or updates.
8. Load on Select; provide explicit Preview after successful activation.
9. Deleting the active version first switches to an available fallback or stops playback with confirmation.
10. Model updates do not occur during a reading without an explicit user action.

## 8. Resource controls and measurable performance

All numbers below are proposed engineering targets. No benchmark has been run for this planning task. Final supported hardware requires measurements and a published compatibility table.

### 8.1 Reference hardware classes

| Class | Proposed minimum test setup | Mode |
|---|---|---|
| Modest laptop | Four physical CPU cores, 8 GB RAM, SSD, no discrete GPU | Eco and Balanced CPU |
| Typical desktop | Six or more CPU cores, 16 GB RAM | Balanced CPU |
| GPU desktop | Compatible NVIDIA GPU with at least 4 GB VRAM, 16 GB RAM | Explicit supported GPU path |
| Integrated GPU | Supported Intel/AMD Windows or Linux GPU, or Apple Silicon | Optional provider validation by exact model/export |

Record exact CPU/GPU model, OS build, driver, power mode, RAM, runtime, and model hash. Do not claim universal support from these broad classes alone.

### 8.2 Resource profiles

Let `P` be detected physical CPU cores, conservatively falling back to logical cores if detection fails.

| Control | Eco | Balanced — default | Performance |
|---|---|---|---|
| Inference threads | `max(1, min(2, P-1))` | `max(1, min(4, floor(P/2)))` | `max(1, P-1)` |
| Concurrent synthesis jobs | 1 | 1 | 1 initially |
| Desired lookahead at current speed | 8 seconds | 15 seconds | 30 seconds |
| Maximum buffered PCM | 32 MiB | 64 MiB | 128 MiB |
| Worker priority | Below normal | Below normal | Normal |
| Idle model unload | 2 minutes | 5 minutes | 10 minutes |
| Optional waveform motion | Minimal | 30 fps playing | 30 fps playing |

Thread counts are controls, not hard CPU-percentage guarantees. Cap library-owned thread pools as well as application workers. ONNX Runtime exposes thread configuration and spinning controls; explicitly disable idle spinning where supported and verify the chosen binding's behavior. [Threading documentation](https://onnxruntime.ai/docs/performance/tune-performance/threading.html)

### 8.3 Acceptance targets

| Metric | Target on declared reference hardware |
|---|---|
| Warm first audible output | p95 ≤ 1.5 s for a 20–40-word English opening sentence |
| Cold first audible output | p95 ≤ 5 s with installed model, excluding download |
| CPU real-time factor | ≤ 0.7 at 1× on the supported Balanced CPU reference |
| GPU real-time factor | Target ≤ 0.35 on the validated GPU reference |
| Stop/pause response | ≤ 150 ms perceived audio response |
| UI interaction latency | p95 ≤ 100 ms during synthesis |
| Idle CPU, no work | Average < 1% of total machine capacity over 60 s |
| Active CPU | Target average ≤ 50% total capacity in Balanced; retain responsive foreground apps |
| Desktop + inference private memory | Target ≤ 1.5 GiB steady state for recommended Kokoro CPU configuration |
| GPU memory | Target ≤ 2 GiB dedicated allocation for recommended GPU configuration |
| Hidden player | No continuous waveform redraw |
| Long reading | No buffer underruns in 30 min at sustainable speed |

Real-time factor = synthesis wall time / produced audio duration. At playback speed `r`, sustained reading requires approximately `RTF < 1/r`; a model passing at 1× may fail at 3×. Show “This voice cannot keep up at this speed” with options to reduce speed, prebuffer longer, or change voice. Do not silently exceed the resource profile.

### 8.4 Scheduler, pressure, and cancellation

- Generate the opening sentence first; never synthesize a whole long article before playback.
- Use one bounded work queue and stop lookahead when either duration or byte budget is reached.
- Resume synthesis below a low-water mark of half the desired lookahead.
- Prioritize a user seek over speculative future segments.
- Discard stale results using session ID and configuration generation.
- On Stop, silence output immediately and cancel scheduled work. If native inference ignores cancellation, discard its result and terminate/restart the worker after a bounded two-second grace period.
- If memory pressure rises, release unused PCM and reduce lookahead before considering a fallback model.
- Keep only one loaded model in normal operation. Switching may unload the old worker first on low-memory machines, accepting a loading delay.
- Process isolation provides crash recovery; soft memory targets are not guaranteed allocation ceilings. Apply provider limits where available and report when unavailable.
- A GPU can briefly reach high utilization during a kernel. Bound duty cycle and memory; do not promise a universal GPU-percent cap.

## 9. Text pipeline and document index

### 9.1 Deterministic processing

1. Accept plain text plus source metadata.
2. Validate a 200,000 UTF-16-code-unit input limit consistently in extension/React JavaScript and Rust (which must count UTF-16 code units explicitly, not bytes or Unicode scalar values).
3. If over limit, ask whether to read the first portion; never silently truncate. Cut at a valid Unicode and preferably sentence boundary.
4. Normalize line endings, control characters, and repeated layout whitespace while retaining paragraph boundaries.
5. Preserve the original source string and a normalized-to-original span mapping.
6. Segment using deterministic sentence rules, not an LLM.
7. Handle abbreviations, decimals, quotes, and Unicode punctuation through explicit fixtures.
8. Split sentences exceeding the model's actual token/phoneme limit at sensible phrase boundaries.
9. Phonemize/tokenize using the model's required pipeline; never feed an incompatible phoneme alphabet.
10. Produce audio asynchronously and record actual sample lengths.

No summarization, paraphrasing, content deletion based on meaning, or inferred language rewriting. Optional URL/code reading behavior must be an explicit deterministic setting.

### 9.2 Data contracts

| Record | Required fields |
|---|---|
| SpeakRequest | Request ID, plain text, source kind, optional source title/origin, optional language hint |
| SpeechDocument | Document ID, original text, normalized text, span mapping, ordered segments |
| Segment | ID, sentence ID, original/normalized ranges, model input, synthesis state, PCM key, sample count |
| SourceReference | Source kind, display name, optional tab/frame/window identity, optional origin |
| PlaybackSnapshot | Session ID, status, current segment, source position, speed, volume, actual/estimated duration |

Source metadata is descriptive only. Page text, URLs, and titles are never executable commands.

### 9.3 Timing and cache rules

- Store source-audio sample positions at normal synthesis pace; do not store only wall-clock progress.
- Speed changes preserve source position while changing future output timing.
- Progress follows consumed audio plus output-device latency correction, not a periodic timer alone.
- Total duration is estimated until every segment has known sample counts.
- Cache keys include model/artifact version, voice, language, phonemizer version, normalized text, synthesis parameters, and audio format.
- Playback speed and volume do not invalidate synthesis cache.
- Advanced synthesis pace or expressiveness changes invalidate affected audio and create a new configuration generation.
- Keep segment metadata after PCM eviction; regenerate evicted audio on seek.
- Memory cache is bounded LRU. Persistent disk caching is opt-in, with a default 512 MiB quota if enabled later.

## 10. Playback behavior and floating player

### 10.1 Session states

`Idle → Preparing → Buffering → Playing ↔ Paused → Completed`

Any active state may transition to `Stopping → Idle` or `Error`. Device changes introduce a visible `Recovering` state. Distinguish Loading voice, Preparing text, and Buffering audio in user-facing labels.

A new validated reading replaces the old session, cancels old work, and receives a new ID. Empty or rejected input must not stop a working reading. Preview is a temporary use of the same player/output, never simultaneous playback.

### 10.2 Compact player

Target approximately 520 × 112, adapting to display scaling and available width.

```text
┌───────────────────────────────────────────────────────────────┐
│ example.com                  Heart       CPU        Expand  × │
│ [Back 10] [Play/Pause] [Forward 10]    [1.0×]       [Volume]    │
│ ▂▃▅▃▂▅▆▃──── waveform / seek ───────────────  02:14 / ~11:38 │
└───────────────────────────────────────────────────────────────┘
```

Requirements:

- One native player for every entry path; no competing in-page playback widget.
- Appears without stealing source-app focus.
- Topmost behavior toggle; no taskbar clutter in compact mode.
- Dedicated drag handle; remember position per monitor and clamp after monitor removal.
- Close stops and hides. Collapse preserves reading. Pause keeps position.
- Speed control supports direct numeric/menu selection; a right-click-only action is insufficient.
- Source chip focuses the original tab/window only on an explicit click and only if still identifiable.
- If the source disappears or navigates, continue speaking the accepted snapshot and mark the source unavailable.
- No remote favicon fetch; prefer a local generic icon or already available browser-provided data.

### 10.3 Expanded player

Target approximately 620 × 500 with a resizable reading pane.

- Display the actual normalized reading text with the current sentence highlighted.
- Click or keyboard-activate a sentence to seek to its beginning.
- Virtualize long text where necessary.
- Auto-scroll only while Follow reading is enabled; manual scrolling temporarily disables it and shows “Return to current sentence.”
- Provide keyboard focus intentionally when opened by the user. Non-activation must not make the expanded player inaccessible.
- Sentence highlighting is required. Exact word highlighting is deferred unless the engine exposes reliable alignment; do not fabricate timestamps.

### 10.4 Seeking and duration

- Skip ±10 seconds in source-audio time, clamp to valid range, and indicate sentence snapping if enabled.
- Known synthesized intervals allow precise seeks.
- Unsynthesized intervals show approximate progress and buffer after selection; prioritize the corresponding sentence.
- Distinguish elapsed source time from estimated remaining listening time at the current rate.
- Reading estimate: `words / (180 × rate)` minutes, explicitly approximate. For languages without reliable word boundaries, use a tested locale-specific estimate or omit it.

### 10.5 Waveform

- Compute a downsampled static amplitude envelope off the audio thread as segments complete.
- Unknown audio renders as a dim placeholder; never invent an exact waveform for unsynthesized text.
- Use up to three subtle layered curves, with the played region clearly distinguishable.
- Tap output amplitude after time stretching into a preallocated ring buffer.
- Audio callback must make no heap allocations, acquire no contended locks, and never touch React, WebView, or the Tauri event system.
- React renders waveform updates at most 30 fps while visible; use Canvas or a bounded SVG path and measure frame time/allocations on each webview. Keep amplitude transfer bounded and off the audio callback.
- No redraw timer while hidden. Paused view settles to a static state.
- Expose an accessible seek slider with text time values independent of the visual waveform.

### 10.6 Audio recovery

On device removal, pause, rebuild output on a valid device, and resume from the last reliable source position. Show recovery status; if recovery fails, remain paused with “Choose an audio device.” Test headphone unplugging, Bluetooth changes, sleep/wake, and sample-rate changes. Never loop indefinitely or restart the article from the beginning.

## 11. Global shortcuts and background behavior

### 11.1 Preserve the preferred shortcut set

Keep the action layout from the pasted Windows prompt. Most of these bindings were already in our spec; they do not require replacing the playback or capture architecture. Keep our additional Speak clipboard shortcut as the reliable fallback.

| Action | Windows default / Linux suggested binding | Proposed macOS equivalent |
|---|---|---|
| Speak clipboard | `Ctrl+Shift+R` | `Command+Option+R` |
| Read selected text | `Ctrl+Shift+S` | `Command+Option+S` |
| Play/pause | `Ctrl+Shift+Space` | `Command+Option+Space` |
| Skip back 10 seconds | `Ctrl+Shift+Left` | `Command+Option+Left` |
| Skip forward 10 seconds | `Ctrl+Shift+Right` | `Command+Option+Right` |
| Speed up | `Ctrl+Shift+Up` | `Command+Option+Up` |
| Slow down | `Ctrl+Shift+Down` | `Command+Option+Down` |
| Stop and hide player | `Ctrl+Shift+X` | `Command+Option+X` |
| Stop and hide while player has focus | `Escape` | `Escape` |

The macOS mappings are proposals, not tested conflict-free bindings; allow the original Control+Shift bindings if the user prefers. Linux shortcuts are suggestions because the desktop/portal may assign or require approval of the final chord. The cross-platform Tauri shell is specified in §5; every actual chord remains subject to OS registration and conflict testing.

### 11.2 Exact behavior

- Global shortcuts work while another application is focused when the platform permits registration.
- Read selection invokes only supported capture; unavailable capture explains the clipboard/extension fallback.
- Speak clipboard uses the explicit clipboard-read path. On platforms that deny background clipboard access, offer a focused Paste action or the extension instead.
- Play/pause toggles an existing session; it never implicitly rereads old clipboard content.
- Speed shortcuts change the current rate by 0.1 within 0.5–3.0, matching our existing control. They do not resynthesize audio.
- Skip uses our source-time timeline and existing sentence-snap setting. Keep precise seeking available rather than making snapping mandatory.
- Outside a reading session, playback-only actions do nothing; they do not open a window or capture text.
- Escape is local to the focused player. In a shortcut recorder it cancels recording; in the Chrome picker it cancels picking. Never reserve Escape globally.
- Ignore auto-repeat for Read selection, Speak clipboard, and Play/pause. Debounce held skip/speed keys to at most four actions per second.
- Application-global shortcuts and the Chrome long-press picker are separate features. A normal chord must cancel a pending picker hold timer.

### 11.3 Registration, customization, and verification

Use the Tauri global-shortcut plugin where it satisfies the required chord on the target OS; use a narrow native adapter only for verified gaps. Introduce low-level hooks only for behavior that actually needs them, with fast callbacks and explicit cleanup. Do not inherit the pasted requirement that every shortcut use a Windows keyboard hook. [Tauri Global Shortcut plugin](https://v2.tauri.app/plugin/global-shortcut/)

- Provide a recorder per action, cancel/reset, readable platform key labels, and duplicate-binding checks.
- Attempt native registration and report actual conflicts or permission denial; do not claim to enumerate every shortcut owned by another app.
- Store requested and effective bindings separately if the desktop remaps them.
- Pause hotkeys releases global bindings and allows another application to use them; re-enabling reports any newly introduced conflicts.
- Re-register after session/backend reconnection when required, without stealing an existing binding or repeatedly prompting for permission.
- Test after 15 minutes idle, sleep/wake, permission changes, and source-app switching.
- Verify triggering or using compact playback controls does not steal source-editor typing focus on supported platforms.

### 11.4 Background behavior retained

Tray/menu actions: Open SpeakIt, Play/Pause, Stop, Speak clipboard, Pause hotkeys, Settings, Exit. Do not read clipboard contents on a timer. Single-instance startup forwards an activation request to the existing instance.

“Background service” means a lightweight process in the signed-in user's desktop session, not a privileged system service. Keep running when settings closes, Start at login, and Start hidden are separate settings. Explicit Exit stops playback and owned workers. A desktop with no supported tray needs a reliable launcher/window route to reopen and quit; never leave an unreachable hidden process.

See the [selective comparison](Windows-Prompt-Feature-Review.md) for platform constraints and features recommended for later adoption.

## 12. Chrome extension specification

### 12.1 Permissions and enablement

Two supported access modes:

1. **On demand:** `activeTab` and scripting access after an explicit extension action or command; selection/context-menu reading and click-to-arm picker.
2. **Always ready on approved sites:** user-granted optional HTTP/HTTPS host access, with registered content scripts so long-press detection is already present.

Do not require all-site access merely to use selection reading. Explain that a long press cannot be detected before a content script is permitted and loaded. `activeTab` is temporary access from supported user gestures. [Chrome activeTab documentation](https://developer.chrome.com/docs/extensions/develop/concepts/activeTab)

Proposed permissions: `nativeMessaging`, `storage`, `activeTab`, `scripting`, `contextMenus`; optional HTTP/HTTPS host permissions. Add further permissions only for a concrete tested need.

Master Off disables input listeners and active pickers, but retains the lightweight control channel needed to receive an enable/settings change. This corrects the impossible requirement that an off extension have no listeners of any kind.

Persist options in `chrome.storage.local`, not sync, to preserve the local-only preference. Incognito disabled by default; if explicitly enabled later, never persist that browsing text/history.

### 12.2 Popup and options

Popup contains:

- Master On/Off.
- Current-site status and access request/revoke action.
- Read selection and Pick text buttons.
- Trigger summary with link to options.
- Desktop connection status and Open SpeakIt/Test connection action.
- Concise unsupported-page or permission error.

Options contain trigger, hold duration 300–1000 ms with 450 ms default, per-origin enablement, shortcut help, and reset. Per-site disable immediately cancels an active picker in that origin.

### 12.3 Trigger state machine

`Disabled → Idle → Holding → Picking → Confirming → Sending → Idle`

Cancel paths return to Idle; Back from Confirming returns to Picking.

Rules:

1. Default provisional gesture is long-press left Ctrl to reflect the user's request. Offer Alt and shortcut-only alternatives.
2. Ignore auto-repeat, IME composition, and events in password fields or editable controls.
3. If another key is pressed during Holding, cancel the timer and preserve the normal shortcut.
4. Ctrl+C, Ctrl+V, Ctrl+L, Ctrl+click, Alt+Tab, and browser zoom must keep their normal behavior outside an already armed picker.
5. Ignore AltGr/composed right-Alt combinations by default; they are needed for normal international typing.
6. Arm after threshold, show a cue, then latch on release. Do not require the user to keep holding the modifier while operating buttons.
7. Blur, tab hidden, navigation, Escape, master disable, and 15 seconds inactivity cancel.
8. Register capture-phase handlers, but acknowledge that a content script cannot guarantee precedence over browser/OS shortcuts or hostile pages.
9. If Alt release still opens browser chrome on a tested configuration, direct the user to Ctrl or shortcut-only mode; `preventDefault` is not a universal OS-menu guarantee.
10. A Chrome command also arms the picker for keyboard users and permission-limited mode.

### 12.4 Candidate discovery and scope

- Hit-test at the pointer and walk ancestors to a readable text block.
- Prefer paragraph, list item, blockquote, heading, or article content.
- Exclude hidden, inert, password, form-entry, navigation, and extension-owned nodes by default.
- Accept short headings and labels when deliberately selected; the pasted fixed 80-character minimum is a heuristic, not an absolute ban.
- Avoid choosing the full body immediately when a smaller meaningful block exists.
- Show element role/tag, text count, and scope indicator.
- Wheel up widens to a readable ancestor; wheel down returns through candidate history. Do not choose an arbitrary child.
- Intercept wheel only during active picking. Restore normal page scrolling immediately on exit.
- Keyboard equivalents: Up widens, Down narrows, Enter confirms, Escape exits; provide visible instructions.
- Cache candidate text during hover and throttle hit-testing with animation frames; do not scan the whole DOM continuously.

### 12.5 Highlight and confirmation UI

- One extension host and shadow root per participating frame.
- Fixed-position overlay, pointer-transparent outside actionable controls.
- Two-pixel accent border, subtle fill, 6-pixel radius, position updated on scroll/resize.
- Read live bounds; avoid changing classes or styles on the page's own elements.
- Shadow DOM reduces style collisions but is not a security boundary or a guarantee against transformed roots, fullscreen/top-layer UI, or deliberate page interference.
- Confirmation panel is clamped to the viewport and flips placement when needed.
- Show a short extracted-text preview, word/character count, approximate duration, and Listen/Copy text/Back/Close.
- Freeze the captured snapshot on confirmation; if the source changes, continue with that preview or explicitly refresh it.
- Consume relevant pointer/click events only while selecting, to prevent accidental links from opening. Include `click`/`auxclick` behavior in tests, not only `mousedown`.
- Copy success shows “Copied”; failure offers manual selection of the preview. No silent clipboard writes.

### 12.6 Extraction and page limitations

Extract visible text from the live rendered document using a tested traversal or live `innerText` strategy. Do not assume `innerText` on a detached clone preserves visibility and layout: detached elements can behave like `textContent`. [MDN innerText documentation](https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement/innerText)

Exclude script/style/noscript content, extension UI, hidden/ARIA-hidden nodes, and ignored site regions. Preserve meaningful headings, list separation, and paragraph boundaries. Layout inspection belongs in the extension; cleaning and segmentation belong in Core.

Supported: ordinary permitted HTML text, selections, readable blocks, and accessible open shadow roots where tested. Unsupported or conditional: browser internal pages, Chrome Web Store pages, built-in PDF viewer, canvas-only text, closed shadow roots, inaccessible frames, and permission-denied origins. Provide manual copy plus `Ctrl+Shift+R` as the fallback when copying is permitted.

### 12.7 Frames and multiple tabs

- Inject only into frames for which permission and browser rules allow it; `all_frames` does not bypass origin access.
- Each frame extracts its own text.
- Route coordination through extension runtime messages with sender tab ID, frame ID, and document ID, rather than trusting page `postMessage` payloads.
- Keep a single active picker owner per tab; pointer entry into a child frame transfers ownership and clears competing highlights.
- Prefer frame-local highlights to fragile cross-origin rectangle translation.
- Cancel stale requests on frame navigation; no reading text from a replacement document using an old frame reference.
- Multiple tabs may request reads, but the native app retains one session. A late state event must not update an unrelated tab's request UI.
- If an inaccessible child frame is under the pointer, show a limitation rather than pretending to read it.

Chrome documents isolated content-script execution and frame injection behavior; implementation must follow actual host-permission and frame rules. [Content-script documentation](https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts)

## 13. Native messaging bridge and protocol

### 13.1 Transport

Use `chrome.runtime.connectNative("com.speakit.bridge")`. Chrome communicates with the host using native-endian length-prefixed UTF-8 JSON; Windows/macOS/Linux release architectures here are little-endian. Decode explicitly per target architecture, not by assuming the protocol itself says little-endian. Chrome's documented limits are 1 MB host-to-browser and 64 MiB browser-to-host, correcting the pasted reversed limit. Content scripts communicate through the extension worker. [Native messaging documentation](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)

SpeakIt imposes stricter independent limits: 2 MiB serialized UTF-8 per incoming `speak` message, 64 KiB per outgoing state/control message, and the separate 200,000-code-unit text cap. Check actual encoded JSON size, including escaping and metadata, before allocation and send.

No loopback HTTP/WebSocket server is needed for this release. Native host stdout contains protocol bytes only; diagnostics go to redacted stderr/log files.

### 13.2 Authorization and lifetime

- Host manifest contains the expected extension origin allowlist.
- Validate Chrome-provided caller origin rather than trusting an `extId` supplied in JSON.
- Desktop IPC is scoped to the current user: a restricted named pipe on Windows or a user-owned Unix socket on macOS/Linux. Validate the installed host handshake and peer where the OS supports it.
- Native messaging and pipe permissions reduce exposure; they do not defend against all malicious software already running as the same user.
- First use asks the user to allow the extension in a focusable desktop prompt. Do not accept `speak` while approval is pending.
- Deny and Revoke close authorized connections and reject future requests until explicitly allowed again.
- Host may start the installed app from a fixed validated installation path; never execute a path or command from page input.
- Startup wait is bounded to five seconds, then returns a recoverable error.
- Treat extension workers and connections as restartable. Reconnect on demand and query a snapshot instead of retaining authoritative state in the worker. [Service-worker lifecycle](https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/lifecycle)
- Use a bounded reconnect backoff; never create an infinite rapid relaunch loop.

### 13.3 Message schema

Every message includes protocol version `v` and type `t`. Requests include an opaque ID. Validate message types, field types, finite numeric values, ranges, and allowed enum members.

Example request:

```json
{
  "v": 1,
  "t": "speak",
  "id": "req-123",
  "text": "The text explicitly selected by the user.",
  "source": {
    "kind": "chrome",
    "origin": "https://example.com",
    "title": "Example article"
  },
  "languageHint": "en-US"
}
```

Example acceptance:

```json
{
  "v": 1,
  "t": "speak.accepted",
  "id": "req-123",
  "sessionId": "session-456",
  "status": "preparing",
  "estimatedDurationMs": null,
  "durationIsFinal": false
}
```

| Type | Direction | Purpose |
|---|---|---|
| hello / hello.ack | Both | Negotiate protocol, report app readiness and pairing |
| speak | Extension → app | Request reading |
| speak.accepted / speak.rejected | App → extension | Correlated result |
| control | Extension → app | Play, pause, stop, skip, set rate for a specified session |
| state.get / state | Both | Recover or update session snapshot |
| ping / pong | Both | Bounded diagnostic liveness check |
| error | App → extension | Structured error code and safe message |

Control fields: `sessionId`, `action`, optional `deltaMs`, optional `rate`. Reject controls for stale sessions. State includes session ID, request ID, status, source position, estimated/final duration, rate, and current sentence ID. Emit transitions immediately and steady playback updates at most 4 Hz.

Duplicate request IDs within a bounded reconnect window return the existing result instead of restarting audio. Cache at most 100 request outcomes for five minutes; reject reuse of the same ID with a different payload. A crash that loses deduplication state requires explicit retry rather than blind repeated sends.

Error codes include `NOT_AUTHORIZED`, `NO_TEXT`, `TEXT_TOO_LONG`, `PAYLOAD_TOO_LARGE`, `NO_VOICE`, `MODEL_LOADING`, `APP_UNAVAILABLE`, `UNSUPPORTED_PROTOCOL`, `STALE_SESSION`, `INVALID_REQUEST`, and `INTERNAL`.

## 14. Local storage, privacy, and diagnostics

- No user text, titles, URLs, audio, or voice previews are sent to a server for processing.
- Network operations are limited to explicit model downloads, catalog refresh, and update checks.
- No analytics or crash uploads by default.
- Store settings and installed-model metadata under the current user's local app-data directory.
- History is off by default. When enabled, store at most the last 20 items with clear Delete item and Clear all controls; explain that replay requires retaining text.
- Retain origin rather than full URL by default; full URLs can contain private tokens.
- Logs contain event IDs, timings, model IDs, resource readings, and sanitized error codes, not reading content.
- Rotate logs with a proposed total quota of 20 MiB.
- Persistent content/cache files, if enabled, are user-private under the OS application-data directory. Evaluate per-platform at-rest protection before claiming encryption; ordinary file permissions do not protect against other processes running as that user.
- Clear cache preserves installed models unless the user explicitly selects model deletion.
- Diagnostic export previews the included metadata and excludes reading text by default.
- Extension settings stay local. No browsing-history collection or passive page-text harvesting.

## 15. Error and empty-state copy

| Condition | Surface | Required response |
|---|---|---|
| Empty clipboard | Player notice | “Clipboard is empty. Copy some text and try again.” |
| Non-text clipboard | Player notice | “Clipboard does not contain text.” |
| Clipboard busy | Player notice | “Clipboard is busy. Try again.” |
| No selection | Player notice | “Select text first, or copy it and use Speak clipboard.” |
| Capture not permitted | Player notice | “This app cannot be read automatically. Copy the text manually.” |
| No usable voice | Voices/player | “Choose or download a voice to start reading.” |
| Model loading | Player | “Loading voice…” with cancellation |
| Download failed | Model card | Retry/resume action and a specific reason |
| Hash mismatch | Model card | “The download could not be verified. Download again.” |
| Unsupported GPU | Performance | “This voice cannot use the selected GPU. Use CPU.” |
| Resource pressure | Player/settings | Explain reduced buffering and offer lower-resource mode |
| Bridge absent | Extension popup | “Install or repair the SpeakIt desktop connection.” |
| App cannot launch | Popup/confirmation | “SpeakIt could not be opened. Open it and retry.” |
| Pairing pending | Popup/confirmation | “Allow this extension in SpeakIt.” |
| Restricted page | Popup | “This page cannot be picked. Copy text and use Speak clipboard.” |
| Over text limit | Confirmation | Offer first 200,000 units with explicit consent or Cancel |
| Audio device missing | Player | “Audio device disconnected. Choose an output device.” |
| Source navigated/closed | Source chip | Mark unavailable; continue accepted reading |
| Inference worker crash | Player | Keep position; Retry or Switch voice |

Errors must remain long enough to act on and be announced accessibly. Do not make a three-second toast the only place containing a recovery action.

## 16. Accessibility requirements

- Every action is keyboard reachable with an accessible name and visible focus.
- Focus order follows layout; no focus traps in popup or confirmation.
- Provide a keyboard route to picker scope, confirmation, and cancellation.
- Target at least 4.5:1 contrast for normal text and 3:1 for large text/control boundaries where applicable; verify actual rendered combinations.
- Color and waveform motion never carry the only status information.
- Player controls have at least 32 × 32 desktop hit areas; use larger targets where space permits.
- Respect system text scaling, high contrast, and reduced motion.
- Avoid constant screen-reader announcements of playback ticks; announce state changes and requested position changes.
- Focus-stealing avoidance and accessibility must coexist: the compact player appears passively, while an explicit open/focus command makes it operable.

## 17. Packaging and installation

1. Produce native packages for every validated OS/architecture: Windows installer, signed/notarized macOS app where distributed, and a chosen Linux package format. Test installation on clean machines.
2. Bundle the Tauri shell, Rust/native TTS worker, native messaging host, required ONNX/audio libraries, and notices. Do not bundle Python, Torch, Node.js as a runtime, an LLM, or unverified GPU libraries.
3. Download model files separately after explicit user action unless a small voice is legally and technically verified for bundling. Do not promise first-run audio before a usable engine/voice is present.
4. Install the Chrome native-host manifest and origin allowlist in each OS-specific per-user registration location; use exact installed paths and extension ID. On Windows use the documented registry key; on macOS/Linux use Chrome's documented host directory.
5. Keep the extension identity stable across the chosen development/distribution workflow. Do not silently enable it in the user's browser.
6. Start-at-login is a user choice implemented by a tested Tauri/native mechanism on each OS. Background operation stays in the signed-in desktop session, never as a privileged system service.
7. Store settings, models, cache, and logs in OS user-data directories, respecting Linux XDG paths. The app's code-signing, executable permissions, shared-library loader paths, and worker supervision must be tested per package.
8. Uninstall removes owned registration and executables; ask whether to retain downloaded models/settings. An update must preserve config and not silently replace an in-use voice.
9. Before public release, complete signatures/notarization where applicable, extension review, model/runtime license review, and a compatibility matrix for Windows, macOS, Linux X11, and named Wayland environments.
10. Revalidate store/distribution policies when publishing; this spec does not authorize publishing. [Tauri distribution](https://v2.tauri.app/start/), [Chrome Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)

## 18. Step-by-step delivery plan

Implementation begins only after a separate user instruction. Complete milestones sequentially and retain evidence for each gate.

### Step 0 — Resolve platform and establish evidence

- [ ] Confirm exact Windows, macOS, and Linux release versions/architectures and representative CPU/GPU hardware.
- [ ] Confirm English-first scope and any required Hindi/other voices.
- [ ] Record private versus public distribution intent.
- [ ] Pin SDK/runtime candidates and catalog schema.
- [ ] Check exact model/export/phonemizer/provider compatibility and licenses.
- [ ] Create benchmark corpus and result template.

**Gate:** written architecture decisions; no vague “GPU supported” claim.

### Step 1 — Design the product UI

- [ ] Apply the installed frontend-design skill to the design brief in §3.
- [ ] Create tokens and component states.
- [ ] Design Read, Voices, Shortcuts, Extension, and Settings.
- [ ] Design compact/expanded player and picker confirmation.
- [ ] Include light/dark, empty, loading, error, disabled, and long-content variants.
- [ ] Review keyboard navigation, contrast, scaling, and user-facing copy.
- [ ] Revise any decorative dashboard elements that compete with reading.

**Gate:** coherent screen designs using real sample text, with no implementation required for review.

### Step 2 — Prove speech quality and resource feasibility

- [ ] Build a throwaway local inference harness when implementation is authorized.
- [ ] Produce one reference WAV using the official Kokoro pipeline.
- [ ] Validate phonemization and token output against the chosen reference.
- [ ] Compare candidate ONNX artifacts, precision modes, and three voices.
- [ ] Measure CPU first; validate one GPU path separately.
- [ ] Evaluate Piper only with its integration/license decision recorded.
- [ ] Select the recommended quality artifact and an offline starter voice path for every OS; if no usable system voice exists, choose a small verified model to integrate in Step 4.

**Gate:** repeatable report showing actual audio quality, first-audio latency, RTF, RAM, and CPU/GPU behavior. If targets fail, revise model choice before building around it.

### Step 3 — Create desktop shell

- [ ] Create Tauri 2 + React/TypeScript shell, Rust crate boundaries, command/event schemas, and permission scopes.
- [ ] Build React navigation, theme resources, and Rust-owned settings persistence.
- [ ] Add tray/menu bar, single-instance behavior, start-at-login option, and explicit Exit.
- [ ] Spike compact overlay positioning/focus on Windows, macOS, Linux X11, and a named Wayland session.
- [ ] Add global shortcut registration, conflict/permission feedback, and fallback actions.

**Gate:** the source editor retains typing focus on every supported platform; hotkeys survive 15 minutes idle and sleep/wake where registration is available. Document Wayland limitations.

### Step 4 — Deliver basic local reading

- [ ] Implement manual input and clipboard path.
- [ ] Add a verified local voice path on each target OS: tested system voice if accessible, otherwise the Step 2 lightweight model with explicit install/download before offline use.
- [ ] Add text validation and safe error messages.
- [ ] Route all entry paths through one session controller.
- [ ] Add play, pause, stop, and replace-session behavior.

**Gate:** after a usable voice is installed, copy a paragraph, trigger the platform-registered Speak clipboard action, hear it offline, pause/stop reliably, and preserve clipboard content on every declared target.

### Step 5 — Build document index and audio scheduling

- [ ] Implement deterministic normalization and source-span mapping.
- [ ] Add sentence/chunk segmentation fixtures.
- [ ] Add bounded inference work queue and PCM cache.
- [ ] Implement cancellation generations and stale-result rejection.
- [ ] Add pitch-preserving speed, volume, and source-time tracking.
- [ ] Implement seek into cached and uncached segments.

**Gate:** long text begins promptly; speed changes preserve source position and do not resynthesize audio.

### Step 6 — Add neural models and resource profiles

- [ ] Implement catalog validation and verified downloads.
- [ ] Build model state transitions and metadata-driven controls.
- [ ] Integrate the chosen Kokoro engine in the inference worker.
- [ ] Add Eco/Balanced/Performance and explicit CPU/GPU selection.
- [ ] Implement unload, pressure handling, worker recovery, and bounded lookahead.
- [ ] Add Piper only if its gates pass.

**Gate:** the recommended quality voice passes quality/performance targets on declared hardware without making foreground work unusable; the Step 4 starter voice remains a working fallback.

### Step 7 — Polish the player

- [ ] Implement waveform and accessible seek control.
- [ ] Implement expanded sentence view and follow-reading behavior.
- [ ] Add source chip, monitor memory, and keyboard access.
- [ ] Add audio-device recovery and optional system media controls.
- [ ] Profile audio callback allocations and hidden-window CPU.

**Gate:** 30-minute playback under ordinary browser/editor load without avoidable audio glitches or unbounded memory growth.

### Step 8 — Add best-effort desktop capture

- [ ] Implement platform selection adapters: Windows UI Automation, macOS Accessibility where permitted, and Linux AT-SPI where available.
- [ ] Add modifier-release waiting and bounded synthetic-copy fallback.
- [ ] Handle clipboard format limitations and concurrent user copies.
- [ ] Test representative apps and OS permission/compositor restrictions; document unsupported capture and fallback routes.
- [ ] Keep manual copy as an explicit fallback.

**Gate:** no Chrome DevTools launch, stuck modifiers, unexpected whole-document reading, or overwritten later user clipboard content.

### Step 9 — Build the native bridge

- [ ] Implement framed reads/writes, byte caps, and schema validation.
- [ ] Add per-user pipe or Unix-socket security, native-host registration, and caller-origin handling on every target OS.
- [ ] Add hello, pairing, revoke, speak, control, and state.
- [ ] Implement launch/reconnect timeouts and request deduplication.
- [ ] Test with a minimal extension before the picker.

**Gate:** round trip succeeds; malformed, oversized, unauthorized, and duplicate messages behave predictably.

### Step 10 — Deliver extension selection reading

- [ ] Create MV3 manifest, worker, popup, and options.
- [ ] Implement permission modes and local settings.
- [ ] Add selected-text context menu and keyboard command.
- [ ] Connect to desktop with visible readiness/errors.
- [ ] Handle worker restart and tab navigation.

**Gate:** selected browser text reaches the same native player without any cloud traffic.

### Step 11 — Deliver long-press picker and copy controls

- [ ] Implement trigger state machine and editable-field exclusions.
- [ ] Add hover highlight and candidate discovery.
- [ ] Add wheel/keyboard scope controls.
- [ ] Add latched confirmation with text preview.
- [ ] Implement Listen, Copy text, Back, and Close.
- [ ] Add safe extraction, frame ownership, and per-site disabling.
- [ ] Verify ordinary Ctrl/Alt shortcuts are preserved.

**Gate:** documented results across at least ten representative sites/fixtures and all picker cancellation paths.

### Step 12 — Verify and package

- [ ] Run the matrix in §19.
- [ ] Verify fresh-user installation and uninstall on each declared Windows, macOS, and Linux package target.
- [ ] Verify offline operation after model installation.
- [ ] Review licenses, permissions, logging, and accessibility.
- [ ] Package private installer and extension instructions.
- [ ] Record known limitations and supported hardware/model combinations.

**Gate:** all P0/P1 acceptance criteria pass or are explicitly removed from release scope by the user.

### Step 13 — Optional subsequent improvements

- [ ] Opt-in recent readings and local usage statistics.
- [ ] Edge verification and packaging.
- [ ] Audio export, reading queue, or exact word alignment as separate specs.
- [ ] Additional languages with independent quality reports.
- [ ] Broader Linux desktop/compositor support and other platforms only after explicit compatibility review.

## 19. Verification matrix

| Area | Required scenarios | Pass condition |
|---|---|---|
| Text | Abbreviations, Unicode, emoji, long sentences, whitespace, cap boundary | Stable segmentation and valid span mappings |
| Playback | Pause/resume, stop during load, rapid replacement, rate changes, seek | No stale audio or incorrect session state |
| Resource use | Eco/Balanced, CPU/GPU, 30-minute read, 3× playback, pressure | Measured targets and bounded memory behavior |
| Models | Interrupted download, wrong hash, insufficient disk, failed update | Prior working model preserved; actionable error |
| Capture | Representative native/browser/editor/PDF apps on each OS | Selected text or honest fallback; no corruption |
| Picker | News page, docs, GitHub, email UI, SPA, lists, short heading | Predictable candidate and correct visible text |
| Frames | Same/cross-origin, nested, inaccessible, navigation | Single owner; no stale-document extraction |
| Keyboard | Ctrl+C/V/L, Alt+Tab, AltGr, IME, key repeat, held modifiers | Normal behavior outside armed picker |
| Bridge | Partial reads, oversized frame, invalid JSON, spoofed fields, reconnect | Safe rejection and recovery |
| Output device | Unplug, Bluetooth, sleep/wake, changed default | Preserved position or clear paused recovery |
| UI | DPI, narrow work area, dark/high contrast, keyboard-only | No clipping or inaccessible controls |
| Privacy | Network disabled, log inspection, history off, incognito disabled | Local reading and no unsolicited content retention |
| Packaging | Fresh install, upgrade, uninstall, missing bridge on each OS | Correct user paths, native-host registration, recovery guidance |
| Platform behavior | Hotkeys, background, focus-safe player on Windows, macOS, Linux X11 and named Wayland session | Working action or documented OS restriction with tested fallback |

Benchmark procedure: one cold launch plus at least 20 warm starts per chosen configuration; record p50/p95, process-tree memory, total-machine-normalized CPU, provider-specific GPU memory, RTF, and underruns. Use the same corpus and repeat under a reproducible foreground workload. Keep quality listening separate from throughput measurements.

## 20. Definition of done

- [ ] Attractive, reviewed UI exists for desktop, player, and extension states.
- [ ] Clipboard and manual input work offline once a usable voice is installed; a zero-download system fallback is provided only where verified.
- [ ] At least one downloadable quality voice passes recorded evaluation.
- [ ] CPU operation is a complete supported mode.
- [ ] At least one GPU combination is verified if GPU support is advertised.
- [ ] Resource profiles constrain concurrency, buffers, and idle work.
- [ ] Chrome selected-text reading and configurable picker work on documented supported pages.
- [ ] Long-press controls, scope changes, Copy text, and Listen work without trapping normal input.
- [ ] All paths use one session and one native player.
- [ ] Model capabilities drive real, supported settings.
- [ ] Failures are visible and recoverable.
- [ ] No LLM or cloud inference dependency exists.
- [ ] No source text is transmitted externally for processing.
- [ ] Windows, macOS, Linux X11, and declared Wayland environments have separate tests, benchmarks, capability notes, and install instructions; unverified combinations are not advertised as supported.

## 21. Corrections and deliberate changes from the pasted v2

| Pasted issue or ambiguity | Decision in this spec |
|---|---|
| Missing v1 requirements | Self-contained behavior and contracts |
| Windows-only WPF/.NET stack | Tauri 2 + React/TypeScript + Rust from the shared conversation; native adapters per OS |
| Model quality/speed asserted without measurements | Benchmark and listening gates |
| “One voice per Piper file” assumed universally | Capability metadata supports actual single/multi-speaker artifacts |
| Active/update/download states conflated | Separate installation, activation, update dimensions |
| Automatic preview after download | Explicit Preview action |
| Native messaging 1 MB direction reversed | Correct Chrome limits plus stricter app limits |
| Character count treated as serialized size | Separate UTF-16 text and encoded JSON byte limits |
| Duration estimate multiplied by speed | Divide by speed |
| Release Alt exits before Listen can be clicked | Latched picker with explicit cancellation |
| Alt-only picker | Configurable Ctrl/Alt/command activation |
| Copy text behavior unclear | Explicit Copy text with preview and feedback |
| Shadow root called page-proof | Style isolation with documented limits |
| Detached clone innerText assumed rendered | Extract from live rendered visibility |
| Public page messages trusted for frame text | Extension runtime coordination with sender identity |
| Broad all-sites permission assumed necessary | Optional approved-site access plus on-demand mode |
| Force-release/restore physical modifiers | Wait for release, then bounded safe fallback |
| Clipboard always restored unconditionally | Preserve later user clipboard changes; skip unsafe capture |
| Full duration known at acceptance | Estimate/null until synthesis supplies actual timing |
| Zero WPF frame allocations guaranteed by API choice | Measure WebView rendering; strict native audio-callback rule |
| Exact reference screenshot implied | No screenshot supplied; follow documented tokens only |

## 22. Decisions to confirm before implementation

These do not prevent using this document as a planning artifact.

1. Exact OS releases, CPU architectures, Linux distributions, and Wayland compositors to claim as supported. Cross-platform desktop scope and Tauri stack are set.
2. Actual CPU/GPU and RAM available for the primary performance target.
3. Required languages beyond English, including whether Hindi is mandatory.
4. Private personal use versus a distributable product, affecting Piper integration and packaging choices.
5. Preferred default browser picker trigger: Ctrl, Alt, or shortcut-only. All remain configurable.
6. Whether public Chrome Web Store distribution belongs in the first release.

Until confirmed, use the assumptions in §1.1 and do not silently expand scope.

## 23. Copyable future development handoff

The following text may be copied into a future development task. It is not an instruction to start implementation now.

```text
Implement SpeakIt using spec/SpeakIt-Product-Spec.md as the source of truth.
Use Tauri 2, React/TypeScript, Rust core and native ONNX TTS worker.
Resolve exact OS versions, hardware, language, and distribution targets.
Keep all speech local. Do not add an LLM, summarization, rewriting, or cloud TTS.
Apply the installed frontend-design skill to the product UI and preserve the
reading-focused design, waveform player, and accessibility requirements.
Follow the numbered milestones sequentially. Start with design and a local
speech feasibility benchmark, then deliver clipboard reading before complex
capture and extension work. Use one playback session and one native player.
Implement resource budgets, cancellation, explicit CPU/GPU reporting, model
verification, and a permission-aware Chrome picker with Ctrl/Alt long press,
scope controls, text preview, Copy text, and Listen. Verify each milestone's
gate and report measured evidence rather than assumed performance. Do not
publish or add LLM/STT features without a separate request.
```

## 24. References and evidence boundary

Primary sources consulted during specification preparation:

1. [Kokoro official model card](https://huggingface.co/hexgrad/Kokoro-82M) — model identity, weights license, reference inference.
2. [Maintained Piper repository](https://github.com/OHF-Voice/piper1-gpl) — engine and license information.
3. [ONNX Runtime execution providers](https://onnxruntime.ai/docs/execution-providers/) — available acceleration backends.
4. [ONNX Runtime thread management](https://onnxruntime.ai/docs/performance/tune-performance/threading.html) — thread pools and spinning controls.
5. [Tauri 2](https://v2.tauri.app/start/) — cross-platform desktop shell and Rust command bridge.
6. [Chrome Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging) — transport, limits, registration, caller origin.
7. [Chrome activeTab](https://developer.chrome.com/docs/extensions/develop/concepts/activeTab) — temporary permission model.
8. [Chrome content scripts](https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts) — script isolation and frames.
9. [Chrome service-worker lifecycle](https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/lifecycle) — restartable extension background work.
10. [MDN innerText](https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement/innerText) — rendered versus detached text behavior.
11. [Frontend design skill source](https://github.com/anthropics/skills/tree/main/skills/frontend-design) — design process applied to this spec.
12. [Shared architecture conversation](https://chatgpt.com/share/6ab777c9-6118-83e8-8ed4-fe510c699ce2) — user-provided Tauri/Rust/React and native ONNX TTS direction; its LLM material is out of scope.
13. [Tauri Global Shortcut plugin](https://v2.tauri.app/plugin/global-shortcut/) — candidate registered hotkey API.
14. [Tauri Shell plugin](https://v2.tauri.app/plugin/shell/) — candidate supervised external worker launch.
15. [Tauri Autostart plugin](https://v2.tauri.app/plugin/autostart/) — candidate login startup control.
16. [XDG GlobalShortcuts portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html) — Linux compositor-dependent global shortcuts.
17. [CPAL](https://github.com/RustAudio/cpal) — cross-platform Rust audio output candidate and backend requirements.
18. [Tauri sidecar guide](https://v2.tauri.app/develop/sidecar/) — packaging worker binaries.

No product implementation, executable prototype, audio benchmark, or visual screenshot validation was performed during this specification task. The installed design skill and these Markdown specification files are the deliverables. The share conversation was read in the browser; its architecture is treated as a design input, not proof that every native API works on every OS.
