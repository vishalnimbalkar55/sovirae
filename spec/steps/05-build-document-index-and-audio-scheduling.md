# Step 5 — Build document index and audio scheduling

[Spec index](../README.md) · [All steps](README.md)

**Planning only. Do not start implementation without a separate user instruction.**

**Dependency:** Complete Step 4 and its gate first.

- [ ] Implement deterministic normalization and source-span mapping.
- [ ] Add sentence/chunk segmentation fixtures.
- [ ] Add bounded inference work queue and PCM cache.
- [ ] Implement cancellation generations and stale-result rejection.
- [ ] Add pitch-preserving speed, volume, and source-time tracking.
- [ ] Implement seek into cached and uncached segments.

**Gate:** long text begins promptly; speed changes preserve source position and do not resynthesize audio.

## Relevant requirements

- [8. Resource controls and measurable performance](../chunks/08-resource-budgets.md)
- [9. Text pipeline and document index](../chunks/09-text-pipeline.md)
- [10. Playback behavior and floating player](../chunks/10-floating-player.md)
