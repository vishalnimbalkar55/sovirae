# Step 4 — Deliver basic local reading

[Spec index](../README.md) · [All steps](README.md)

**Planning only. Do not start implementation without a separate user instruction.**

**Dependency:** Complete Step 3 and its gate first.

- [ ] Implement manual input and clipboard path.
- [ ] Add a verified local voice path on each target OS: tested system voice if accessible, otherwise the Step 2 lightweight model with explicit install/download before offline use.
- [ ] Add text validation and safe error messages.
- [ ] Route all entry paths through one session controller.
- [ ] Add play, pause, stop, and replace-session behavior.

**Gate:** after a usable voice is installed, copy a paragraph, trigger the platform-registered Speak clipboard action, hear it offline, pause/stop reliably, and preserve clipboard content on every declared target.

## Relevant requirements

- [6. Entry paths and exact user journeys](../chunks/06-reading-entry-paths.md)
- [9. Text pipeline and document index](../chunks/09-text-pipeline.md)
- [10. Playback behavior and floating player](../chunks/10-floating-player.md)
