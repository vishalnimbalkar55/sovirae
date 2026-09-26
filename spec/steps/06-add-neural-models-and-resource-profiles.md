# Step 6 — Add neural models and resource profiles

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 5 and its gate first.

- [x] Implement catalog validation and verified downloads.
- [ ] Build model state transitions and metadata-driven controls.
- [x] Integrate the chosen Kokoro engine in the inference worker.
- [x] Add Eco/Balanced/Performance and explicit CPU/GPU selection.
- [x] Implement unload, pressure handling, worker recovery, and bounded lookahead.
- [ ] Add Piper only if its gates pass.

**Gate:** the recommended quality voice passes quality/performance targets on declared hardware without making foreground work unusable; the Step 4 starter voice remains a working fallback.

## Progress — 2026-09-26

- Catalog pins revision, byte size, and SHA-256 for the model and 28 English voices. Downloads check free space, resume `.part` files only when the ETag still matches, verify every hash, and write `installed.json` last; the previous version stays usable until then.
- Inference runs in `speakit-kokoro-worker`, a separate process with ONNX Runtime linked in statically, intra-op threads from the resource profile (Balanced = 4 here), and idle spinning off. Cancellation or a crash replaces the worker; the idle audio release also unloads it.
- End-to-end test through the real controller and audio output (`cargo test -p speakit end_to_end_kokoro -- --ignored`): first audio 1.75 s from a cold worker and 0.92 s warm.
- Voices screen: download with size and license shown, progress, cancel, retry, remove with confirmation, and Kokoro voices listed once installed. Open: GPU selection, memory-pressure fallback, and measured RAM.
- Multiple models: the catalog is data (`crates/speakit-models/catalog/models.json`, schema-checked by tests), the engine registry holds any number of installed models routed by voice prefix, and the Voices screen lists System voices plus every catalog model. Adding a model of a supported family is a catalog entry; see the catalog README.
- Voices screen now follows the requested layout: Language, Gender, and Voice dropdowns plus Preview at the top for the model in use, then a model list. Downloadable models show Download (with version choice), progress with Cancel, Use it, and Delete with confirmation; System voices show only Use it. Gender comes from the catalog for Kokoro and from macOS (AVSpeechSynthesisVoice) for system voices.
- All voices: the catalog now lists all 55 Kokoro voices (American and British English, Spanish, French, Hindi, Italian, Japanese, Brazilian Portuguese, Mandarin) and all 176 macOS system voices are shown. Verified: every Kokoro voice is installed and listed (including Echo and Eric), and one voice per language produces speech (`cargo run --release -p speakit-tts --example kokoro_languages`).
- Pronunciation outside English uses espeak-ng with misaki's general mapping. Japanese and Mandarin are labelled approximate: espeak-ng does not read kanji and Mandarin tone digits are dropped. misaki's Japanese and Chinese front ends are not ported.
- An installed model that gains catalog voices offers to download only the missing ones.

### GPU — 2026-09-26

Measured on Apple M4 (10 cores), macOS 15.1, Kokoro-82M fp32 at revision 1939ad2a, ONNX Runtime 1.28 (ort 2.0.0-rc.13, pyke `coreml,webgpu` build), 4 intra-op threads, 10-sentence English corpus (`kokoro_bench … 4 cpu|gpu`):

| Path | Warm RTF | CPU time for the corpus | Peak RSS | Result |
| --- | --- | --- | --- | --- |
| CPU | 0.18–0.20 | 40.8–42.6 s | 0.89–0.97 GB | Baseline |
| **WebGPU (Metal via Dawn)** | **0.124–0.147** | **1.1–1.2 s** | 0.81–0.83 GB | **Chosen**: 2,292 nodes on the GPU, 178 shape nodes on the CPU, 32 copies |
| CoreML, MLProgram | — | — | — | Fails at runtime: zero-length dynamic tensor in the iSTFT (1,363 of 2,255 nodes, 43 partitions) |
| CoreML, NeuralNetwork | 0.20–0.21 | 41–42 s | 2.2–2.3 GB | Rejected: no faster, doubles memory (1,038 nodes, 109 partitions) |
| CoreML, static shapes | 0.18–0.19 | 42–43 s | 0.95–0.98 GB | Rejected: no faster (559 nodes) |

- GPU audio is the same length as CPU audio, correlation 0.997–0.998, waveform SNR 22–25 dB, log-spectral distance 1.6–2.1 dB (numeric precision, not a different voice). CPU output is bit-identical run to run. A listening comparison is still open.
- End-to-end through the controller and audio output (`cargo test -p speakit end_to_end_kokoro_gpu -- --ignored`): first audio 2.05 s cold and 1.13 s warm on the GPU, vs 1.72 s / 1.28 s on the CPU in the same run; the player shows `GPU`.
- Settings → Performance → **Use GPU** switch, off by default; it can be turned on only on Apple Silicon (disabled elsewhere). Only downloaded voices are affected; system voices are run by macOS.
- The player shows the device the worker actually loaded, never the requested one. If the GPU cannot load, or fails mid-reading and the CPU succeeds on the same segment, the engine continues on the CPU and Settings shows the reason. Changing the setting tries the GPU again. Covered by fake-worker tests in `speakit-tts`.
- Packaging: the worker links `libwebgpu_dawn.dylib` (8.7 MB, BSD-3-Clause), bundled in `Contents/Frameworks` and checked by `scripts/build.sh`. Intel Macs, Windows (DirectML/CUDA), and Linux have no GPU path yet.

## Relevant requirements

- [7. Local TTS strategy](../chunks/07-local-tts-and-models.md)
- [8. Resource controls and measurable performance](../chunks/08-resource-budgets.md)
