# Model catalog

`models.json` lists every downloadable voice model. The app shows each entry
on the Voices screen with Download, Use it, and Delete.

## Adding a Hugging Face model

1. Pin a full 40-character commit (`revision`); never use `main`.
2. For every file, record its exact `bytes` and `sha256`. Hugging Face's API
   returns both as LFS metadata:
   `https://huggingface.co/api/models/<repo>/tree/<revision>/<folder>`
3. Pick a unique `voicePrefix`. Voice IDs become `<voicePrefix>:<voice id>`.
4. List each voice with `language` (BCP-47, e.g. `en-US` or `fr`) and
   `gender` (`female`, `male`, or omit when the provider does not say). When
   a voice was made from a recording under another license than the model,
   record it in the voice's `license` (SPDX ID, or `unverified`).
5. Set `family` to an engine the app already has (`kokoro`, `pocket`). A new
   family also needs an adapter in `crates/speakit-tts`, a worker, and match
   arms in `src-tauri/src/models.rs` (`engine_for` and `worker_for`).
   Files every artifact needs (a tokenizer) go in the model's `files`;
   engine settings go in `options`, which the worker receives as JSON.
6. Run `cargo test -p speakit-models`; it rejects duplicate IDs or prefixes,
   unpinned revisions, and malformed hashes.

Record the model's license and link it in `licenseUrl`. Do not add a model
before its engine license and voice licenses are reviewed (spec §7.1).

## Pocket TTS

`pocket-tts-en` is Kyutai's English 6-layer model (`languages/english`, the
same weights as `english_2026-09`) from
`kyutai/pocket-tts-without-voice-cloning` (ungated, CC-BY-4.0), with its
tokenizer and all 27 predefined voices. Only English is listed (user
decision, 2026-09-26). Kyutai also ships French, German, Spanish, Italian,
Portuguese, and Dutch; the worker already supports them. Adding one takes
a catalog entry whose `options` copy the text rules (`remove_semicolons`,
`replace_characters`) from `pocket_tts/config/<language>.yaml` in
github.com/kyutai-labs/pocket-tts.

Voice genders are set only for the VCTK speakers, whose dataset documents
them. Voice licenses come from the kyutai/tts-voices card and the
reference's `_ORIGINS_OF_PREDEFINED_VOICES`: `cosette` (Expresso) and `jean`
(EARS) are CC-BY-NC-4.0, and the sources of `juergen` and `rafael` are not
documented. Resolve both before public distribution.
