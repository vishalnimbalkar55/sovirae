# 4. Screen-by-screen behavior

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This topic is synchronized with the full specification; original numbering is preserved.

### 4.1 Read screen

1. Show an editable multiline text area with placeholder “Paste text to read aloud.”
2. Provide Paste, Clear, and Listen actions; Paste reads the clipboard only when clicked.
3. Show word count and an explicitly approximate duration.
4. Disable Listen when text is empty or whitespace-only.
5. On Listen, validate length, create a session, and display the floating player.
6. Keep the draft available while the app remains open. Do not persist it across exits by default.
7. If already reading, the button label becomes “Read this instead.”
8. Show actual selected voice and execution device, with a link to change them.

### 4.2 Voices screen

1. Display model cards with name, description, languages, installed/download size, license link, and supported devices.
2. Clearly distinguish download size, disk size, and observed memory usage.
3. Mark recommended voices only after evaluation; no invented quality grades.
4. Allow Download, Cancel, Retry, Select, Preview, Update, and Delete according to state.
5. Expand configuration beneath the selected model only.
6. Show language selector only when multiple tested languages exist.
7. Filter voices by language; show speaker selector only for multi-speaker models.
8. Provide explicit Preview buttons. Never play audio automatically after downloading.
9. A preview pauses the current reading, uses the same audio output, and restores the prior session in paused state afterward.
10. Put synthesis parameters in Advanced, with a clear cache-invalidation explanation.
11. Put ordinary playback speed and volume in a separate playback section.
12. Do not let selecting a model discard a functioning engine until the replacement has loaded successfully.

Model status uses separate dimensions to avoid impossible combined states:

| Dimension | Values |
|---|---|
| Installation | Missing, Downloading, Verifying, Installed, Failed |
| Activation | Inactive, Loading, Active, LoadFailed |
| Update | Current, UpdateAvailable |

Exactly one engine is active after successful initialization; zero is valid while none are available. An active installed model can also have an update available. Failed updates leave the old version usable.

### 4.3 Shortcuts screen

Each row includes action, current chord, Record, and Reset. Recording provides Cancel and a visible listening state. Reject bare keys for global actions; identify duplicate app bindings; report registration failures from the OS. Do not claim to detect every shortcut used by every other application.

### 4.4 Extension screen

Show bridge installation health, last connection, authorized extension identity, connection status, Revoke, installation instructions, and Test connection. “Not connected” must not be reported as “not installed” without evidence. Display separate checks for extension connection, native host registration, and desktop readiness.

### 4.5 Settings screen

Sections: Appearance, Playback, Performance, Background behavior, Privacy and storage, Diagnostics.

| Setting | Default | Behavior |
|---|---|---|
| Theme | System | Light/dark manual override |
| Reading font size | 18 | Applies to expanded reading view |
| Playback speed | 1.0× | Range 0.5–3.0×, step 0.1 |
| Volume | 80% | Range 0–100%; independent of OS master volume |
| Execution device | CPU | Auto/GPU available after compatibility checks |
| Resource profile | Balanced | Eco/Balanced/Performance |
| Start at login | Off | Per-user startup mechanism on each supported OS |
| Keep running when windows close | On | Closing settings leaves tray/player available |
| Start minimized at login | Off | Enabled only when startup is enabled |
| Pause global hotkeys | Off | Also available in tray menu |
| Remember reading history | Off | Opt-in local persistence |
| Persistent audio cache | Off | Memory cache only initially |
| Diagnostic logging | Errors | No source text or page titles by default |

Explicit Exit always stops playback and terminates all owned workers. Closing the main window exits when background mode is off. The player close action stops and hides the current reading but does not independently terminate the application.

---

[Previous](03-ui-design.md) · [Next](05-architecture.md)
