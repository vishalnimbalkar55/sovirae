# 11. Global shortcuts and background behavior

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This file is a topic-sized extract of the full specification. Original requirement numbering is preserved.

| Action | Proposed Windows default |
|---|---|
| Speak clipboard | `Ctrl+Shift+R` |
| Capture selected text | `Ctrl+Shift+S` |
| Play/pause | `Ctrl+Shift+Space` |
| Back/forward 10 seconds | `Ctrl+Shift+Left/Right` |
| Speed down/up | `Ctrl+Shift+Down/Up` |
| Stop and hide | `Ctrl+Shift+X` |

Use registered global hotkeys where sufficient. Introduce low-level hooks only for behavior that actually needs them, with fast callbacks and explicit cleanup. Configurable defaults can conflict with other apps; show registration failure and let the user remap.

Tray menu: Open SpeakIt, Play/Pause, Stop, Speak clipboard, Pause hotkeys, Settings, Exit. Do not read clipboard contents on a timer. Single-instance startup forwards an activation request to the existing instance.

---

[Previous](10-floating-player.md) · [Next](12-chrome-extension.md)
