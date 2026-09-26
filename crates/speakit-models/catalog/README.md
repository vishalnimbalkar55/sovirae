# Model catalog

`models.json` lists every downloadable voice model. The app shows each entry
on the Voices screen with Download, Use it, and Delete.

## Adding a Hugging Face model

1. Pin a full 40-character commit (`revision`); never use `main`.
2. For every file, record its exact `bytes` and `sha256`. Hugging Face's API
   returns both as LFS metadata:
   `https://huggingface.co/api/models/<repo>/tree/<revision>/<folder>`
3. Pick a unique `voicePrefix`. Voice IDs become `<voicePrefix>:<voice id>`.
4. List each voice with `language` (BCP-47, e.g. `en-US`) and `gender`
   (`female`, `male`, or omit when the provider does not say).
5. Set `family` to an engine the app already has (`kokoro`). A new family
   also needs an adapter in `crates/speakit-tts` and a match arm in
   `src-tauri/src/models.rs` (`engine_for` and `missing`).
6. Run `cargo test -p speakit-models`; it rejects duplicate IDs or prefixes,
   unpinned revisions, and malformed hashes.

Record the model's license and link it in `licenseUrl`. Do not add a model
before its engine license and voice licenses are reviewed (spec §7.1).
