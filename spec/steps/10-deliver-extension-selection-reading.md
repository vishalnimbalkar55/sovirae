# Step 10 — Deliver extension selection reading

[Spec index](../README.md) · [All steps](README.md)

**Planning only. Do not start implementation without a separate user instruction.**

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
