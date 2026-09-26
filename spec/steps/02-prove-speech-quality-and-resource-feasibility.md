# Step 2 — Prove speech quality and resource feasibility

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 1 and its gate first.

- [x] Build a throwaway local inference harness when implementation is authorized.
- [ ] Produce one reference WAV using the official Kokoro pipeline.
- [ ] Validate phonemization and token output against the chosen reference.
- [ ] Compare candidate ONNX artifacts, precision modes, and three voices.
- [ ] Measure CPU first; validate one GPU path separately.
- [ ] Evaluate Piper only with its integration/license decision recorded.
- [ ] Select the recommended quality artifact and an offline starter voice path for every OS; if no usable system voice exists, choose a small verified model to integrate in Step 4.

**Gate:** repeatable report showing actual audio quality, first-audio latency, RTF, RAM, and CPU/GPU behavior. If targets fail, revise model choice before building around it.

## Progress — 2026-09-26

- Kokoro-82M v1.0 ONNX (onnx-community, revision 1939ad2a, Apache-2.0), full precision, CPU, 4 inference threads, Apple M4: real-time factor 0.18–0.20 over 10 passages and 2 voices (target ≤ 0.7). Worker start plus model load adds about 0.8 s. Peaks 0.44–0.66, no clipping. Harness: `cargo run --release -p speakit-tts --example kokoro_bench`.
- Phonemes come from espeak-ng 1.52 with misaki's espeak-to-Kokoro mapping. misaki's dictionary lookup is not ported, and output has not yet been compared with the official Python pipeline, so pronunciation validation and the listening evaluation are still open.
- Not yet done: the compact 8-bit artifact comparison, a third voice, a GPU path (CoreML), and Piper.

## Relevant requirements

- [7. Local TTS strategy](../chunks/07-local-tts-and-models.md)
- [8. Resource controls and measurable performance](../chunks/08-resource-budgets.md)
- [19. Verification matrix](../chunks/19-verification.md)
