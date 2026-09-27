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
- [x] Optional: Kokoro on the GPU, with fallback reporting. CUDA on NVIDIA, not DirectML (see Progress).

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

- 2026-09-27 — Kokoro on an NVIDIA GPU (CUDA). Measured on a GeForce GTX 1650 laptop GPU (4 GB, Turing), driver 592.82, Windows 11 26200, Kokoro-82M fp32 at revision 1939ad2a, ONNX Runtime 1.28 (ort 2.0.0-rc.13, pyke `cuda13` build), 4 intra-op threads (`kokoro_bench … 4 cpu|gpu`):

  | Provider | Warm RTF | Cold first sentence | Result |
  |---|---|---|---|
  | CPU | 0.31–0.42 | 3.5–5.8 s | Baseline |
  | DirectML | — | — | **Rejected**: loads, then every request fails in `/encoder/F0.1/pool/ConvTranspose` (`80070057 The parameter is incorrect`) at every graph optimization level |
  | WebGPU (Dawn, D3D12) | — | — | Not measured: pyke's Windows WebGPU build needs the MSVC 14.50 (VS 2026) STL and does not link with 14.44 |
  | **CUDA 13 + cuDNN 9.14** | **0.097–0.126** | 2.8 s | **Chosen**: 1.1 GB VRAM, 50–75 % GPU utilization while speaking |

  - GPU audio is the same length as CPU audio, correlation 0.997–0.999, waveform SNR 22–26 dB, log-spectral distance 1.3 dB (numeric precision, as on Apple Silicon). GPU output is identical run to run. A listening comparison is still open.
  - End to end through the controller and audio output (`cargo test --release -p sovirae end_to_end_kokoro -- --ignored`, with `SOVIRAE_ESPEAK` pointing at the bundled eSpeak): both pass and the player shows `GPU`. First audio 10.4 s cold on the GPU (loading ~1.3 GB of NVIDIA DLLs from a cold disk cache) and 2.4 s warm, the same warm latency as the CPU.
  - Only the display driver is needed, not the CUDA toolkit. `node scripts/fetch-cuda.mjs` downloads NVIDIA's redistributable archives (cudart 13.1, cuBLAS 13.2, cuFFT 12.1, cuDNN 9.14 for CUDA 13; ~950 MB, SHA-256 pinned) and copies the DLLs next to the built workers. `prepare-windows.mjs` bundles them with `onnxruntime_providers_cuda.dll` and the NVIDIA licenses only once they have been fetched (+1.3 GB to the installer); otherwise the installer is unchanged.
  - Without the libraries, or without an NVIDIA GPU, the worker fails to load on the GPU with a short reason (e.g. "the NVIDIA CUDA libraries are not installed (cublasLt64_13.dll is missing)") and the engine continues on the CPU, as on macOS.
  - Fixed (found on Windows, affects every platform): the player could show "Playing" with the time and line moving while nothing was audible. `Player::clear` waits at most 200 ms for the audio callback to drain the ring; the first callback of a new stream came later, so the late drain discarded the first 200 ms of the new reading, the output count never matched what was played, and the player never reported buffering or completion. The feeder now writes nothing until the drain is acknowledged, starvation ignores the stretcher's sub-frame remainder, and the player UI extrapolates at most 500 ms past the last reported position. Covered by `waiting_for_audio_shows_buffering_and_holds_the_position` (ignored: opens the audio device) and a stretcher unit test. Warm first audio on the GPU went from 2.4 s to 1.0 s.
  - Open: how end users get the libraries. Bundling makes the installer ~1.3 GB larger; the alternative is an explicit, optional in-app download (spec §7.3: no silent large downloads). Cold GPU start and memory under a 30-minute read are not yet measured.

## Relevant requirements

- [5. Architecture](../chunks/05-architecture.md)
- [10. Floating player](../chunks/10-floating-player.md)
- [13. Native bridge](../chunks/13-native-bridge.md)
- [17. Packaging and installation](../chunks/17-packaging.md)
