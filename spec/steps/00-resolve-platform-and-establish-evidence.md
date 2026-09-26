# Step 0 — Resolve platform and establish evidence

[Spec index](../README.md) · [All steps](README.md)

**Planning only. Do not start implementation without a separate user instruction.**

**Dependency:** None; resolve these assumptions before development.

- [ ] Confirm Windows-first target and actual CPU/GPU hardware.
- [ ] Confirm English-first scope and any required Hindi/other voices.
- [ ] Record private versus public distribution intent.
- [ ] Pin SDK/runtime candidates and catalog schema.
- [ ] Check exact model/export/phonemizer/provider compatibility and licenses.
- [ ] Create benchmark corpus and result template.

**Gate:** written architecture decisions; no vague “GPU supported” claim.

## Relevant requirements

- [1. Purpose and source of truth](../chunks/01-overview-and-assumptions.md)
- [5. Architecture and platform boundaries](../chunks/05-architecture.md)
- [7. Local TTS strategy](../chunks/07-local-tts-and-models.md)
- [22. Decisions to confirm before implementation](../chunks/22-decisions-to-confirm.md)
