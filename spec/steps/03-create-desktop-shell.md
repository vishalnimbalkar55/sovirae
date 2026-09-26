# Step 3 — Create desktop shell

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 2 and its gate first.

- [x] Create Tauri 2 + React/TypeScript shell, Rust crate boundaries, command/event schemas, and permission scopes.
- [ ] Build React navigation, theme resources, and Rust-owned settings persistence.
- [ ] Add tray/menu bar, single-instance behavior, start-at-login option, and explicit Exit.
- [ ] Spike compact overlay positioning/focus on Windows, macOS, Linux X11, and a named Wayland session.
- [ ] Add global shortcut registration, conflict/permission feedback, and fallback actions.

**Gate:** the source editor retains typing focus on every supported platform, or the platform restriction is documented with a tested fallback; hotkeys survive 15 minutes idle and sleep/wake where registration is available. Document Wayland limitations.

## Progress — 2026-09-26

- Implemented but not yet checked by hand: menu-bar icon and menu, single instance, start at login, Exit, and settings persistence.
- Focus spike (macOS): the player appears without taking focus (`orderFrontRegardless`). Clicking its buttons still activates SpeakIt. Re-classing the window as a non-activating `NSPanel` crashed, because Tauri's window class is larger than `NSPanel`, so a different approach is needed.
- Global shortcuts: Speak clipboard and Read selection are always registered; playback shortcuts are registered only during a reading (automated test confirms they are released after Stop). Not yet pressed in a live session.

## Relevant requirements

- [4. Screen-by-screen behavior](../chunks/04-screens-and-settings.md)
- [5. Architecture and platform boundaries](../chunks/05-architecture.md)
- [11. Global shortcuts and background behavior](../chunks/11-shortcuts-and-background.md)
