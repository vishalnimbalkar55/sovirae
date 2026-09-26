# 15. Error and empty-state copy

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This topic is synchronized with the full specification; original numbering is preserved.

| Condition | Surface | Required response |
|---|---|---|
| Empty clipboard | Player notice | “Clipboard is empty. Copy some text and try again.” |
| Non-text clipboard | Player notice | “Clipboard does not contain text.” |
| Clipboard busy | Player notice | “Clipboard is busy. Try again.” |
| No selection | Player notice | “Select text first, or copy it and use Speak clipboard.” |
| Capture not permitted | Player notice | “This app cannot be read automatically. Copy the text manually.” |
| No usable voice | Voices/player | “Choose or download a voice to start reading.” |
| Model loading | Player | “Loading voice…” with cancellation |
| Download failed | Model card | Retry/resume action and a specific reason |
| Hash mismatch | Model card | “The download could not be verified. Download again.” |
| Unsupported GPU | Performance | “This voice cannot use the selected GPU. Use CPU.” |
| Resource pressure | Player/settings | Explain reduced buffering and offer lower-resource mode |
| Bridge absent | Extension popup | “Install or repair the SpeakIt desktop connection.” |
| App cannot launch | Popup/confirmation | “SpeakIt could not be opened. Open it and retry.” |
| Pairing pending | Popup/confirmation | “Allow this extension in SpeakIt.” |
| Restricted page | Popup | “This page cannot be picked. Copy text and use Speak clipboard.” |
| Over text limit | Confirmation | Offer first 200,000 units with explicit consent or Cancel |
| Audio device missing | Player | “Audio device disconnected. Choose an output device.” |
| Source navigated/closed | Source chip | Mark unavailable; continue accepted reading |
| Inference worker crash | Player | Keep position; Retry or Switch voice |

Errors must remain long enough to act on and be announced accessibly. Do not make a three-second toast the only place containing a recovery action.

---

[Previous](14-privacy-and-storage.md) · [Next](16-accessibility.md)
