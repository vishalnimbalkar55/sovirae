# 11. Global shortcuts and background behavior

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This file is synchronized with section 11 of the full specification.

### 11.1 Preserve the preferred shortcut set

Keep the action layout from the pasted Windows prompt. Most of these bindings were already in our spec; they do not require replacing the playback or capture architecture. Keep our additional Speak clipboard shortcut as the reliable fallback.

| Action | Windows default / Linux suggested binding | Proposed macOS equivalent |
|---|---|---|
| Speak clipboard | `Ctrl+Shift+R` | `Command+Option+R` |
| Read selected text | `Ctrl+Shift+S` | `Command+Option+S` |
| Play/pause | `Ctrl+Shift+Space` | `Command+Option+Space` |
| Skip back 10 seconds | `Ctrl+Shift+Left` | `Command+Option+Left` |
| Skip forward 10 seconds | `Ctrl+Shift+Right` | `Command+Option+Right` |
| Speed up | `Ctrl+Shift+Up` | `Command+Option+Up` |
| Slow down | `Ctrl+Shift+Down` | `Command+Option+Down` |
| Stop and hide player | `Ctrl+Shift+X` | `Command+Option+X` |
| Stop and hide while player has focus | `Escape` | `Escape` |

The macOS mappings are proposals, not tested conflict-free bindings; allow the original Control+Shift bindings if the user prefers. Linux shortcuts are suggestions because the desktop/portal may assign or require approval of the final chord. Cross-platform architecture remains a separate decision; this table does not make the current WPF shell portable.

### 11.2 Exact behavior

- Global shortcuts work while another application is focused when the platform permits registration.
- Read selection invokes only supported capture; unavailable capture explains the clipboard/extension fallback.
- Speak clipboard uses the explicit clipboard-read path. On platforms that deny background clipboard access, offer a focused Paste action or the extension instead.
- Play/pause toggles an existing session; it never implicitly rereads old clipboard content.
- Speed shortcuts change the current rate by 0.1 within 0.5–3.0, matching our existing control. They do not resynthesize audio.
- Skip uses our source-time timeline and existing sentence-snap setting. Keep precise seeking available rather than making snapping mandatory.
- Outside a reading session, playback-only actions do nothing; they do not open a window or capture text.
- Escape is local to the focused player. In a shortcut recorder it cancels recording; in the Chrome picker it cancels picking. Never reserve Escape globally.
- Ignore auto-repeat for Read selection, Speak clipboard, and Play/pause. Debounce held skip/speed keys to at most four actions per second.
- Application-global shortcuts and the Chrome long-press picker are separate features. A normal chord must cancel a pending picker hold timer.

### 11.3 Registration, customization, and verification

Use OS-supported registered shortcuts where sufficient. Introduce low-level hooks only for behavior that actually needs them, with fast callbacks and explicit cleanup. Do not inherit the pasted requirement that every shortcut use a Windows keyboard hook.

- Provide a recorder per action, cancel/reset, readable platform key labels, and duplicate-binding checks.
- Attempt native registration and report actual conflicts or permission denial; do not claim to enumerate every shortcut owned by another app.
- Store requested and effective bindings separately if the desktop remaps them.
- Pause hotkeys releases global bindings and allows another application to use them; re-enabling reports any newly introduced conflicts.
- Re-register after session/backend reconnection when required, without stealing an existing binding or repeatedly prompting for permission.
- Test after 15 minutes idle, sleep/wake, permission changes, and source-app switching.
- Verify triggering or using compact playback controls does not steal source-editor typing focus on supported platforms.

### 11.4 Background behavior retained

Tray/menu actions: Open SpeakIt, Play/Pause, Stop, Speak clipboard, Pause hotkeys, Settings, Exit. Do not read clipboard contents on a timer. Single-instance startup forwards an activation request to the existing instance.

“Background service” means a lightweight process in the signed-in user's desktop session, not a privileged system service. Keep running when settings closes, Start at login, and Start hidden are separate settings. Explicit Exit stops playback and owned workers. A desktop with no supported tray needs a reliable launcher/window route to reopen and quit; never leave an unreachable hidden process.

See the [selective comparison](../Windows-Prompt-Feature-Review.md) for platform constraints and features recommended for later adoption.

---

[Previous](10-floating-player.md) · [Next](12-chrome-extension.md)
