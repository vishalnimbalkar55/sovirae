# SpeakIt — Specification in small chunks

**Development in progress, macOS first (authorized 2026-09-26).** The product's display name is **Sovirae** (see the [branding spec](Sovirae-Branding-Spec.md)); code identifiers still use `speakit`. No step gate has passed yet; see the progress table below.

The [shared conversation](https://chatgpt.com/share/6ab777c9-6118-83e8-8ed4-fe510c699ce2) establishes the architecture: **Tauri 2 + React/TypeScript + Rust**, with local ONNX TTS. Windows, macOS, and Linux desktop are target platforms. LLM and STT are outside the current product. Read the [architecture](chunks/05-architecture.md) and [Windows prompt feature comparison](Windows-Prompt-Feature-Review.md) for the selected behaviors and shortcuts.

The specification has **24 topic files** and **14 individual delivery steps**. The [complete specification](SpeakIt-Product-Spec.md) remains available for copying. Topic and step files are synchronized extracts; update both representations when requirements change.

## Progress at a glance (2026-09-26)

| Step | State | Evidence |
|---|---|---|
| 0 Platform and evidence | Partly done | macOS 15 on Apple M4 and English-only confirmed; licenses and benchmark corpus open |
| 1 Product UI | Mostly done | Redesigned as Sovirae with the installed design skills; all screens and both player sizes reviewed in light/dark |
| 2 Speech quality | Started | Kokoro fp32 runs at 0.18–0.20 real-time on CPU; listening and pronunciation checks open |
| 3 Desktop shell | Mostly built | Tauri shell, tray, shortcuts, settings; focus-on-click limitation found on macOS |
| 4 Basic local reading | Mostly built | End-to-end test passes; clipboard path awaits a hands-on check |
| 5 Index and scheduling | Mostly built | 11 text tests, 4 stretcher tests, measured 2.00× at 2.0× |
| 6 Neural models | Mostly built | Verified Kokoro download and install, isolated worker, 0.92 s warm first audio |
| 7 Player polish | Partly done | Waveform, accessible seek, expanded sentence view |
| 8 Desktop capture | Not started | — |
| 9 Native bridge | Mostly built | Chrome stand-in passed 22/22; host starts the app in 0.31 s |
| 10–11 Extension | Built, untested in Chrome | Selection reading, picker, popup, options; 11 unit tests |
| 12 Packaging | Started | `npm run app:build` bundles the app, workers, and extension |

A step's checkbox below is ticked only when its gate passes.

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
