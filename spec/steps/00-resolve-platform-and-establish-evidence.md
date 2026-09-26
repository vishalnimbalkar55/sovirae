# Step 0 — Resolve platform and establish evidence

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** None; resolve these assumptions before development.

- [ ] Confirm exact Windows, macOS, and Linux release versions/architectures and representative CPU/GPU hardware.
- [x] Confirm English-first scope and any required Hindi/other voices.
- [ ] Record private versus public distribution intent.
- [ ] Pin SDK/runtime candidates and catalog schema.
- [ ] Check exact model/export/phonemizer/provider compatibility and licenses.
- [ ] Create benchmark corpus and result template.

**Gate:** written architecture decisions; no vague “GPU supported” claim.

## Progress — 2026-09-26

- macOS 15.1 on Apple M4 (10 cores, 24 GB) is the first build and test machine; Windows and Linux versions are still open.
- Pinned so far: Tauri 2.11, Rust 1.98, cpal 0.18, React 19, Vite 8. Model catalog schema, licenses, and benchmark corpus are not started.

## Relevant requirements

- [1. Purpose and source of truth](../chunks/01-overview-and-assumptions.md)
- [5. Architecture and platform boundaries](../chunks/05-architecture.md)
- [7. Local TTS strategy](../chunks/07-local-tts-and-models.md)
- [22. Decisions to confirm before implementation](../chunks/22-decisions-to-confirm.md)
