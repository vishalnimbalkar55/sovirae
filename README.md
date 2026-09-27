# Sovirae

Local read-aloud app with a floating player and a Chrome extension. Speech is
generated on this computer: macOS system voices out of the box, and Kokoro-82M
or Kyutai Pocket TTS neural voices after a one-time download.

The product spec lives in [`spec/`](spec/README.md); progress is tracked in the
step files there.

## Build the complete project

One command checks prerequisites, installs packages, runs every test, and
builds the app, installer, and Chrome extension for every platform this
computer can build. It works the same on macOS and Windows:

```bash
npm run build:all
```

Everything ends up in `build/` (each platform folder is cleared and refilled):

```
build/macos/Sovirae.app                    the macOS app            (built on a Mac)
build/macos/Sovirae_0.1.0_aarch64.dmg      the macOS installer
build/windows/Sovirae_0.1.0_x64-setup.exe  the Windows installer    (built on Windows, or on a
                                           Mac set up for cross-builds)
build/ext/                                 the Chrome extension, for Load unpacked
```

Options: `-- --skip-tests`, `-- --no-install` (reuse `node_modules`), and
`-- --mac` / `-- --windows` to build one platform, e.g.
`npm run build:all -- --windows`. `npm run app:build` is the quick form
(no tests, no reinstall). To copy the last builds into `build/` again without
rebuilding:

```bash
npm run build:collect
```

A full macOS build takes about 3 minutes from a warm cache (longer the first
time, while Rust and ONNX Runtime download).

### Windows installer from a Mac (optional, experimental)

A Mac can also cross-build the Windows installer. Set it up once; the script
asks you to accept Microsoft's license for the Windows SDK that cargo-xwin
downloads, then installs NSIS, LLVM, cargo-xwin, and the Rust target:

```bash
npm run setup:windows-cross
```

After that `npm run build:all` fills both `build/macos/` and `build/windows/`.
Without it, the Mac build skips Windows with a warning. Test cross-built
installers on a Windows PC.

### Prerequisites on Windows (Windows 10 1803+ or 11, x64)

| Tool | Install |
|---|---|
| Visual Studio 2022 Build Tools | "Desktop development with C++" workload |
| Rust (stable, MSVC) | from rustup.rs (`rustup default stable-msvc`) |
| Node.js 22+ | from nodejs.org |

Windows support is being ported in phases; see
[spec/steps/14-windows-support.md](spec/steps/14-windows-support.md).

### Prerequisites (macOS 13 or later; built and tested on Apple Silicon)

| Tool | Install |
|---|---|
| Xcode Command Line Tools | `xcode-select --install` |
| Rust (stable) | `curl https://sh.rustup.rs -sSf \| sh` |
| Node.js 22+ | from nodejs.org or `brew install node` |
| espeak-ng (for Kokoro voices) | `brew install espeak-ng` |

`build:all` stops with a clear message if a required tool is missing and warns
if espeak-ng is absent (the app still builds; system voices still work).

### What the build contains

`Sovirae.app/Contents/MacOS/` holds four programs:

| File | Role |
|---|---|
| `sovirae` | The app: windows, player, shortcuts, menu bar icon |
| `sovirae-kokoro-worker` | Kokoro inference in its own process (ONNX Runtime built in) |
| `sovirae-pocket-worker` | Pocket TTS inference in its own process (Candle, Apple Accelerate) |
| `sovirae-native-host` | Connects Chrome to the app |

`build/ext/` is the Chrome extension, ready for **Load unpacked** (the app also carries a copy in `Contents/Resources/ext/`).
Neural voice models are not bundled; download them in Sovirae › Voices.

The build is ad-hoc signed, which is fine on the machine that built it. To
share it, sign and notarize with an Apple Developer ID.

## Everyday commands

| Task | Command |
|---|---|
| Build everything (tests + app + installer + extension) | `npm run build:all` |
| Set up a Mac to also build the Windows installer (once) | `npm run setup:windows-cross` |
| Build without tests | `npm run app:build` |
| Copy the last build into `build/` again (no rebuild) | `npm run build:collect` |
| Run in development (hot reload) | `npm run app:dev` |
| Run all tests (Rust + extension) | `npm test` |
| Work on the UI in a browser with sample data | `npm run dev`, then open `http://localhost:1420` |

Longer checks that play audio or need a downloaded model are opt-in:

```bash
cargo test -p sovirae -- --ignored          # real playback, including Kokoro and Pocket TTS
python3 scripts/check-bridge.py             # Chrome bridge, 22 checks (extension must be allowed)
cargo run --release -p speakit-tts --example kokoro_bench -- \
  "$HOME/Library/Application Support/com.sovirae.desktop/models" fp32 /tmp/kokoro-wav
cargo run --release -p speakit-tts --example pocket_bench -- \
  "$HOME/Library/Application Support/com.sovirae.desktop/models" /tmp/pocket-wav
```

## Chrome extension

1. Start Sovirae once so it registers its connection with Chrome, Edge, Brave,
   Chromium, Vivaldi, and Arc.
2. Open `chrome://extensions`, turn on **Developer mode**, choose **Load
   unpacked**, and select `build/ext` (or Sovirae › Extension › Show folder).
   In development you can load [`ext/`](ext/) directly.
3. Select text and press **⌥⇧S**, right-click › **Read with Sovirae**, or hold
   **Option** to pick part of a page. Allow Chrome the first time Sovirae asks.

The extension ID is fixed at `jhbbmlhbjhjdepmaebpniefhaoljfgoe` by the `key` in
`ext/manifest.json`. The matching private key (only needed to pack a `.crx`) is
kept outside the repository in
`~/Library/Application Support/com.sovirae.desktop/keys/`.

## Layout

| Path | What |
|---|---|
| `src/` | React UI: main window and floating player |
| `src-tauri/` | Tauri shell: session controller, shortcuts, tray, models, Chrome bridge |
| `crates/speakit-core` | Text validation, normalization, sentence index |
| `crates/speakit-audio` | Audio output and pitch-preserving speed |
| `crates/speakit-tts` | System voices, Kokoro and Pocket TTS engines, phonemizer, engine registry |
| `crates/speakit-models` | Model catalog (`catalog/models.json`) and verified downloads |
| `crates/speakit-protocol` | Chrome bridge message format |
| `workers/speakit-kokoro-worker` (`sovirae-kokoro-worker`) | Isolated ONNX inference process |
| `workers/speakit-pocket-worker` (`sovirae-pocket-worker`) | Isolated Pocket TTS inference process (Rust port of Kyutai's model) |
| `workers/speakit-native-host` (`sovirae-native-host`) | Chrome native messaging relay |
| `ext/` | Chrome MV3 extension |
