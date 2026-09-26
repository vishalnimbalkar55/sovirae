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

## Progress — 2026-09-26

- Built, not yet loaded in Chrome: MV3 manifest with a pinned ID (`jhbbmlhbjhjdepmaebpniefhaoljfgoe`), service worker (on-demand native port, waits for the allow prompt, one same-ID retry after a reconnect, badge), popup (status, now playing with controls, Read selection, Pick text, per-site Always ready and on/off), options (trigger, hold time, site lists), a Read with Sovirae context menu, and Option+Shift+S / Option+Shift+P commands. Settings live in `chrome.storage.local`; site access is optional and requested per site.
- The popup was rendered with stand-in Chrome APIs and looks correct. The Sovirae app's Extension screen now shows connection status, registered browsers, Test connection, install steps, allowed extensions with Revoke, and a Match page language switch.

## Relevant requirements

- [12. Chrome extension specification](../chunks/12-chrome-extension.md)
- [13. Native messaging bridge and protocol](../chunks/13-native-bridge.md)
