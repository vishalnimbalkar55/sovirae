# Step 4 — Deliver basic local reading

[Spec index](../README.md) · [All steps](README.md)

**Development authorized on 2026-09-26 (macOS first).** A checked item has test or review evidence; see Progress below.

**Dependency:** Complete Step 3 and its gate first.

- [ ] Implement manual input and clipboard path.
- [ ] Add a verified local voice path on each target OS: tested system voice if accessible, otherwise the Step 2 lightweight model with explicit install/download before offline use.
- [x] Add text validation and safe error messages.
- [x] Route all entry paths through one session controller.
- [x] Add play, pause, stop, and replace-session behavior.

**Gate:** after a usable voice is installed, copy a paragraph, trigger the platform-registered Speak clipboard action, hear it offline, pause/stop reliably, and preserve clipboard content on every declared target.

## Progress — 2026-09-26

- macOS voice path verified by tests: offline system-voice synthesis and cancellation. Windows and Linux voice paths are not started.
- End-to-end test (`cargo test -p speakit end_to_end -- --ignored`): first audio 0.7–1.2 s, pause acknowledged in 6 ms, replacement reading gets a new session, empty input does not interrupt it, Stop returns to Idle and hides the player.
- Clipboard path is implemented (bounded retry, never writes the clipboard) but not yet exercised by hand.

## Relevant requirements

- [6. Entry paths and exact user journeys](../chunks/06-reading-entry-paths.md)
- [9. Text pipeline and document index](../chunks/09-text-pipeline.md)
- [10. Playback behavior and floating player](../chunks/10-floating-player.md)
