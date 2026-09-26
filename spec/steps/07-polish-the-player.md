# Step 7 — Polish the player

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 6 and its gate first.

- [x] Implement waveform and accessible seek control.
- [x] Implement expanded sentence view and follow-reading behavior.
- [ ] Add source chip, monitor memory, and keyboard access.
- [ ] Add audio-device recovery and optional system media controls.
- [ ] Profile audio callback allocations and hidden-window CPU.

**Gate:** 30-minute playback under ordinary browser/editor load without avoidable audio glitches or unbounded memory growth.

## Progress — 2026-09-26

- Waveform uses real per-segment amplitude and a dim placeholder for unsynthesized audio, capped at 30 fps and stopped while paused or hidden. Reviewed in light and dark at 520 × 112 and 620 × 500.
- Open: source chip focus action, per-monitor position memory checks, audio-device recovery testing, media keys, and profiling.

## Relevant requirements

- [10. Playback behavior and floating player](../chunks/10-floating-player.md)
- [16. Accessibility requirements](../chunks/16-accessibility.md)
