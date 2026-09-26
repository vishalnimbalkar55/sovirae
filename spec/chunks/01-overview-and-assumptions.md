# 1. Purpose and source of truth

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: development in progress (macOS first).** This topic is synchronized with the full specification; original numbering is preserved.

Build a beautiful, local text-to-speech application with a floating player and Chrome extension. It should read selected or copied text in a crisp, pleasant voice while leaving enough CPU, GPU, and memory for the user's other work.

This is a self-contained specification. It combines the shared Tauri conversation with selected behavior from the pasted Windows v1/v2 documents. Requirements are defined here rather than inherited from those documents.

Inputs reviewed:

- The user's request for attractive UI, efficient local CPU/GPU speech, a Chrome extension, granular steps, and no LLM for now.
- The complete pasted document, “SpeakIt — Spec v2: Model Config UI, Chrome Extension, Floating Player.”
- The original private [ChatGPT conversation](https://chatgpt.com/c/6ab76a7f-19f4-83ee-a304-248787073002) could not be fetched, but its [shared copy](https://chatgpt.com/share/6ab777c9-6118-83e8-8ed4-fe510c699ce2) was read. It explicitly proposes Tauri 2 + React/TypeScript + Rust and local native ONNX TTS. No referenced screenshot was available.
- Primary technical sources listed in [§24](24-references.md) were checked while preparing the spec. Proposed budgets and UX decisions are requirements, not measured results.

### 1.1 Interpretation and assumptions

| Topic | Working decision |
|---|---|
| Product name | SpeakIt |
| Desktop platforms | macOS 15 arm64 is built and verified first (confirmed 2026-09-26). Windows 11 x64 and named Linux targets keep separate adapters and are tested before being claimed |
| Desktop framework | Tauri 2 + React/TypeScript + Rust, following the shared conversation |
| Current development workspace | macOS; Windows and Linux require separate native test environments |
| “No LLM” | No chat model, summarization, rewriting, translation, agent, or cloud language-model dependency; dedicated local neural TTS remains in scope |
| “Long press control” | Support long-press Ctrl explicitly; also allow Alt and shortcut-only activation |
| Voices and languages | All voices offered by each model and the OS are listed (changed at the user's request on 2026-09-26; earlier English-only). Voices without a stated gender or nameable language appear under "Other". Languages whose phonemizer is a stand-in are labelled "approximate pronunciation"; quality evaluation remains English-first |
| Default hardware policy | Balanced CPU; optional verified GPU acceleration |
| New read during playback | Replace current reading; no queue in this release |
| Distribution | Private native packages for validated target OSes plus unpacked extension first; public distribution later |

Platform intent and the Tauri stack come from the user and shared conversation. Exact OS versions, architectures, hardware targets, and packaging remain to be verified before implementation.

### 1.2 Priority definitions

- **P0:** Required for the useful desktop core and local TTS.
- **P1:** Required for the full initial product, including the Chrome picker and polished player.
- **P2:** Subsequent enhancement; not a dependency of initial release.
- A checked task in this document would mean verified completion. All tasks below are intentionally unchecked.

---

[Next](02-scope-and-success.md)
