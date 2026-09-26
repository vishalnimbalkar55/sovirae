# Step 3 — Create desktop shell

[Spec index](../README.md) · [All steps](README.md)

**Planning only. Do not start implementation without a separate user instruction.**

**Dependency:** Complete Step 2 and its gate first.

- [ ] Create project boundaries and dependency checks.
- [ ] Build MVVM navigation, theme resources, and settings persistence.
- [ ] Add tray, single-instance behavior, and explicit Exit.
- [ ] Add compact overlay positioning and focus behavior.
- [ ] Add hotkey registration and conflict feedback.

**Gate:** Notepad retains typing focus when the overlay appears; hotkeys remain reliable after 15 minutes idle and sleep/wake.

## Relevant requirements

- [4. Screen-by-screen behavior](../chunks/04-screens-and-settings.md)
- [5. Architecture and platform boundaries](../chunks/05-architecture.md)
- [11. Global shortcuts and background behavior](../chunks/11-shortcuts-and-background.md)
