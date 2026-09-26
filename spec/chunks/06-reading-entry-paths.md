# 6. Entry paths and exact user journeys

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: development in progress (macOS first).** This topic is synchronized with the full specification; original numbering is preserved.

### 6.1 Path A: clipboard hotkey — first delivery

1. User selects text in another app and copies it themselves.
2. User presses `Ctrl+Shift+R`.
3. SpeakIt reads Unicode text from the clipboard with a short bounded retry for clipboard contention.
4. Empty, non-text, or over-limit content produces a specific message.
5. Core starts a reading and the player appears without stealing typing focus.
6. SpeakIt does not modify the clipboard in this path.

The path depends on the source application permitting copy and the desktop allowing clipboard access when the shortcut fires. Offer a focused Paste action or Chrome extension route where background clipboard reads are restricted.

### 6.2 Path B: selected text without manual copy

1. User selects text and invokes the platform-mapped Read selection shortcut.
2. The platform adapter tries a supported accessibility selection API: Windows UI Automation, macOS Accessibility with consent, or Linux AT-SPI where exposed.
3. If the API fails, consider a synthetic Copy only when the OS permits it and all physical trigger modifiers have been released within a short timeout.
4. Preserve and restore clipboard contents only when the platform can do so safely; never overwrite a newer user copy. If formats or ownership cannot be preserved, skip this fallback.
5. If capture is unavailable, explain the manual-copy shortcut or Chrome extension route. Whole-document reading is always a separate explicit action.

Windows must verify that synthetic Copy does not open Chrome DevTools or leave modifiers stuck. macOS must handle Accessibility permission denial. Linux Wayland must not claim arbitrary cross-application selection access. Each result belongs in the compatibility matrix.

### 6.3 Path C: Chrome selection

1. User selects page text.
2. User chooses “Read selection with SpeakIt” from the context menu, a configured shortcut, or the popup.
3. Extension captures only the requested selection and minimal source metadata.
4. Worker establishes the native connection, validates the request, and sends it.
5. Desktop accepts or rejects; acceptance means the request is queued for preparation, not that sound already started.
6. Native floating player owns playback from that point onward.

### 6.4 Path D: Chrome element picker

1. User holds the configured Ctrl/Alt trigger on a permitted page.
2. After 450 ms, a visible cue arms the picker.
3. User releases the modifier; picker remains latched for selection.
4. Hover highlights a readable paragraph or block.
5. Mouse wheel or dedicated keyboard controls widen/narrow the candidate scope.
6. Clicking the candidate freezes selection and shows Listen, Copy text, Back, and Close.
7. Listen sends a snapshot to the app; Copy text writes exactly the previewed extraction to the clipboard after an explicit click.
8. Back returns to hover selection. Close or Escape exits completely.

Latching resolves the ambiguity in the pasted flow: a user must be able to release Ctrl/Alt and still click Listen. The Copy text action is included explicitly because the user referred to copy-text extension behavior.

---

[Previous](05-architecture.md) · [Next](07-local-tts-and-models.md)
