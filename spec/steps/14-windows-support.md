# Step 14 — Windows support

[Spec index](../README.md) · [All steps](README.md)

**Authorized on 2026-09-27.** Windows 10 1803+ / Windows 11, x64 only. The user builds and tests on their own Windows PC; code is compile-checked from macOS with `cargo check --target x86_64-pc-windows-gnu` (mingw-w64). A checked item has test or review evidence on a real Windows PC; see Progress below.

**Dependency:** none beyond the macOS app; macOS behavior must not change.

## Phase 1 — core app

- [ ] Workspace compiles for `x86_64-pc-windows-msvc` on the Windows PC.
- [ ] Built-in Windows voices (WinRT `Windows.Media.SpeechSynthesis`) are listed with language and gender, and read aloud.
- [ ] The floating player appears without taking focus from the app the user is typing in (`WS_EX_NOACTIVATE`, `SW_SHOWNOACTIVATE`), and takes focus only when expanded.
- [ ] Global shortcuts (Ctrl+Shift layout), tray menu, start at login, and keep-running work.
- [ ] Per-user NSIS installer (no administrator rights) installs, launches, and uninstalls cleanly.
- [ ] Window chrome: native title bar, no macOS spacing.

## Phase 2 — Chrome bridge

- [x] Bridge transport compiles on Windows: AF_UNIX socket in `%LOCALAPPDATA%\com.sovirae.desktop\bridge.sock`, peer checked by process user SID.
- [ ] Native host builds on Windows (no `libc`/Unix-only APIs) and starts the app detached.
- [ ] Host manifests registered under `HKCU\Software\<browser>\NativeMessagingHosts\<name>` for Chrome, Edge, Brave, Chromium, and Vivaldi; removed when the bridge is turned off.
- [ ] Extension pairing, speak, and state messages work end to end in Chrome and Edge.

## Phase 3 — neural models

- [ ] Kokoro worker builds with ONNX Runtime for MSVC and finds eSpeak NG on Windows.
- [ ] Pocket TTS worker builds and runs on the CPU; real-time factor measured.
- [ ] Workers bundled next to `Sovirae.exe` by the installer.
- [ ] Optional: Kokoro on the GPU through DirectML, with fallback reporting.

## Progress

- 2026-09-27 — Phase 1 code written and compile-checked from macOS (no Windows run yet):
  - `crates/speakit-tts/src/windows_speech.rs`: WinRT system engine (plain-text synthesis, so page text is never SSML; cancellable; resampled to the output rate). WAV decoding moved to shared `pcm_from_wav`.
  - `src-tauri/src/platform.rs`: passive player show and keyable toggle via the window's extended style; `peer_is_current_user` for the bridge.
  - `src-tauri/src/bridge.rs`: `uds_windows` sockets on Windows; `speakit-protocol` socket path under `%LOCALAPPDATA%`; native host looked up as `.exe`.
  - `speakit-models`: free disk space via `GetDiskFreeSpaceExW`.
  - UI: `data-platform` on `<html>`; macOS title-bar spacing removed on Windows/Linux; shortcut labels show "Win" for the Super key.
  - `src-tauri/tauri.windows.conf.json`: NSIS, per-user install; workers not bundled until Phase 3 (model cards show "voice engine" missing).
  - Build: `npm run build:all` (`scripts/build.mjs`) builds every platform this computer can and fills `build/macos`, `build/windows`, `build/ext`; a Mac cross-builds the Windows installer after `npm run setup:windows-cross` (experimental, cargo-xwin).
  - macOS: 56 Rust tests and 11 extension tests pass; Windows check of all crates except the native host and Kokoro worker is clean.

- 2026-09-27 — Phase 3 on the Windows PC (debug app run from `target/`):
  - Both workers build for MSVC unchanged; Kokoro runs on the CPU (ONNX Runtime linked in, `DirectML.dll` not needed). Measured with the bench examples, 4 threads: Kokoro fp32 RTF ~0.54 warm, Pocket TTS RTF ~0.55–0.98; cancellation ok.
  - eSpeak NG 1.52.0 is bundled, not installed: `scripts/prepare-windows.mjs` (Tauri's before-dev/build command on Windows) downloads the official MSI (SHA-256 pinned), unpacks it with `msiexec /a`, and stages it with the workers and its `COPYING` in `target/windows-bundle/`, which the installer ships next to `Sovirae.exe`. The bundled copy needs `ESPEAK_DATA_PATH` (no registry entry), which the phonemizer sets.
  - Helper processes start with `CREATE_NO_WINDOW` so the windowed app doesn't flash consoles.
  - Model download seen failing once with "The system cannot find the file specified" on a voice file; retrying resumed and completed. Cause not identified.
  - Not yet verified: the NSIS installer with the bundled helpers, and the Mac cross-build (`prepare-windows.mjs` needs `msiextract` from msitools there).

## Relevant requirements

- [5. Architecture](../chunks/05-architecture.md)
- [10. Floating player](../chunks/10-floating-player.md)
- [13. Native bridge](../chunks/13-native-bridge.md)
- [17. Packaging and installation](../chunks/17-packaging.md)
