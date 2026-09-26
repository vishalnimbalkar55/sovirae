# Step 9 — Build the native bridge

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 8 and its gate first.

- [x] Implement framed reads/writes, byte caps, and schema validation.
- [ ] Add per-user pipe or Unix-socket security, native-host registration, and caller-origin handling on every target OS.
- [ ] Add hello, pairing, revoke, speak, control, and state.
- [x] Implement launch/reconnect timeouts and request deduplication.
- [ ] Test with a minimal extension before the picker.

**Gate:** round trip succeeds; malformed, oversized, unauthorized, and duplicate messages behave predictably.

## Progress — 2026-09-26

- Built: `speakit-protocol` (framing, limits, validation, de-duplication; 8 tests), `speakit-native-host` (relay; checks the socket is owned by this user; starts the app with `--background`; 5 s limit), and the app bridge (`src-tauri/src/bridge.rs`: owner-only Unix socket at 0600, peer-UID check, host manifests for Chrome, Chrome Beta/Canary, Chromium, Edge, Brave, Vivaldi, and Arc, pairing prompt, revoke, Test connection).
- Gate evidence (macOS): a Chrome stand-in script launched the registered host exactly as Chrome does and passed 22/22 checks: invalid origin refused, malformed JSON, unsupported version, out-of-range values, empty and over-limit text, a 3 MB frame refused without losing sync, speak accepted with an estimated duration, playing within 0.78 s, request ID in state, duplicate ID replayed, reused ID refused, stale-session control refused, pause and stop. With the app closed, the host started it in the background and answered in 0.31 s.
- Open: the on-screen pairing prompt and Revoke were not clicked through by a test; Windows (named pipe) and Linux are not built; testing with the real extension in Chrome is pending.

## Relevant requirements

- [5. Architecture and platform boundaries](../chunks/05-architecture.md)
- [13. Native messaging bridge and protocol](../chunks/13-native-bridge.md)
- [14. Local storage, privacy, and diagnostics](../chunks/14-privacy-and-storage.md)
