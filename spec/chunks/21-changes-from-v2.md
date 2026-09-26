# 21. Corrections and deliberate changes from the pasted v2

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This topic is synchronized with the full specification; original numbering is preserved.

| Pasted issue or ambiguity | Decision in this spec |
|---|---|
| Missing v1 requirements | Self-contained behavior and contracts |
| Windows-only WPF/.NET stack | Tauri 2 + React/TypeScript + Rust from the shared conversation; native adapters per OS |
| Model quality/speed asserted without measurements | Benchmark and listening gates |
| “One voice per Piper file” assumed universally | Capability metadata supports actual single/multi-speaker artifacts |
| Active/update/download states conflated | Separate installation, activation, update dimensions |
| Automatic preview after download | Explicit Preview action |
| Native messaging 1 MB direction reversed | Correct Chrome limits plus stricter app limits |
| Character count treated as serialized size | Separate UTF-16 text and encoded JSON byte limits |
| Duration estimate multiplied by speed | Divide by speed |
| Release Alt exits before Listen can be clicked | Latched picker with explicit cancellation |
| Alt-only picker | Configurable Ctrl/Alt/command activation |
| Copy text behavior unclear | Explicit Copy text with preview and feedback |
| Shadow root called page-proof | Style isolation with documented limits |
| Detached clone innerText assumed rendered | Extract from live rendered visibility |
| Public page messages trusted for frame text | Extension runtime coordination with sender identity |
| Broad all-sites permission assumed necessary | Optional approved-site access plus on-demand mode |
| Force-release/restore physical modifiers | Wait for release, then bounded safe fallback |
| Clipboard always restored unconditionally | Preserve later user clipboard changes; skip unsafe capture |
| Full duration known at acceptance | Estimate/null until synthesis supplies actual timing |
| Zero WPF frame allocations guaranteed by API choice | Measure WebView rendering; strict native audio-callback rule |
| Exact reference screenshot implied | No screenshot supplied; follow documented tokens only |

---

[Previous](20-definition-of-done.md) · [Next](22-decisions-to-confirm.md)
