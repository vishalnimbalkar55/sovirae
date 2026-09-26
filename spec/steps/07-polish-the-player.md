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
- Player line styles (user's pick from the 10 ideas in `design/speaking-visuals.html`): Settings › Appearance › Player line offers Waveform (default), Word ticker, Sentence steps, Live meter, and Breathing line, saved as `playerLine`. Word ticker and Sentence steps make the collapsed player 124 px tall instead of 112 px. Engines report no word timings, so the spoken word is estimated by spreading each segment's text over the voiced frames of its envelope. Every style seeks by drag, click, or arrow keys through the same invisible range. Reviewed live in the browser (dev mock now simulates playback) at 520 × 112/124 in light and dark; Rust test covers the fallback and heights.
- Open: source chip focus action, per-monitor position memory checks, audio-device recovery testing, media keys, and profiling.

## Relevant requirements

- [10. Playback behavior and floating player](../chunks/10-floating-player.md)
- [16. Accessibility requirements](../chunks/16-accessibility.md)
