# Step 5 — Build document index and audio scheduling

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 4 and its gate first.

- [x] Implement deterministic normalization and source-span mapping.
- [x] Add sentence/chunk segmentation fixtures.
- [x] Add bounded inference work queue and PCM cache.
- [x] Implement cancellation generations and stale-result rejection.
- [x] Add pitch-preserving speed, volume, and source-time tracking.
- [ ] Implement seek into cached and uncached segments.

**Gate:** long text begins promptly; speed changes preserve source position and do not resynthesize audio.

## Progress — 2026-09-26

- Speed uses a built-in streaming WSOLA stretcher; measured 2.00× playback at the 2.0× setting with no resynthesis. Volume is applied in the audio callback.
- Seeking into already synthesized audio is tested (skip +5 s while paused). Seeking into unsynthesized audio is implemented but not tested yet.

## Relevant requirements

- [8. Resource controls and measurable performance](../chunks/08-resource-budgets.md)
- [9. Text pipeline and document index](../chunks/09-text-pipeline.md)
- [10. Playback behavior and floating player](../chunks/10-floating-player.md)
