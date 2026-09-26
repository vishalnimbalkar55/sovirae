# 12. Chrome extension specification

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This topic is synchronized with the full specification; original numbering is preserved.

### 12.1 Permissions and enablement

Two supported access modes:

1. **On demand:** `activeTab` and scripting access after an explicit extension action or command; selection/context-menu reading and click-to-arm picker.
2. **Always ready on approved sites:** user-granted optional HTTP/HTTPS host access, with registered content scripts so long-press detection is already present.

Do not require all-site access merely to use selection reading. Explain that a long press cannot be detected before a content script is permitted and loaded. `activeTab` is temporary access from supported user gestures. [Chrome activeTab documentation](https://developer.chrome.com/docs/extensions/develop/concepts/activeTab)

Proposed permissions: `nativeMessaging`, `storage`, `activeTab`, `scripting`, `contextMenus`; optional HTTP/HTTPS host permissions. Add further permissions only for a concrete tested need.

Master Off disables input listeners and active pickers, but retains the lightweight control channel needed to receive an enable/settings change. This corrects the impossible requirement that an off extension have no listeners of any kind.

Persist options in `chrome.storage.local`, not sync, to preserve the local-only preference. Incognito disabled by default; if explicitly enabled later, never persist that browsing text/history.

### 12.2 Popup and options

Popup contains:

- Master On/Off.
- Current-site status and access request/revoke action.
- Read selection and Pick text buttons.
- Trigger summary with link to options.
- Desktop connection status and Open SpeakIt/Test connection action.
- Concise unsupported-page or permission error.

Options contain trigger, hold duration 300–1000 ms with 450 ms default, per-origin enablement, shortcut help, and reset. Per-site disable immediately cancels an active picker in that origin.

### 12.3 Trigger state machine

`Disabled → Idle → Holding → Picking → Confirming → Sending → Idle`

Cancel paths return to Idle; Back from Confirming returns to Picking.

Rules:

1. Default provisional gesture is long-press left Ctrl to reflect the user's request. Offer Alt and shortcut-only alternatives.
2. Ignore auto-repeat, IME composition, and events in password fields or editable controls.
3. If another key is pressed during Holding, cancel the timer and preserve the normal shortcut.
4. Ctrl+C, Ctrl+V, Ctrl+L, Ctrl+click, Alt+Tab, and browser zoom must keep their normal behavior outside an already armed picker.
5. Ignore AltGr/composed right-Alt combinations by default; they are needed for normal international typing.
6. Arm after threshold, show a cue, then latch on release. Do not require the user to keep holding the modifier while operating buttons.
7. Blur, tab hidden, navigation, Escape, master disable, and 15 seconds inactivity cancel.
8. Register capture-phase handlers, but acknowledge that a content script cannot guarantee precedence over browser/OS shortcuts or hostile pages.
9. If Alt release still opens browser chrome on a tested configuration, direct the user to Ctrl or shortcut-only mode; `preventDefault` is not a universal OS-menu guarantee.
10. A Chrome command also arms the picker for keyboard users and permission-limited mode.

### 12.4 Candidate discovery and scope

- Hit-test at the pointer and walk ancestors to a readable text block.
- Prefer paragraph, list item, blockquote, heading, or article content.
- Exclude hidden, inert, password, form-entry, navigation, and extension-owned nodes by default.
- Accept short headings and labels when deliberately selected; the pasted fixed 80-character minimum is a heuristic, not an absolute ban.
- Avoid choosing the full body immediately when a smaller meaningful block exists.
- Show element role/tag, text count, and scope indicator.
- Wheel up widens to a readable ancestor; wheel down returns through candidate history. Do not choose an arbitrary child.
- Intercept wheel only during active picking. Restore normal page scrolling immediately on exit.
- Keyboard equivalents: Up widens, Down narrows, Enter confirms, Escape exits; provide visible instructions.
- Cache candidate text during hover and throttle hit-testing with animation frames; do not scan the whole DOM continuously.

### 12.5 Highlight and confirmation UI

- One extension host and shadow root per participating frame.
- Fixed-position overlay, pointer-transparent outside actionable controls.
- Two-pixel accent border, subtle fill, 6-pixel radius, position updated on scroll/resize.
- Read live bounds; avoid changing classes or styles on the page's own elements.
- Shadow DOM reduces style collisions but is not a security boundary or a guarantee against transformed roots, fullscreen/top-layer UI, or deliberate page interference.
- Confirmation panel is clamped to the viewport and flips placement when needed.
- Show a short extracted-text preview, word/character count, approximate duration, and Listen/Copy text/Back/Close.
- Freeze the captured snapshot on confirmation; if the source changes, continue with that preview or explicitly refresh it.
- Consume relevant pointer/click events only while selecting, to prevent accidental links from opening. Include `click`/`auxclick` behavior in tests, not only `mousedown`.
- Copy success shows “Copied”; failure offers manual selection of the preview. No silent clipboard writes.

### 12.6 Extraction and page limitations

Extract visible text from the live rendered document using a tested traversal or live `innerText` strategy. Do not assume `innerText` on a detached clone preserves visibility and layout: detached elements can behave like `textContent`. [MDN innerText documentation](https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement/innerText)

Exclude script/style/noscript content, extension UI, hidden/ARIA-hidden nodes, and ignored site regions. Preserve meaningful headings, list separation, and paragraph boundaries. Layout inspection belongs in the extension; cleaning and segmentation belong in Core.

Supported: ordinary permitted HTML text, selections, readable blocks, and accessible open shadow roots where tested. Unsupported or conditional: browser internal pages, Chrome Web Store pages, built-in PDF viewer, canvas-only text, closed shadow roots, inaccessible frames, and permission-denied origins. Provide manual copy plus `Ctrl+Shift+R` as the fallback when copying is permitted.

### 12.7 Frames and multiple tabs

- Inject only into frames for which permission and browser rules allow it; `all_frames` does not bypass origin access.
- Each frame extracts its own text.
- Route coordination through extension runtime messages with sender tab ID, frame ID, and document ID, rather than trusting page `postMessage` payloads.
- Keep a single active picker owner per tab; pointer entry into a child frame transfers ownership and clears competing highlights.
- Prefer frame-local highlights to fragile cross-origin rectangle translation.
- Cancel stale requests on frame navigation; no reading text from a replacement document using an old frame reference.
- Multiple tabs may request reads, but the native app retains one session. A late state event must not update an unrelated tab's request UI.
- If an inaccessible child frame is under the pointer, show a limitation rather than pretending to read it.

Chrome documents isolated content-script execution and frame injection behavior; implementation must follow actual host-permission and frame rules. [Content-script documentation](https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts)

---

[Previous](11-shortcuts-and-background.md) · [Next](13-native-bridge.md)
