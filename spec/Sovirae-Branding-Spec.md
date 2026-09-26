# Sovirae — Branding specification

Version: 0.2 • Status: **branding direction** • Prepared: 2026-09-26

This is the branding brief for Sovirae, a local speech application. It defines the name, positioning, visual identity, and writing style across desktop, floating player, and Chrome extension.

## 1. Name

| Item | Direction |
|---|---|
| Display name | **Sovirae** |
| Pronunciation | **so-VEER-ay** — three syllables |
| Spelling | Capital S, lowercase remainder; never “SoviRae” or all caps in normal UI |
| Meaning | Coined word; no established dictionary definition |
| Intended brand idea | **Ideas finding a voice** |
| Current product description | Local read-aloud app for text selected, copied, or entered by the user |
| Later scope | Name can also accommodate transcription or local AI if those features are actually built |

## 2. Brand position

**One-sentence promise:** Sovirae helps people hear the text they choose, with local voices and controls that stay out of the way.

**Who it serves:** People who read articles, documents, messages, or long passages while working, learning, or resting their eyes. It should feel useful to someone listening for five minutes or for an hour.

**What people should notice first:** Their text starts reading clearly; the player shows where they are and stays available without interrupting the source app.

**Personality:** Calm, articulate, capable, private, and lightly warm. The brand should feel closer to a well-made reading tool than a synthetic assistant or entertainment mascot.

**Proof through the product:** Explicit text selection, honest model/device status, a readable sentence view, familiar shortcuts, bounded resource use, and clear permission explanations. The name alone must not carry the trust claim.

## 3. Naming architecture

- Use **Sovirae** as the application and Chrome extension name.
- Describe the current feature as **Read aloud** or **Listen**. Do not rename a button to an abstract brand term.
- Use plain feature labels: **Read**, **Voices**, **Shortcuts**, **Extension**, **Settings**, **Floating player**.
- Keep individual voice/model names accurate to their providers. Do not imply Sovirae created Kokoro or Piper.
- Do not create names for STT or LLM modules now. Add them only when a feature has a defined user job and a separate approved spec.
- Use consistent Sovirae naming across app surfaces; keep technical identifiers aligned with the product architecture.

## 4. Verbal identity

### Voice and tone

Write in short, direct sentences. Name the action, then state the result or recovery step. The interface should sound like a helpful tool, not a person claiming to understand the user.

| Situation | Preferred copy | Avoid |
|---|---|---|
| Start action | “Listen” | “Let the magic begin” |
| Source selection | “Choose text to read aloud” | “Feed me some words” |
| Model loading | “Loading voice…” | “Waking up your AI companion…” |
| Offline status | “Voice ready offline” | “Your data is 100% safe forever” |
| Missing permission | “Allow access to read selected text from other apps” | “Something went wrong” |
| Unsupported capture | “Copy the text, then use Speak clipboard” | “Cannot process your request” |
| Idle background state | “Sovirae is ready in the menu bar” or platform equivalent | “Sovirae is always watching” |

Use **local** only when describing on-device synthesis. Do not promise that websites, operating systems, or extension stores are offline. Do not market the current product as an LLM assistant. “AI voice” may be used in descriptive copy only when followed by the concrete fact that speech runs locally.

### Taglines

- **Primary launch line:** “Hear what matters.” Short and broad, but use only alongside an accurate explanation that the user chooses the text.
- **Descriptive line:** “Read aloud, right where you work.” Suitable for a download page or extension listing once those exist.
- **Brand idea, internal:** “Ideas finding a voice.” A creative direction, not a claim that the current app generates ideas.

Never put a tagline on every surface. The player and extension popup need controls and status, not advertising.

### App-store description draft

> Sovirae reads the text you choose aloud using local voices. Start from copied text, a selection, or the Chrome extension. Keep listening in a small floating player while you work.

This copy must be revised if any named entry path is not available on a released platform.

## 5. Visual identity

### 5.1 Core concept: a reading mark becoming sound

Use a **single cursor-to-wave gesture** as the symbol: a short upright reading caret transitions into one smooth outward sound curve. The form should also resemble a quiet lowercase “s” at a glance. The mark represents user-chosen text becoming audio, while remaining broad enough for future listening/transcription features.

The primary wordmark is **Sovirae** in a carefully spaced, humanist sans style. The distinctiveness should come from the shape of the “S” and the measured letter spacing, not from a microphone, robot head, sparkle, or a generic waveform pasted above the word.

Deliver the symbol as a code-native vector asset when design work begins. Review its clarity at app-icon and toolbar sizes before finalizing.

