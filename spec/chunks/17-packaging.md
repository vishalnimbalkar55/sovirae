# 17. Packaging and installation

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This file is a topic-sized extract of the full specification. Original requirement numbering is preserved.

1. Produce a per-user installer using a consistent writable install path under local app data; do not mix a no-admin claim with a required Program Files write.
2. Include app, inference host, native host, required runtime components, and license notices.
3. Write the native-host manifest and per-user Chrome registration with exact executable paths.
4. Keep the extension identity stable across the chosen development/distribution workflow. Do not claim it changes on every ordinary reload; pin the development key/identity where needed.
5. Offer unpacked-extension instructions for private use; do not silently enable an extension in the user's browser.
6. Treat Edge support as a separately verified P2 target, including its registration and store identity.
7. Uninstall removes owned registration and executables; ask whether to retain downloaded models/settings.
8. Update atomically, stop owned workers cleanly, and preserve user configuration.
9. Before public release, complete installer signing, extension review requirements, and license obligations for the actual distributed artifacts.
10. Revalidate current store/distribution policies when publishing; this spec does not authorize publishing.

---

[Previous](16-accessibility.md) · [Next](18-implementation-plan.md)
