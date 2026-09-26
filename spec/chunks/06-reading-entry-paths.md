# 6. Entry paths and exact user journeys

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This file is a topic-sized extract of the full specification. Original requirement numbering is preserved.

### 6.1 Path A: clipboard hotkey — first delivery

1. User selects text in another app and copies it themselves.
2. User presses `Ctrl+Shift+R`.
3. SpeakIt reads Unicode text from the clipboard with a short bounded retry for clipboard contention.
4. Empty, non-text, or over-limit content produces a specific message.
5. Core starts a reading and the player appears without stealing typing focus.
6. SpeakIt does not modify the clipboard in this path.

The path depends on the source application permitting copy. Do not promise compatibility with protected content or every elevated application.

### 6.2 Path B: selected text without manual copy

1. User selects text and presses `Ctrl+Shift+S`.
2. Try supported UI Automation selection retrieval first.
3. If selection retrieval fails, wait for the triggering physical modifier keys to be released, with a short timeout.
4. Only then attempt a synthetic copy in the foreground application, if clipboard preservation is safe.
5. Never synthesize copy while Shift/Alt/Windows remain physically held; abort with the clipboard-path hint on timeout.
6. Observe clipboard sequence changes and restore prior contents only if the clipboard still contains SpeakIt's capture result.
7. If the user copied something else meanwhile, preserve that new content.
8. If prior clipboard formats cannot be preserved reliably, skip synthetic capture and ask the user to copy manually.
9. If nothing is selected, say so. Whole-document reading is an explicit separate action, never a silent fallback.

This replaces the pasted blanket “release and restore all modifiers” recipe, which risks stuck or inconsistent keyboard state. The implementation gate includes ordinary Ctrl/Shift shortcuts immediately after capture and verification that Chrome DevTools never opens.

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
