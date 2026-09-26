# 3. UI design direction and skill usage

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: development in progress (macOS first).** This topic is synchronized with the full specification; original numbering is preserved.

### 3.1 Selected skill

The `frontend-design` skill from [Anthropic's skills repository](https://github.com/anthropics/skills/tree/main/skills/frontend-design) was installed and read for this specification. Local installation: `/Users/zee/.codex/skills/frontend-design/SKILL.md`.

It is a suitable design-direction skill, not a claim that one objectively “best” UI skill exists. Its principles are adapted to a Tauri desktop product: intentional typography, meaningful visual hierarchy, restrained animation, realistic content, and one memorable visual element. No UI code or prototype has been implemented.

### 3.2 Concept: a quiet reading desk

The text being read is the center of the experience. The waveform is the signature visual. Navigation, model management, and hardware settings should remain calm and easy to scan.

Keep the warm surfaces and violet accent requested in the pasted design. Avoid filling every screen with identical cards, decorative statistics, or gradients. Use an uninterrupted reading area, grouped settings rows, and cards only for independently actionable models or readings.

### 3.3 Design tokens

All sizes are device-independent pixels for desktop and CSS pixels for extension UI.

| Token | Light | Dark | Purpose |
|---|---|---|---|
| Page | `#F7F6F3` | `#17181C` | Main background |
| Surface | `#FFFFFF` | `#222329` | Panels and model cards |
| Sidebar | `#F2F1EE` | `#1D1E23` | Navigation |
| Text primary | `#1A1A1A` | `#F1F0F4` | Main text |
| Text secondary | `#6B6862` | `#B8B6C2` | Supporting text |
| Accent | `#7C5CFF` | `#A28BFF` | Selection, waveform, focus rings; not normal-size light-theme text (4.0:1) |
| Accent button | `#6242D6` | `#B5A2FF` | Primary button and accent-coloured text such as links; white text on light, dark text on dark |
| Border | `#E5E3DE` | `#3B3D46` | Decorative group separation only |
| Control border | `#8A867F` | `#737585` | Input, switch, and other control boundaries (≥ 3:1) |
| Success | `#2E7D5B` | `#7ACBA7` | Successful state |
| Error | `#B63229` | `#FF9C91` | Error text and icons |

Typography:

- UI: the platform system font — `-apple-system`/SF Pro on macOS, Segoe UI Variable then Segoe UI on Windows, and `system-ui` (typically Cantarell, Noto Sans, or Ubuntu) on Linux; extension uses `system-ui`. Verify metrics on each webview; do not bundle a proprietary system font.
- Reading text: Georgia, fallback serif, user-selectable sans-serif alternative.
- UI text 14; secondary text 12; section title 20; page title 28; reading text 18 by default.
- Reading text adjustable 14–28 with 1.55 line height; target 60–75 characters per line.
- Use sentence case. Do not rely on color, emoji, or a tiny icon to convey a state.

Geometry and motion:

- Four-pixel spacing grid; principal gaps 8/12/16/24/32.
- Panel radius 14; button radius 10; input radius 8; pills fully rounded.
- Main content padding 28; sidebar width approximately 220.
- One subtle shadow for floating surfaces; no permanent shadow on every settings row.
- Interaction transitions 120–180 ms. Waveform capped at 30 fps while playing.
- Reduced-motion mode stops breathing, gliding highlights, and decorative transitions.
- Paused player uses a static waveform and clear “Paused” label by default to save resources.

### 3.4 Layout and review criteria

Preferred settings window 1080 × 740; minimum 860 × 620. Smaller work areas scroll the content rather than clipping controls. Test 100%, 125%, 150%, and 200% DPI.

```text
┌───────────────────────────────────────────────────────────────────┐
│ SpeakIt                                            Window controls│
├──────────────────┬────────────────────────────────────────────────┤
│ Read             │ Read                                           │
│ Voices           │ Paste text and listen                          │
│ Shortcuts        │ ┌────────────────────────────────────────────┐ │
│ Extension        │ │ Editable text                              │ │
│ Settings         │ │                                            │ │
│                  │ └────────────────────────────────────────────┘ │
│                  │ 1,240 words    About 7 min     [Listen]         │
│                  │                                                │
│                  │ Voice: Heart    Device: CPU    [Change voice]  │
│ Local audio      │ Continue reading / recent items, if enabled    │
└──────────────────┴────────────────────────────────────────────────┘
```

Before implementation, produce static designs for Read, Voices, Settings, extension popup, picker confirmation, and both player sizes. Review real long labels, missing voices, empty state, download failure, and disconnected extension—not only the happy path.

Design critique already applied to this spec: removed the decorative streak/hero dashboard as a release dependency, retained the source palette, centered the actual reading task, and made the waveform the main expressive element. History and optional statistics remain P2.

---

[Previous](02-scope-and-success.md) · [Next](04-screens-and-settings.md)
