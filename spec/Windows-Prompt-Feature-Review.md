# Windows prompt — What to reuse in SpeakIt

[Spec index](README.md) · [Preferred shortcuts](chunks/11-shortcuts-and-background.md)

**Status: analysis and specification only. No implementation started.**

## 1. Recommendation

Reuse useful behavior from the pasted prompt, not its Windows-only dependency list. Our current spec already contains most of its strongest features and improves resource control, clipboard safety, model management, and the Chrome workflow.

The shortcut layout is worth keeping. It was already largely present in our spec; section 11 now makes its behavior explicit and adds proposed macOS mappings. Other additions below are recommendations, not an instruction to implement or a claim that they already exist.

Important distinction: this workspace contains specifications, not a working SpeakIt application. “Our version is stronger” describes a better-defined requirement or safer proposed design, not measured software quality.

## 2. Keep from the pasted version

| Feature | Current spec | Decision |
|---|---|---|
| Ctrl+Shift+S to read selection | Already included | Keep the exact Windows binding |
| Ctrl+Shift+Space to play/pause | Already included | Keep |
| Ctrl+Shift+Left/Right to skip | Already included | Keep |
| Ctrl+Shift+Up/Down to change speed | Already included | Keep; specify 0.1× steps |
| Ctrl+Shift+X to stop/hide | Already included | Keep |
| Escape when player has focus | Not explicit in the player shortcut table | Add; never intercept Escape globally |
| Shortcut recorder and reset | Already included | Keep; report real registration conflicts |
| Floating player without stealing focus | Already included | Keep as a platform verification gate |
| Expanded text with sentence highlighting/click-to-seek | Already included | Keep |
| Background running, tray, and single instance | Already included | Keep; use a per-user desktop process |
| Synthesize ahead while playing | Already included | Keep our bounded scheduler |
| Pitch-preserving speed without resynthesis | Already included | Keep |
| Audio-device change recovery | Already included | Keep and test on each OS |
| Model downloader with hash verification | Already included | Keep our stricter lifecycle |
| Offline TTS and a headless core | Already included | Keep |

These are not reasons to copy the pasted build order or replace the current architecture wholesale.

## 3. Useful additions to consider next

### A. Pronunciation dictionary — highest-value new feature

The pasted prompt explicitly includes a word → spoken form dictionary. Our current spec does not define its management UI in detail.

Suggested scope:

- Add, edit, delete, import, and export local dictionary entries.
- Support literal whole-word or phrase matching, a language filter, and a case-sensitivity option.
- Do not expose arbitrary executable replacement rules.
- Preview a replacement before saving.
- Preserve original display text and source-span mappings while changing spoken text.
- Include dictionary version in synthesis cache keys.
- Warn that changes affect newly generated audio; do not mutate buffered playback silently.

**Recommendation:** Add after the basic text pipeline, before broad pronunciation polishing. This works across platforms and needs no LLM.

### B. Clear reading-rule settings

The pasted cleaning examples are more granular than our current reading settings. Reuse that specificity, with conservative defaults.

Suggested controls: URLs (read/say “link”/skip), fenced code (read/announce line count/skip), citation markers, abbreviation list, and acronym pronunciation exceptions. Keep cleaning, normalization, and segmentation independently testable.

Do not automatically apply all examples to every source. Underscores can be meaningful, bracketed numbers can be content, and hyphenated line breaks can be real words. Currency/date/unit rules must be locale-aware. Repeated PDF-header removal requires page context that plain clipboard text may not contain.

**Recommendation:** Add configurable rules and table-driven tests. Keep plain-text preservation as the safe baseline; do not silently strip all emoji or mathematical/table content.

### C. System media keys and media controls

Our delivery plan already mentions this as optional; the pasted version specifies it more clearly.

Use play/pause and skip actions through an OS adapter. Candidate integrations: Windows system media controls, macOS media-command APIs, and Linux MPRIS. Verify each integration before promising parity.

Use a generic title such as “SpeakIt reading” by default. Do not copy the first 60 characters of private text into lock-screen/media metadata automatically.

**Recommendation:** Keep as playback polish after audio recovery; do not block the core on it.

## 4. Keep our stronger requirements

| Topic | Pasted approach | Why keep ours |
|---|---|---|
| Reliable first use | Auto-capture is an early dependency | Manual input and explicit clipboard reading deliver value sooner |
| Resource use | Keep all PCM up to 512 MB, then spill to temp | Our profiles bound buffers, concurrency, lookahead, and idle model lifetime |
| Clipboard safety | Always overwrite clipboard with the old snapshot | Ours preserves a newer user copy and skips unsafe restoration |
| No selection | Read the whole document automatically | Ours avoids unexpected capture/readout of unrelated content |
| Modifier handling | Inject Ctrl+C from a held hotkey | Ours waits for modifiers to release before attempting safe capture |
| Downloads | Unprompted audio after installation | Our explicit Preview avoids unexpected sound |
| Model quality | Hardcoded quality dots and “best” claims | Our benchmark/listening gates require evidence |
| Catalog | Stable hardcoded voice lists | Our metadata tracks the actual installed artifact |
| Progress | All durations appear known in the central record | Ours separates estimates from synthesized timing |
| Seek | Always snap backwards | Ours permits precise seeking and an explicit sentence-snap setting |
| Long documents | Large session PCM retained by default | Ours can evict and regenerate audio while retaining sentence metadata |
| Diagnostics | Log captured text during capture testing | Ours keeps reading content out of ordinary logs |
| Browser | Generic capture fallback | Our extension adds visible selection, scope control, Copy text, and native messaging |

