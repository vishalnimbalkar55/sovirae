# Step 3 — Create desktop shell

[Spec index](../README.md) · [All steps](README.md)

**Planning only. Do not start implementation without a separate user instruction.**

**Dependency:** Complete Step 2 and its gate first.

- [ ] Create Tauri 2 + React/TypeScript shell, Rust crate boundaries, command/event schemas, and permission scopes.
- [ ] Build React navigation, theme resources, and Rust-owned settings persistence.
- [ ] Add tray/menu bar, single-instance behavior, start-at-login option, and explicit Exit.
- [ ] Spike compact overlay positioning/focus on Windows, macOS, Linux X11, and a named Wayland session.
- [ ] Add global shortcut registration, conflict/permission feedback, and fallback actions.

**Gate:** the source editor retains typing focus on every supported platform; hotkeys survive 15 minutes idle and sleep/wake where registration is available. Document Wayland limitations.

## Relevant requirements

- [4. Screen-by-screen behavior](../chunks/04-screens-and-settings.md)
- [5. Architecture and platform boundaries](../chunks/05-architecture.md)
- [11. Global shortcuts and background behavior](../chunks/11-shortcuts-and-background.md)
