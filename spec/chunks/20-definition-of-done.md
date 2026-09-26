# 20. Definition of done

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This topic is synchronized with the full specification; original numbering is preserved.

- [ ] Attractive, reviewed UI exists for desktop, player, and extension states.
- [ ] Clipboard and manual input work offline once a usable voice is installed; a zero-download system fallback is provided only where verified.
- [ ] At least one downloadable quality voice passes recorded evaluation.
- [ ] CPU operation is a complete supported mode.
- [ ] At least one GPU combination is verified if GPU support is advertised.
- [ ] Resource profiles constrain concurrency, buffers, and idle work.
- [ ] Chrome selected-text reading and configurable picker work on documented supported pages.
- [ ] Long-press controls, scope changes, Copy text, and Listen work without trapping normal input.
- [ ] All paths use one session and one native player.
- [ ] Model capabilities drive real, supported settings.
- [ ] Failures are visible and recoverable.
- [ ] No LLM or cloud inference dependency exists.
- [ ] No source text is transmitted externally for processing.
- [ ] Windows, macOS, Linux X11, and declared Wayland environments have separate tests, benchmarks, capability notes, and install instructions; unverified combinations are not advertised as supported.

---

[Previous](19-verification.md) · [Next](21-changes-from-v2.md)
