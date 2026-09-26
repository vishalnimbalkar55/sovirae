# SpeakIt — Specification in small chunks

**Planning only. No application implementation has started.**

Start with the [decisions to confirm](chunks/22-decisions-to-confirm.md), then use the [small implementation steps](steps/README.md). Open only the linked requirements needed for the current step.

The specification is split into **24 topic files** and **14 individual delivery steps**. Every step includes a checklist, dependency, acceptance gate, and related requirements. The original [complete specification](SpeakIt-Product-Spec.md) is retained for copying the whole document.

These files preserve the full specification's requirements; they do not introduce new product scope. Topic and step files are synchronized extracts of the full document. When requirements change, update the full document and the corresponding extracts together. The full document resolves any accidental disagreement.

## Small delivery steps

- [ ] [Step 0: Resolve platform and establish evidence](steps/00-resolve-platform-and-establish-evidence.md)
- [ ] [Step 1: Design the product UI](steps/01-design-the-product-ui.md)
- [ ] [Step 2: Prove speech quality and resource feasibility](steps/02-prove-speech-quality-and-resource-feasibility.md)
- [ ] [Step 3: Create desktop shell](steps/03-create-desktop-shell.md)
- [ ] [Step 4: Deliver basic local reading](steps/04-deliver-basic-local-reading.md)
- [ ] [Step 5: Build document index and audio scheduling](steps/05-build-document-index-and-audio-scheduling.md)
- [ ] [Step 6: Add neural models and resource profiles](steps/06-add-neural-models-and-resource-profiles.md)
- [ ] [Step 7: Polish the player](steps/07-polish-the-player.md)
- [ ] [Step 8: Add best-effort desktop capture](steps/08-add-best-effort-desktop-capture.md)
- [ ] [Step 9: Build the native bridge](steps/09-build-the-native-bridge.md)
- [ ] [Step 10: Deliver extension selection reading](steps/10-deliver-extension-selection-reading.md)
- [ ] [Step 11: Deliver long-press picker and copy controls](steps/11-deliver-long-press-picker-and-copy-controls.md)
- [ ] [Step 12: Verify and package](steps/12-verify-and-package.md)
- [ ] [Step 13: Optional subsequent improvements](steps/13-optional-subsequent-improvements.md)

## Requirements by topic

- [1. Purpose and source of truth](chunks/01-overview-and-assumptions.md)
- [2. Scope and success criteria](chunks/02-scope-and-success.md)
- [3. UI design direction and skill usage](chunks/03-ui-design.md)
- [4. Screen-by-screen behavior](chunks/04-screens-and-settings.md)
- [5. Architecture and platform boundaries](chunks/05-architecture.md)
- [6. Entry paths and exact user journeys](chunks/06-reading-entry-paths.md)
- [7. Local TTS strategy](chunks/07-local-tts-and-models.md)
- [8. Resource controls and measurable performance](chunks/08-resource-budgets.md)
- [9. Text pipeline and document index](chunks/09-text-pipeline.md)
- [10. Playback behavior and floating player](chunks/10-floating-player.md)
- [11. Global shortcuts and background behavior](chunks/11-shortcuts-and-background.md)
- [12. Chrome extension specification](chunks/12-chrome-extension.md)
- [13. Native messaging bridge and protocol](chunks/13-native-bridge.md)
- [14. Local storage, privacy, and diagnostics](chunks/14-privacy-and-storage.md)
- [15. Error and empty-state copy](chunks/15-errors-and-empty-states.md)
- [16. Accessibility requirements](chunks/16-accessibility.md)
- [17. Packaging and installation](chunks/17-packaging.md)
- [18. Step-by-step delivery plan](chunks/18-implementation-plan.md)
- [19. Verification matrix](chunks/19-verification.md)
- [20. Definition of done](chunks/20-definition-of-done.md)
- [21. Corrections and deliberate changes from the pasted v2](chunks/21-changes-from-v2.md)
- [22. Decisions to confirm before implementation](chunks/22-decisions-to-confirm.md)
- [23. Copyable future development handoff](chunks/23-development-handoff.md)
- [24. References and evidence boundary](chunks/24-references.md)

## How to use one chunk

1. Open a step file.
2. Read its linked requirements.
3. Complete only that step after development is authorized.
4. Verify its acceptance gate before continuing.
5. Keep checklist completion backed by actual evidence.
