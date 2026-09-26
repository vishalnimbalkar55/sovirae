# Step 8 — Add best-effort desktop capture

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 7 and its gate first.

- [ ] Implement platform selection adapters: Windows UI Automation, macOS Accessibility where permitted, and Linux AT-SPI where available.
- [ ] Add modifier-release waiting and bounded synthetic-copy fallback.
- [ ] Handle clipboard format limitations and concurrent user copies.
- [ ] Test representative apps and OS permission/compositor restrictions; document unsupported capture and fallback routes.
- [ ] Keep manual copy as an explicit fallback.

**Gate:** no Chrome DevTools launch, stuck modifiers, unexpected whole-document reading, or overwritten later user clipboard content.

## Relevant requirements

- [5. Architecture and platform boundaries](../chunks/05-architecture.md)
- [6. Entry paths and exact user journeys](../chunks/06-reading-entry-paths.md)
- [11. Global shortcuts and background behavior](../chunks/11-shortcuts-and-background.md)
