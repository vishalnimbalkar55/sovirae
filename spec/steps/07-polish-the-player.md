# Step 7 — Polish the player

[Spec index](../README.md) · [All steps](README.md)

**Planning only. Do not start implementation without a separate user instruction.**

**Dependency:** Complete Step 6 and its gate first.

- [ ] Implement waveform and accessible seek control.
- [ ] Implement expanded sentence view and follow-reading behavior.
- [ ] Add source chip, monitor memory, and keyboard access.
- [ ] Add audio-device recovery and optional system media controls.
- [ ] Profile audio callback allocations and hidden-window CPU.

**Gate:** 30-minute playback under ordinary browser/editor load without avoidable audio glitches or unbounded memory growth.

## Relevant requirements

- [10. Playback behavior and floating player](../chunks/10-floating-player.md)
- [16. Accessibility requirements](../chunks/16-accessibility.md)
