# 17. Packaging and installation

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: development in progress (macOS first).** This topic is synchronized with the full specification; original numbering is preserved.

1. Produce native packages for every validated OS/architecture: Windows installer, signed/notarized macOS app where distributed, and a chosen Linux package format. Test installation on clean machines.
2. Bundle the Tauri shell, Rust/native TTS worker, native messaging host, required ONNX/audio libraries, and notices. Do not bundle Python, Torch, Node.js as a runtime, an LLM, or unverified GPU libraries.
3. Download model files separately after explicit user action unless a small voice is legally and technically verified for bundling. Do not promise first-run audio before a usable engine/voice is present.
4. Install the Chrome native-host manifest and origin allowlist in each OS-specific per-user registration location; use exact installed paths and extension ID. On Windows use the documented registry key; on macOS/Linux use Chrome's documented host directory.
5. Keep the extension identity stable across the chosen development/distribution workflow. Do not silently enable it in the user's browser.
6. Start-at-login is a user choice implemented by a tested Tauri/native mechanism on each OS. Background operation stays in the signed-in desktop session, never as a privileged system service.
7. Store settings, models, cache, and logs in OS user-data directories, respecting Linux XDG paths. The app's code-signing, executable permissions, shared-library loader paths, and worker supervision must be tested per package.
8. Uninstall removes owned registration and executables; ask whether to retain downloaded models/settings. An update must preserve config and not silently replace an in-use voice.
9. Private builds are updated manually by installing a newer package. Any automatic updater, such as the Tauri updater with signed update manifests, is a separate decision before public release.
10. Before public release, complete signatures/notarization where applicable, extension review, model/runtime license review, and a compatibility matrix for Windows, macOS, Linux X11, and named Wayland environments.
11. Revalidate store/distribution policies when publishing; this spec does not authorize publishing. [Tauri distribution](https://v2.tauri.app/start/), [Chrome Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)

---

[Previous](16-accessibility.md) · [Next](18-implementation-plan.md)
