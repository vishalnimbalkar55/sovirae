# 7. Local TTS strategy

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: development in progress (macOS first).** This topic is synchronized with the full specification; original numbering is preserved.

### 7.1 Model shortlist and selection policy

| Candidate | Intended role | CPU/GPU policy | Release condition |
|---|---|---|---|
| Kokoro-82M | Primary quality candidate | CPU baseline; GPU only for tested export/provider combinations | Pass pronunciation, startup, resource, and listening gates |
| Piper | Lightweight alternative | CPU first | Pass quality checks and engine/voice distribution-license review |
| Platform system voice, where available | Optional zero-download fallback | OS-managed local speech | Test whether each OS exposes offline voices and controllable PCM; do not promise on Linux |

Kokoro has 82 million parameters and Apache-2.0 model weights. That makes it a reasonable compact quality candidate, not proof of performance on the user's hardware. Its official example emits 24 kHz audio; use each artifact's actual declared format. [Model card](https://huggingface.co/hexgrad/Kokoro-82M)

The weights license does not cover text-to-phoneme conversion. Kokoro's reference pipeline uses the `misaki` G2P, which falls back to espeak-ng (GPL-3.0) for out-of-vocabulary words. Record the exact phonemizer used in the Rust worker and its license before packaging. A GPL phonemizer is acceptable for private use but must be resolved before public distribution.

The currently maintained Piper repository describes a local engine and carries GPL-3.0 licensing. Review the exact engine integration and each voice's license before packaging; do not assume all voice files share one license. Piper remains conditional until those decisions are recorded. [Repository](https://github.com/OHF-Voice/piper1-gpl)

### 7.2 Quality evaluation

Prepare a fixed 30-passage corpus covering:

- Short sentences and long paragraphs.
- Questions, dialogue, commas, abbreviations, dates, prices, and decimals.
- Personal names, technical terms, URLs, acronyms, and code-heavy prose.
- Long words, emojis, punctuation-only input, and mixed-script text.
- English accents separately; Hindi and other languages only if explicitly included in the supported catalog.

Compare at least three available Kokoro voices, one suitable Piper voice if included, and any verified OS built-in fallback. Review naturalness, crisp consonants, intelligibility, pronunciation, pauses, missing/repeated words, clipping, and fatigue over 15 minutes.

Proposed acceptance: average at least 4/5 for clarity and listening comfort from three listeners on the agreed corpus; no systematic omitted/repeated words or audible clipping. Record listeners, equipment, text, model hash, and settings. If only the owner evaluates a private build, label that limitation rather than calling it a broader study.

Quantized and full-precision artifacts must be compared directly. An int8 file may reduce storage without improving every hardware path; do not make it the recommendation solely because it is smaller.

### 7.3 CPU and GPU requirements

- CPU mode must work without CUDA or a discrete GPU.
- GPU selection is explicit and reversible; CPU remains available.
- Auto mode benchmarks a small local sample with consent before selecting a device.
- Record model export, runtime version, execution provider, device, and any CPU fallback nodes.
- Candidate GPU providers include CUDA for compatible NVIDIA hardware, DirectML for compatible Windows GPUs, CoreML on eligible Macs, and OpenVINO where the exact Intel/export/OS combination is tested. Provider availability is not model compatibility; validate actual execution and CPU fallback nodes. Do not import the shared conversation’s `llama.cpp` GPU mapping into ONNX TTS. [Provider documentation](https://onnxruntime.ai/docs/execution-providers/)
- Never label a session “GPU” merely because the machine has a GPU.
- If GPU loading fails, offer CPU fallback and show the reason. Do not silently download large runtimes.
- On battery, Auto prefers Eco CPU unless the measured GPU path is more efficient.
- No model training, background benchmarking, or speculative model preloading.

### 7.4 Model capabilities and catalog

Catalog entries must include:

| Group | Fields |
|---|---|
| Identity | Catalog schema version, model ID, artifact version, engine adapter ID |
| Files | HTTPS URL, exact bytes, SHA-256, relative installation path |
| Compatibility | Runtime requirements, tested providers, architecture, phonemizer identity |
| Audio | Sample rate, channels, output sample format |
| Text | Languages, tokenizer identity, exact model token/phoneme limit |
| Voices | Single/list/system mode, IDs, labels, language, sample text |
| Parameters | Typed ID, label, min/max/step or enum values, default, help, cache effect |
| Rights | Engine and artifact license URLs, required attribution |
| Evidence | Benchmark report ID and recommendation status |

Do not hardcode “54 voices,” a fixed language count, a token limit, or “one voice per Piper file” as a universal engine rule. Catalog the actual artifact, including multi-speaker files when present.

The UI renders existing parameter types from metadata: number → slider plus numeric input; boolean → switch; enum → dropdown. Adding a supported parameter requires no new UI control code, but the engine adapter must actually understand and map it. Unknown parameters are rejected or hidden with diagnostics; a JSON edit cannot create an unsupported engine capability.

### 7.5 Download and activation lifecycle

1. User explicitly chooses Download after seeing size and license information.
2. Validate free disk space including temporary-file overhead.
3. Download into a versioned temporary directory with cancellation and bounded concurrency.
4. Resume only when server validators and range support establish that the partial file still matches.
5. Verify every hash before activation; never execute code from the model catalog.
6. Atomically mark the version installed after all required files verify.
7. Preserve the prior working version through failed downloads or updates.
8. Load on Select; provide explicit Preview after successful activation.
9. Deleting the active version first switches to an available fallback or stops playback with confirmation.
10. Model updates do not occur during a reading without an explicit user action.

---

[Previous](06-reading-entry-paths.md) · [Next](08-resource-budgets.md)
