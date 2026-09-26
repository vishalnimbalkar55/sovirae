# Step 11 — Deliver long-press picker and copy controls

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 10 and its gate first.

- [x] Implement trigger state machine and editable-field exclusions.
- [ ] Add hover highlight and candidate discovery.
- [ ] Add wheel/keyboard scope controls.
- [ ] Add latched confirmation with text preview.
- [ ] Implement Listen, Copy text, Back, and Close.
- [ ] Add safe extraction, frame ownership, and per-site disabling.
- [ ] Verify ordinary Ctrl/Alt shortcuts are preserved.

**Gate:** documented results across at least ten representative sites/fixtures and all picker cancellation paths.

## Progress — 2026-09-26

- Built, not yet loaded in Chrome: long-press trigger (Option on macOS, Ctrl elsewhere, or shortcut-only; 300–1000 ms, default 450 ms; latches after release; cancelled by other keys, clicks, repeats, IME, AltGr, and editable fields; 6 Node tests), hover highlight with a scope label, ↑/↓ and Option+wheel resizing, a confirmation panel with preview, word count, estimate, Listen, Copy text, Back, and Close, link clicks blocked while picking, 15 s inactivity cancel, one picker per tab across frames, and live `innerText` extraction.
- Fixed 2026-09-26 after first real use: (1) the picker closed as soon as it opened, because the "one picker per tab" broadcast also reached the frame that started it. Each frame now has its own token and ignores its own broadcast. Checked in a browser with a stub that mirrors the service worker: hold → highlight → click → confirmation panel. (2) Long press did nothing until Sovirae was clicked on the page, because host access was optional per site. By the user's decision, access to every HTTP/HTTPS site is now in the manifest, the content script is declared for all pages, and install/reload injects it into open tabs; the per-site "Always ready" switches were removed. Verified in real Chrome 154 (extension loaded through the DevTools protocol in a throwaway profile): a long press showed the picker and highlight on a fresh tab and on a tab opened before the extension loaded, with no click first. (3) The banner logo had no size, and the scope label sat under the banner for blocks near the top of the page; it now drops below the block.
- Improvement over the spec's wording: plain scrolling keeps working while picking; resizing uses the arrow keys, Option+wheel, or the ↑/↓ buttons on the label.

## Relevant requirements

- [12. Chrome extension specification](../chunks/12-chrome-extension.md)
- [16. Accessibility requirements](../chunks/16-accessibility.md)
