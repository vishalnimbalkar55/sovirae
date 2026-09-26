# 16. Accessibility requirements

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This topic is synchronized with the full specification; original numbering is preserved.

- Every action is keyboard reachable with an accessible name and visible focus.
- Focus order follows layout; no focus traps in popup or confirmation.
- Provide a keyboard route to picker scope, confirmation, and cancellation.
- Target at least 4.5:1 contrast for normal text and 3:1 for large text/control boundaries where applicable; verify actual rendered combinations.
- Color and waveform motion never carry the only status information.
- Player controls have at least 32 × 32 desktop hit areas; use larger targets where space permits.
- Respect system text scaling, high contrast, and reduced motion.
- Avoid constant screen-reader announcements of playback ticks; announce state changes and requested position changes.
- Focus-stealing avoidance and accessibility must coexist: the compact player appears passively, while an explicit open/focus command makes it operable.

---

[Previous](15-errors-and-empty-states.md) · [Next](17-packaging.md)
