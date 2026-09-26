# 1. Purpose and source of truth

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This file is a topic-sized extract of the full specification. Original requirement numbering is preserved.

Build a beautiful, local text-to-speech application with a floating player and Chrome extension. It should read selected or copied text in a crisp, pleasant voice while leaving enough CPU, GPU, and memory for the user's other work.

This is a self-contained specification. It replaces references to the unavailable `SpeakIt-ClaudeCode-Prompt.md` in the pasted v2 document. Requirements are defined here rather than inherited from that missing file.

Inputs reviewed:

- The user's request for attractive UI, efficient local CPU/GPU speech, a Chrome extension, granular steps, and no LLM for now.
- The complete pasted document, “SpeakIt — Spec v2: Model Config UI, Chrome Extension, Floating Player.”
- The linked [ChatGPT conversation](https://chatgpt.com/c/6ab76a7f-19f4-83ee-a304-248787073002) could not be fetched. This spec does not claim to include unseen conversation content or the reference screenshot mentioned in the paste.
- Primary technical sources listed in [§24](24-references.md) were checked while preparing the spec. Proposed budgets and UX decisions are requirements, not measured results.

### 1.1 Interpretation and assumptions

| Topic | Working decision |
|---|---|
| Product name | SpeakIt |
| Initial desktop platform | Windows 11 x64, following the pasted WPF design |
| Current development workspace | macOS; this does not imply that WPF runs on macOS |
| Cross-platform support | Future work; requires a separate UI/platform decision before coding if desired |
| “No LLM” | No chat model, summarization, rewriting, translation, agent, or cloud language-model dependency; dedicated local neural TTS remains in scope |
| “Long press control” | Support long-press Ctrl explicitly; also allow Alt and shortcut-only activation |
| Initial language validation | English US/UK; other catalog languages only marked supported after testing |
| Default hardware policy | Balanced CPU; optional verified GPU acceleration |
| New read during playback | Replace current reading; no queue in this release |
| Distribution | Private Windows installer plus unpacked extension first; public distribution later |

The platform and target hardware are assumptions to revisit before implementation, not approvals inferred from silence.

### 1.2 Priority definitions

- **P0:** Required for the useful desktop core and local TTS.
- **P1:** Required for the full initial product, including the Chrome picker and polished player.
- **P2:** Subsequent enhancement; not a dependency of initial release.
- A checked task in this document would mean verified completion. All tasks below are intentionally unchecked.

---

[Next](02-scope-and-success.md)
