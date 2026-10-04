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
- [x] Add Kyutai Pocket TTS English (predefined voices; user request 2026-09-26).

**Gate:** the recommended quality voice passes quality/performance targets on declared hardware without making foreground work unusable; the Step 4 starter voice remains a working fallback.

## Progress — 2026-09-26

- Catalog pins revision, byte size, and SHA-256 for the model and 28 English voices. Downloads check free space, resume `.part` files only when the ETag still matches, verify every hash, and write `installed.json` last; the previous version stays usable until then.
- Inference runs in `speakit-kokoro-worker`, a separate process with ONNX Runtime linked in statically, intra-op threads from the resource profile (Balanced = 4 here), and idle spinning off. Cancellation or a crash replaces the worker. Idle model unload (2026-10-04): the worker exits once no sentence has been generated for the profile's idle time (Eco 2 min, Balanced 5 min, Performance 10 min, spec §8.2), including while a reading is paused or finished with its audio already prepared; the next sentence starts it again. The audio device closes separately, 30/60/180 s after a reading ends. Worker priority (2026-10-04): Eco and Balanced start workers at below-normal CPU priority (nice 10 on macOS, BELOW_NORMAL_PRIORITY_CLASS on Windows); Performance runs them at normal priority.
- End-to-end test through the real controller and audio output (`cargo test -p speakit end_to_end_kokoro -- --ignored`): first audio 1.75 s from a cold worker and 0.92 s warm.
- Voices screen: download with size and license shown, progress, cancel, retry, remove with confirmation, and Kokoro voices listed once installed. Open: GPU selection, memory-pressure fallback, and measured RAM.
- Multiple models: the catalog is data (`crates/speakit-models/catalog/models.json`, schema-checked by tests), the engine registry holds any number of installed models routed by voice prefix, and the Voices screen lists System voices plus every catalog model. Adding a model of a supported family is a catalog entry; see the catalog README.
- Voices screen now follows the requested layout: Language, Gender, and Voice dropdowns plus Preview at the top for the model in use, then a model list. Downloadable models show Download (with version choice), progress with Cancel, Use it, and Delete with confirmation; System voices show only Use it. Gender comes from the catalog for Kokoro and from macOS (AVSpeechSynthesisVoice) for system voices.
- All voices: the catalog now lists all 55 Kokoro voices (American and British English, Spanish, French, Hindi, Italian, Japanese, Brazilian Portuguese, Mandarin) and all 176 macOS system voices are shown. Verified: every Kokoro voice is installed and listed (including Echo and Eric), and one voice per language produces speech (`cargo run --release -p speakit-tts --example kokoro_languages`).
- Pronunciation outside English uses espeak-ng with misaki's general mapping. Japanese and Mandarin are labelled approximate: espeak-ng does not read kanji and Mandarin tone digits are dropped. misaki's Japanese and Chinese front ends are not ported.
- An installed model that gains catalog voices offers to download only the missing ones.
- Pocket TTS languages (2026-10-04, user request): the catalog now lists Kyutai's French, German, Spanish, Italian, Portuguese, and Dutch 6-layer models from the same pinned revision, each with its own tokenizer, weights, 27 voices, and the text rules from `pocket_tts/config/<language>.yaml`. Hashes come from the Hugging Face tree API (LFS ids) and from downloading each tokenizer. Not yet listened to; the worker was verified on French earlier.
- Studio (2026-10-04, user request): a separate screen for script projects. A project holds a script with iterations, its own voice (model and voice chosen separately from the reading voice), the markers that voice supports (app-level pauses for every voice; a model's own tags from the catalog `features`, none for the bundled models), a copyable prompt for ChatGPT or Claude that restricts an assistant to those markers, and the generated audio. Generation is paragraph by paragraph: the first part (the opening paragraph, or up to three short ones totalling about 200 characters) is generated first; the user listens, accepts, and the rest follows. Each paragraph is a 24 kHz WAV under `<app data>/studio/<project>/`; "Download first part" and "Download all" join them with a short gap through the save dialog. Expressions (`[laugh]`, `[chuckle]`, `[giggle]`, `[sigh]`, `[clear throat]`, `[gasp]`, `[hmm]`, also plural forms) are approximated for every voice by a spoken sound ("Ha ha ha!", "Ahem." …) and labelled approximate; a model that lists the same tag in `features` receives the tag instead. Other bracketed markers are removed before synthesis and counted per paragraph. Decision 2026-10-04: approximations now, a tag-capable model (Orpheus or CosyVoice 2 class) as a later step. The screen opens on a project list (cards with inline delete); an open project has a fixed left column (Back, voice, scrolling marker list, prompt) and a main area that scrolls on its own. Each generated paragraph stores its text; generating again reuses the audio of unchanged paragraphs and regenerates only edited, added, or failed ones (files are named by content, unreferenced ones removed; projects from before this change get their paragraph texts filled in from the script on load), and the editor marks changed paragraphs. After any edit the choices are "Regenerate first part" (always redoes the first part, then asks for acceptance again; later paragraphs that changed follow on accept) or "Regenerate all" (user decision 2026-10-04). Scripts are saved explicitly (Save button, ⌘S); leaving with unsaved changes asks Save / Discard / Cancel, and switching screens returns Studio to the list. Covered by `src-tauri/src/studio.rs` unit tests and an ignored end-to-end test with the system voice.
- Pronunciation rules (2026-10-04, user request): Settings › Pronunciation holds "written as → spoken as" rules (whole word, any case, phrases allowed, empty spoken form skips the word). They are applied per sentence to the text sent to the voice (`speakit-core::pronounce`), so the document on screen keeps its spelling. Stored in settings, capped at 500.

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
- Settings → Performance → **Use GPU** switch, off by default; it can be turned on only on Apple Silicon and Windows (disabled elsewhere). Only downloaded voices are affected; system voices are run by the OS.
- The player shows the device the worker actually loaded, never the requested one. If the GPU cannot load, or fails mid-reading and the CPU succeeds on the same segment, the engine continues on the CPU and Settings shows the reason. Changing the setting tries the GPU again. Covered by fake-worker tests in `speakit-tts`.
- Packaging: the worker links `libwebgpu_dawn.dylib` (8.7 MB, BSD-3-Clause), bundled in `Contents/Frameworks` and checked by `scripts/build.sh`. Windows uses CUDA on NVIDIA GPUs ([step 14](14-windows-support.md)); Intel Macs and Linux have no GPU path yet.

## Relevant requirements

- [7. Local TTS strategy](../chunks/07-local-tts-and-models.md)
- [8. Resource controls and measurable performance](../chunks/08-resource-budgets.md)

### Pocket TTS — 2026-09-26

Added at the user's request (huggingface.co/kyutai/pocket-tts).

- Source: `kyutai/pocket-tts-without-voice-cloning` at revision a5ce31f3 (ungated; the main repo requires a Hugging Face login). One catalog model, English only (user decision 2026-09-26): 6-layer weights (219 MB, bf16), tokenizer, and all 27 predefined voices (389 MB in total). Voice cloning is not included. The worker also handles Kyutai's other languages (verified on French below); each would be a catalog entry.
- Engine: `speakit-pocket-worker`, a Rust port of Kyutai's reference (FlowLM transformer, one-step LSD flow head, streaming Mimi decoder) on Candle with Apple Accelerate, in its own process with the same frame protocol and kill-on-cancel handling as Kokoro. No phonemizer: the model reads text, with each language's text rules from the catalog. Text is split and chunked exactly as the reference does (50-token chunks).
- Parity with Kyutai's Python package (pocket-tts at d299fb65, torch 2.14) at temperature 0: identical sample counts; correlation 0.99999995 (English, one chunk), 0.99999999 (English, two chunks with a decimal number and clause splitting), 0.99998 (French with character replacement).
- Measured on Apple M4, macOS 15.1 (`pocket_bench … en 4`): warm RTF 0.124–0.128 (about 8× real time); cold first sentence 1.6 s including worker start. Through the real controller and audio output (`cargo test -p speakit end_to_end_pocket -- --ignored`): first audio 2.0 s cold, 0.68 s warm. Peak memory footprint 0.58–0.60 GB; 2 and 4 threads perform the same, so Eco loses nothing. GPU: Kyutai reports no speedup on Apple Silicon; not attempted.
- Download, verification, install marker, and the Voices screen work unchanged (English installed through the verified downloader). The worker scales down a segment whose peak exceeds 0.98 instead of letting playback clip.
- Before public distribution: voices `cosette` and `jean` come from non-commercial datasets (CC-BY-NC-4.0), and the recordings behind `juergen` and `rafael` are undocumented. The catalog records each voice's license. Genders are set only for the 12 VCTK voices; the rest are listed under Other.
