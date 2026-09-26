# 2. Scope and success criteria

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This topic is synchronized with the full specification; original numbering is preserved.

### 2.1 Required outcomes

| ID | Priority | Requirement | Evidence of completion |
|---|---|---|---|
| UI-01 | P0 | Cohesive desktop UI with light/dark themes | Reviewed screenshots at supported sizes and DPI |
| TTS-01 | P0 | Speak manually entered or copied text locally | Works after disconnecting the network |
| TTS-02 | P0 | At least one high-quality downloadable local voice | Listening evaluation and reproducible benchmark |
| PERF-01 | P0 | Bounded resource use | CPU, RAM, VRAM, responsiveness, and buffer report |
| PLAY-01 | P0 | One playback session across all entry paths | Replacement/cancellation integration tests |
| EXT-01 | P1 | Chrome selection and block picking | Site compatibility matrix |
| EXT-02 | P1 | Configurable long-press Ctrl/Alt and scope control | Interaction tests, including ordinary shortcuts |
| BRIDGE-01 | P1 | Local native messaging transport | Reconnect, authorization, and malformed-message tests |
| CAP-01 | P1 | Best-effort auto-capture from desktop applications | Documented supported/unsupported cases |
| PRIV-01 | P0 | No text leaves the machine for speech | Network inspection and offline run |
| ACCESS-01 | P1 | Keyboard and screen-reader access | Manual accessibility review |

### 2.2 Explicitly out of scope

- LLM chat, summaries, semantic rewriting, translation, and AI agents.
- Cloud speech APIs, accounts, billing, telemetry, or analytics.
- Voice cloning, training, celebrity voice imitation, and microphone input.
- OCR, image reading, scanned PDF recognition, and DRM bypass.
- A second audio engine or player inside the extension.
- Mobile clients and multi-device synchronization. Windows, macOS, and Linux desktop are in scope, with platform-specific capability gates.
- Playback queues, batch audiobook production, and audio-file export in the first release.
- A website or marketing landing page; the UI design skill is used for product design guidance.

---

[Previous](01-overview-and-assumptions.md) · [Next](03-ui-design.md)
