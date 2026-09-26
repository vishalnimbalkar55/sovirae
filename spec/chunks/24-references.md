# 24. References and evidence boundary

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This file is a topic-sized extract of the full specification. Original requirement numbering is preserved.

Primary sources consulted during specification preparation:

1. [Kokoro official model card](https://huggingface.co/hexgrad/Kokoro-82M) — model identity, weights license, reference inference.
2. [Maintained Piper repository](https://github.com/OHF-Voice/piper1-gpl) — engine and license information.
3. [ONNX Runtime execution providers](https://onnxruntime.ai/docs/execution-providers/) — available acceleration backends.
4. [ONNX Runtime thread management](https://onnxruntime.ai/docs/performance/tune-performance/threading.html) — thread pools and spinning controls.
5. [.NET support policy](https://dotnet.microsoft.com/en-us/platform/support/policy/dotnet-core) — supported runtime lifecycle.
6. [Chrome Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging) — transport, limits, registration, caller origin.
7. [Chrome activeTab](https://developer.chrome.com/docs/extensions/develop/concepts/activeTab) — temporary permission model.
8. [Chrome content scripts](https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts) — script isolation and frames.
9. [Chrome service-worker lifecycle](https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/lifecycle) — restartable extension background work.
10. [MDN innerText](https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement/innerText) — rendered versus detached text behavior.
11. [Frontend design skill source](https://github.com/anthropics/skills/tree/main/skills/frontend-design) — design process applied to this spec.

No product implementation, executable prototype, audio benchmark, or visual screenshot validation was performed during this specification task. The installed design skill and this Markdown document are the deliverables.

---

[Previous](23-development-handoff.md)
