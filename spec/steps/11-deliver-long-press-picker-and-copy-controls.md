# Step 11 — Deliver long-press picker and copy controls

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 10 and its gate first.

- [ ] Implement trigger state machine and editable-field exclusions.
- [ ] Add hover highlight and candidate discovery.
- [ ] Add wheel/keyboard scope controls.
- [ ] Add latched confirmation with text preview.
- [ ] Implement Listen, Copy text, Back, and Close.
- [ ] Add safe extraction, frame ownership, and per-site disabling.
- [ ] Verify ordinary Ctrl/Alt shortcuts are preserved.

**Gate:** documented results across at least ten representative sites/fixtures and all picker cancellation paths.

## Relevant requirements

- [12. Chrome extension specification](../chunks/12-chrome-extension.md)
- [16. Accessibility requirements](../chunks/16-accessibility.md)
