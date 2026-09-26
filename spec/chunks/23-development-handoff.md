# 23. Copyable future development handoff

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This file is a topic-sized extract of the full specification. Original requirement numbering is preserved.

The following text may be copied into a future development task. It is not an instruction to start implementation now.

```text
Implement SpeakIt using spec/SpeakIt-Product-Spec.md as the source of truth.
First resolve its platform, hardware, language, and distribution assumptions.
Keep all speech local. Do not add an LLM, summarization, rewriting, or cloud TTS.
Apply the installed frontend-design skill to the product UI and preserve the
reading-focused design, waveform player, and accessibility requirements.
Follow the numbered milestones sequentially. Start with design and a local
speech feasibility benchmark, then deliver clipboard reading before complex
capture and extension work. Use one playback session and one native player.
Implement resource budgets, cancellation, explicit CPU/GPU reporting, model
verification, and a permission-aware Chrome picker with Ctrl/Alt long press,
scope controls, text preview, Copy text, and Listen. Verify each milestone's
gate and report measured evidence rather than assumed performance. Do not
publish or expand platform scope without a separate request.
```

---

[Previous](22-decisions-to-confirm.md) · [Next](24-references.md)
