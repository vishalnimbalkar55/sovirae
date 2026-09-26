# Step 1 — Design the product UI

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 0 and its gate first.

- [x] Apply the installed frontend-design skill to the design brief in [§3](../chunks/03-ui-design.md).
- [x] Create tokens and component states.
- [x] Design Read, Voices, Shortcuts, Extension, and Settings.
- [ ] Design compact/expanded player and picker confirmation.
- [ ] Include light/dark, empty, loading, error, disabled, and long-content variants.
- [ ] Review keyboard navigation, contrast, scaling, and user-facing copy.
- [x] Revise any decorative dashboard elements that compete with reading.

**Gate:** coherent screen designs using real sample text, with no implementation required for review.

## Progress — 2026-09-26

- Screens are built as working React views instead of static mockups, reviewed with a browser preview (`npm run dev`, then open `index.html` or `player.html?state=playing|paused|notice|idle`).
- Open: the Codex frontend-design skill was not available in this environment; picker confirmation design, loading/error/long-content variants, and a keyboard review.
- Redesigned with the installed frontend-design, anti-ui-slop, and web-design-guidelines skills and the Sovirae branding brief: brand palette kept, system UI type, layered surfaces instead of heavy outlines, custom slider/switch/segmented controls, a reading sheet with a guided empty state, and a separately tuned dark theme. Reviewed in light and dark at 1080 × 740, 520 × 112, and 620 × 500.
- Provisional Sovirae mark (a lowercase s whose top stroke is a text caret) is used in the sidebar, player, app icon, and menu-bar icon. The brand spec's full concept review is still open.

## Relevant requirements

- [3. UI design direction and skill usage](../chunks/03-ui-design.md)
- [4. Screen-by-screen behavior](../chunks/04-screens-and-settings.md)
- [10. Playback behavior and floating player](../chunks/10-floating-player.md)
- [16. Accessibility requirements](../chunks/16-accessibility.md)