### 5.2 Color system

Retain the approved product-spec palette so branding and UI remain coherent.

| Role | Light | Dark | Use |
|---|---|---|---|
| Quiet canvas | `#F7F6F3` | `#17181C` | Reading/settings background |
| Surface | `#FFFFFF` | `#222329` | Player and actionable panels |
| Ink | `#1A1A1A` | `#F1F0F4` | Wordmark and main text |
| Secondary ink | `#6B6862` | `#B8B6C2` | Supporting labels |
| Signature violet | `#7C5CFF` | `#A28BFF` | Waveform and selective highlights |
| Action violet | `#6242D6` | `#B5A2FF` | Primary actions and accessible small text |

The violet is a recognizable signal, not a full-screen wash. Use it where speaking, selection, or focus is active. Keep ordinary copy in ink colors. Check actual color contrast for every control state; the signature violet is not automatically safe as small text on the light canvas.

### 5.3 Type and layout

- Use the platform system UI font inside the app for reliable Windows/macOS/Linux metrics. The wordmark may use custom outlined lettering after license and legibility review.
- Use a readable long-form face for the expanded reading view, with a sans-serif option.
- Keep headings sentence case and avoid decorative all-caps labels.
- Give the reading area the most space. Treat controls as a stable frame around it.
- Use cards for independent models or saved readings; keep ordinary settings as grouped rows.
- At 320–360 px extension width, the name should not crowd the primary Listen action.

### 5.4 Motion and sound

The waveform is the sole expressive motion. It reflects audio that exists, settles when paused, and stops animating when hidden or reduced motion is enabled. A simple 120–180 ms transition may confirm opening or closing the player. No startup sound, voice greeting, or automatic preview audio.

The brand must remain recognizable in still screenshots, silent mode, high contrast, and a static tray icon.

## 6. Asset system and product placement

| Asset/surface | Requirement |
|---|---|
| Primary wordmark | Vector, horizontal, light/dark versions; readable at approximately 120 px width |
| Symbol | Square vector, one-color first, no tiny internal detail; test 16/20/24/32/128 px |
| App icon | Symbol on quiet surface with OS-appropriate safe area; avoid text at small sizes |
| Menu bar/tray | Monochrome template form plus active/paused state that also uses a shape or label |
| Floating player | Symbol only if it helps identify source; prioritize controls, time, and text |
| Chrome toolbar | Same symbol with enabled/disabled state; badge only for a meaningful playback state |
| Extension popup | Wordmark once in header, then concise status and actions |
| Installer/website later | Full wordmark and one plain descriptive line |

Do not turn each voice/model into a different sub-brand. The person should recognize one Sovirae app across desktop, floating player, and extension.

## 7. Design process and acceptance gates

This brief applies the installed [frontend-design skill](</Users/zee/.codex/skills/frontend-design/SKILL.md>) to Sovirae: begin with the product's reading task, choose one memorable element, use real copy, review empty/error states, and critique generic styling before implementation.

### Before drawing final artwork

- [ ] Gather 8–12 neighboring reading, accessibility, and audio icons; document visual collisions.
- [ ] Draw at least three black-and-white symbol concepts using the cursor-to-wave idea or an equally specific reading-to-sound idea.
- [ ] Test each at 16 px and beside a line of real app text.
- [ ] Pick one concept for visual clarity and recognizability, then apply the palette.

### Before adding it to a product build

- [ ] Verify legibility in light, dark, high-contrast, and reduced-motion states.
- [ ] Review app icon, menu bar/tray, compact player, and Chrome toolbar side by side.
- [ ] Test with actual text such as “Loading voice…”, “Clipboard is empty”, and a long hostname.
- [ ] Check spelling and pronunciation with several readers unfamiliar with the name.
- [ ] Review accessible names and contrast; the icon alone is never the only label of an action.
- [ ] Export SVG masters and OS-specific icon sizes from one approved source.

The branding work is complete when the real assets pass these checks. This document is the design brief for those assets.

## 8. Relationship to existing specifications

- [Product specification](SpeakIt-Product-Spec.md): defines product behavior and architecture; this document defines the Sovirae brand.
- [UI design direction](chunks/03-ui-design.md): supplies the product color, typography, reading layout, and motion requirements reused here.
- [Floating player](chunks/10-floating-player.md): functional behavior remains governed by the product spec.
- [Chrome extension](chunks/12-chrome-extension.md): functional permissions and picker behavior remain governed by the product spec.

Branding decisions should be applied consistently when product surfaces are designed.
