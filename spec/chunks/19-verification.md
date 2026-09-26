# 19. Verification matrix

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: development in progress (macOS first).** This topic is synchronized with the full specification; original numbering is preserved.

| Area | Required scenarios | Pass condition |
|---|---|---|
| Text | Abbreviations, Unicode, emoji, long sentences, whitespace, cap boundary | Stable segmentation and valid span mappings |
| Playback | Pause/resume, stop during load, rapid replacement, rate changes, seek | No stale audio or incorrect session state |
| Resource use | Eco/Balanced, CPU/GPU, 30-minute read, 3× playback, pressure | Measured targets and bounded memory behavior |
| Models | Interrupted download, wrong hash, insufficient disk, failed update | Prior working model preserved; actionable error |
| Capture | Representative native/browser/editor/PDF apps on each OS | Selected text or honest fallback; no corruption |
| Picker | News page, docs, GitHub, email UI, SPA, lists, short heading | Predictable candidate and correct visible text |
| Frames | Same/cross-origin, nested, inaccessible, navigation | Single owner; no stale-document extraction |
| Keyboard | Ctrl+C/V/L, Alt+Tab, AltGr, IME, key repeat, held modifiers | Normal behavior outside armed picker |
| Bridge | Partial reads, oversized frame, invalid JSON, spoofed fields, reconnect | Safe rejection and recovery |
| Output device | Unplug, Bluetooth, sleep/wake, changed default | Preserved position or clear paused recovery |
| UI | DPI, narrow work area, dark/high contrast, keyboard-only | No clipping or inaccessible controls |
| Privacy | Network disabled, log inspection, history off, incognito disabled | Local reading and no unsolicited content retention |
| Packaging | Fresh install, upgrade, uninstall, missing bridge on each OS | Correct user paths, native-host registration, recovery guidance |
| Platform behavior | Hotkeys, background, focus-safe player on Windows, macOS, Linux X11 and named Wayland session | Working action or documented OS restriction with tested fallback |

Benchmark procedure: one cold launch plus at least 20 warm starts per chosen configuration; record p50/p95, process-tree memory, total-machine-normalized CPU, provider-specific GPU memory, RTF, and underruns. Use the same corpus and repeat under a reproducible foreground workload. Keep quality listening separate from throughput measurements.

---

[Previous](18-implementation-plan.md) · [Next](20-definition-of-done.md)
