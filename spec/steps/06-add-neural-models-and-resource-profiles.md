# Step 6 — Add neural models and resource profiles

[Spec index](../README.md) · [All steps](README.md)

**Planning only. Do not start implementation without a separate user instruction.**

**Dependency:** Complete Step 5 and its gate first.

- [ ] Implement catalog validation and verified downloads.
- [ ] Build model state transitions and metadata-driven controls.
- [ ] Integrate the chosen Kokoro engine in the inference worker.
- [ ] Add Eco/Balanced/Performance and explicit CPU/GPU selection.
- [ ] Implement unload, pressure handling, worker recovery, and bounded lookahead.
- [ ] Add Piper only if its gates pass.

**Gate:** recommended local voice passes quality/performance targets on declared hardware without making foreground work unusable.

## Relevant requirements

- [7. Local TTS strategy](../chunks/07-local-tts-and-models.md)
- [8. Resource controls and measurable performance](../chunks/08-resource-budgets.md)
