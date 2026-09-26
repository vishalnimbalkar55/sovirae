# Step 9 — Build the native bridge

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 8 and its gate first.

- [ ] Implement framed reads/writes, byte caps, and schema validation.
- [ ] Add per-user pipe or Unix-socket security, native-host registration, and caller-origin handling on every target OS.
- [ ] Add hello, pairing, revoke, speak, control, and state.
- [ ] Implement launch/reconnect timeouts and request deduplication.
- [ ] Test with a minimal extension before the picker.

**Gate:** round trip succeeds; malformed, oversized, unauthorized, and duplicate messages behave predictably.

## Relevant requirements

- [5. Architecture and platform boundaries](../chunks/05-architecture.md)
- [13. Native messaging bridge and protocol](../chunks/13-native-bridge.md)
- [14. Local storage, privacy, and diagnostics](../chunks/14-privacy-and-storage.md)