## 5. Leave out or correct

- **Windows-only / “no abstractions” instruction:** conflicts with the user's macOS/Linux requirement. Do not carry it forward as a product constraint.
- **Exact WPF/NAudio/Win32 stack on all OSes:** cannot be copied as a portable implementation. Keep Windows-specific APIs inside their eventual adapters.
- **Low-level keyboard hook for every shortcut:** the preferred chord bindings do not inherently require modifier-only interception. Choose supported platform registration mechanisms.
- **Silent whole-document fallback:** leave out; offer an explicit Read document action if requested later.
- **Universal clipboard restoration:** leave out; delayed clipboard formats and concurrent copies require conditional handling.
- **Automatic preview playback:** leave out.
- **Universal espeak-only phonemizer claim:** validate the specific model's reference pipeline and vocabulary rather than assuming every TTS engine takes identical input.
- **KittenTTS routed through a Kokoro adapter:** do not assume compatible signatures, tokenization, or voice data. Evaluate it separately only if existing models fail quality/resource goals.
- **Separate process/assembly as an automatic licensing solution:** do not make this claim. Record the licenses of the actual distributed components and review the chosen packaging.
- **Placeholder hashes in a release catalog:** development tooling can compute hashes, but published manifests must contain verified pinned artifacts. A hash alone does not establish a trustworthy download source.
- **“Any application” and “on top of everything”:** replace with a tested compatibility matrix, honest restrictions, and recovery paths.
- **Fixed milestone order that starts risky capture before useful reading:** retain our clipboard-first plan and early model feasibility evaluation.

## 6. Hotkeys, background running, and floating windows across OSes

The same user-facing actions are the goal. Their native mechanisms are different.

| Behavior | Windows | macOS | Linux |
|---|---|---|---|
| Global shortcuts | Registered hotkeys; narrow hook use only if needed | Native shortcut registration, with permissions if the chosen mechanism needs them | X11 registration or supported GlobalShortcuts portal; compositor-dependent |
| Background running | User-session app with tray | User-session app with menu-bar access | User-session app; tray support varies by desktop |
| Start at login | Per-user startup registration | User-controlled login item | Desktop autostart or supported background portal |
| Floating player | Native non-activating window behavior | Native non-activating panel behavior | Window-manager hints on X11; Wayland capabilities vary |
| Selection capture | UI Automation and safe fallback | Accessibility APIs where permitted | AT-SPI where supported; not universal |
| Reliable fallback | Explicit copy/clipboard or extension | Explicit copy/clipboard or extension | Extension or focused Paste if background clipboard access is restricted |

Apple documents non-activating panels and Accessibility trust checking; these provide implementation paths, not proof that every target application exposes selected text. [Non-activating panel](https://developer.apple.com/documentation/appkit/nswindow/stylemask-swift.struct/nonactivatingpanel), [Accessibility trust](https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions)

Linux offers a GlobalShortcuts portal, but availability depends on the desktop's portal backend. Do not assume X11 shortcuts can control native Wayland applications. Treat missing support as a reported capability, with desktop-assigned shortcuts or extension controls where available. [GlobalShortcuts portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html)

Background behavior belongs to the logged-in desktop session; no privileged system daemon is required for reading. Linux autostart and portal-based background permission are separate mechanisms to select according to packaging. [Desktop autostart](https://specifications.freedesktop.org/autostart/latest/), [Background portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Background.html)

For a later architecture decision, **Avalonia plus a shared C# core and native platform adapters** is a reasonable candidate. It supports Windows, macOS, and Linux, but its documented Linux backends and tray support have qualifications. Merely changing the UI framework does not solve global capture or overlay restrictions. [Supported platforms](https://docs.avaloniaui.net/docs/supported-platforms), [Tray support](https://docs.avaloniaui.net/controls/navigation/trayicon)

No framework migration is approved or implemented by this review. The Windows-first sections in the existing spec need reconciliation before cross-platform development begins.

## 7. Changes made now versus recommendations

**Updated in the spec now:** preserved the preferred shortcut table, added focused-player Escape, proposed macOS equivalents, and clarified shortcut repeat, conflict, pause, and background behavior. Updated the full document and its shortcut chunk together.

**Recommended for a later spec update:** pronunciation dictionary, more explicit reading-rule controls, and platform media-control details.

**Kept unchanged:** resource strategy, text/index architecture, playback design, capability-driven model UI, extension workflow, and the current technology decision pending a separate cross-platform revision.

**Not performed:** application coding, scaffolding, installing runtime dependencies, migrating frameworks, or claiming tested cross-platform parity.
