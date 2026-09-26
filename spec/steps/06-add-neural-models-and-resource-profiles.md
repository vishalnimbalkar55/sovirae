# Step 6 — Add neural models and resource profiles

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 5 and its gate first.

- [x] Implement catalog validation and verified downloads.
- [ ] Build model state transitions and metadata-driven controls.
- [x] Integrate the chosen Kokoro engine in the inference worker.
- [ ] Add Eco/Balanced/Performance and explicit CPU/GPU selection.
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

## Relevant requirements

- [7. Local TTS strategy](../chunks/07-local-tts-and-models.md)
- [8. Resource controls and measurable performance](../chunks/08-resource-budgets.md)
