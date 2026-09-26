# 24. References and evidence boundary

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This topic is synchronized with the full specification; original numbering is preserved.

Primary sources consulted during specification preparation:

1. [Kokoro official model card](https://huggingface.co/hexgrad/Kokoro-82M) — model identity, weights license, reference inference.
2. [Maintained Piper repository](https://github.com/OHF-Voice/piper1-gpl) — engine and license information.
3. [ONNX Runtime execution providers](https://onnxruntime.ai/docs/execution-providers/) — available acceleration backends.
4. [ONNX Runtime thread management](https://onnxruntime.ai/docs/performance/tune-performance/threading.html) — thread pools and spinning controls.
5. [Tauri 2](https://v2.tauri.app/start/) — cross-platform desktop shell and Rust command bridge.
6. [Chrome Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging) — transport, limits, registration, caller origin.
7. [Chrome activeTab](https://developer.chrome.com/docs/extensions/develop/concepts/activeTab) — temporary permission model.
8. [Chrome content scripts](https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts) — script isolation and frames.
9. [Chrome service-worker lifecycle](https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/lifecycle) — restartable extension background work.
10. [MDN innerText](https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement/innerText) — rendered versus detached text behavior.
11. [Frontend design skill source](https://github.com/anthropics/skills/tree/main/skills/frontend-design) — design process applied to this spec.
12. [Shared architecture conversation](https://chatgpt.com/share/6ab777c9-6118-83e8-8ed4-fe510c699ce2) — user-provided Tauri/Rust/React and native ONNX TTS direction; its LLM material is out of scope.
13. [Tauri Global Shortcut plugin](https://v2.tauri.app/plugin/global-shortcut/) — candidate registered hotkey API.
14. [Tauri Shell plugin](https://v2.tauri.app/plugin/shell/) — candidate supervised external worker launch.
15. [Tauri Autostart plugin](https://v2.tauri.app/plugin/autostart/) — candidate login startup control.
16. [XDG GlobalShortcuts portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html) — Linux compositor-dependent global shortcuts.
17. [CPAL](https://github.com/RustAudio/cpal) — cross-platform Rust audio output candidate and backend requirements.
18. [Tauri sidecar guide](https://v2.tauri.app/develop/sidecar/) — packaging worker binaries.

No product implementation, executable prototype, audio benchmark, or visual screenshot validation was performed during this specification task. The installed design skill and these Markdown specification files are the deliverables. The share conversation was read in the browser; its architecture is treated as a design input, not proof that every native API works on every OS.

---

[Previous](23-development-handoff.md)
