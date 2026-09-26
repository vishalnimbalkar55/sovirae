# 22. Decisions to confirm before implementation

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This topic is synchronized with the full specification; original numbering is preserved.

These do not prevent using this document as a planning artifact.

1. Exact OS releases, CPU architectures, Linux distributions, and Wayland compositors to claim as supported. Cross-platform desktop scope and Tauri stack are set.
2. Actual CPU/GPU and RAM available for the primary performance target.
3. Required languages beyond English, including whether Hindi is mandatory.
4. Private personal use versus a distributable product, affecting Piper integration and packaging choices.
5. Preferred default browser picker trigger: Ctrl, Alt, or shortcut-only. All remain configurable.
6. Whether public Chrome Web Store distribution belongs in the first release.

Until confirmed, use the assumptions in [§1.1](01-overview-and-assumptions.md) and do not silently expand scope.

---

[Previous](21-changes-from-v2.md) · [Next](23-development-handoff.md)
