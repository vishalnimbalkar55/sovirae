# Step 8 — Add best-effort desktop capture

[Spec index](../README.md) · [All steps](README.md)

**Planning only. Do not start implementation without a separate user instruction.**

**Dependency:** Complete Step 7 and its gate first.

- [ ] Implement UI Automation capture.
- [ ] Add modifier-release waiting and bounded synthetic-copy fallback.
- [ ] Handle clipboard format limitations and concurrent user copies.
- [ ] Test standard apps and elevated/protected failures.
- [ ] Keep manual copy as an explicit fallback.

**Gate:** no Chrome DevTools launch, stuck modifiers, unexpected whole-document reading, or overwritten later user clipboard content.

## Relevant requirements

- [6. Entry paths and exact user journeys](../chunks/06-reading-entry-paths.md)
- [11. Global shortcuts and background behavior](../chunks/11-shortcuts-and-background.md)
