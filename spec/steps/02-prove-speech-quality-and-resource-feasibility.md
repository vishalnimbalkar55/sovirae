# Step 2 — Prove speech quality and resource feasibility

[Spec index](../README.md) · [All steps](README.md)

**Planning only. Do not start implementation without a separate user instruction.**

**Dependency:** Complete Step 1 and its gate first.

- [ ] Build a throwaway local inference harness when implementation is authorized.
- [ ] Produce one reference WAV using the official Kokoro pipeline.
- [ ] Validate phonemization and token output against the chosen reference.
- [ ] Compare candidate ONNX artifacts, precision modes, and three voices.
- [ ] Measure CPU first; validate one GPU path separately.
- [ ] Evaluate Piper only with its integration/license decision recorded.
- [ ] Select the recommended artifact from listening and resource results.

**Gate:** repeatable report showing actual audio quality, first-audio latency, RTF, RAM, and CPU/GPU behavior. If targets fail, revise model choice before building around it.

## Relevant requirements

- [7. Local TTS strategy](../chunks/07-local-tts-and-models.md)
- [8. Resource controls and measurable performance](../chunks/08-resource-budgets.md)
- [19. Verification matrix](../chunks/19-verification.md)
