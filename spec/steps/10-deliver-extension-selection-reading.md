# Step 10 — Deliver extension selection reading

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 9 and its gate first.

- [ ] Create MV3 manifest, worker, popup, and options.
- [ ] Implement permission modes and local settings.
- [ ] Add selected-text context menu and keyboard command.
- [ ] Connect to desktop with visible readiness/errors.
- [ ] Handle worker restart and tab navigation.

**Gate:** selected browser text reaches the same native player without any cloud traffic.

## Relevant requirements

- [12. Chrome extension specification](../chunks/12-chrome-extension.md)
- [13. Native messaging bridge and protocol](../chunks/13-native-bridge.md)
