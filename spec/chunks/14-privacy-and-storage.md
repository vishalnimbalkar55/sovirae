# 14. Local storage, privacy, and diagnostics

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: development in progress (macOS first).** This topic is synchronized with the full specification; original numbering is preserved.

- No user text, titles, URLs, audio, or voice previews are sent to a server for processing.
- Network operations are limited to explicit model downloads, catalog refresh, and update checks.
- No analytics or crash uploads by default.
- Store settings and installed-model metadata under the current user's local app-data directory.
- History is off by default. When enabled, store at most the last 20 items with clear Delete item and Clear all controls; explain that replay requires retaining text.
- Retain origin rather than full URL by default; full URLs can contain private tokens.
- Logs contain event IDs, timings, model IDs, resource readings, and sanitized error codes, not reading content.
- Rotate logs with a proposed total quota of 20 MiB.
- Persistent content/cache files, if enabled, are user-private under the OS application-data directory. Evaluate per-platform at-rest protection before claiming encryption; ordinary file permissions do not protect against other processes running as that user.
- Clear cache preserves installed models unless the user explicitly selects model deletion.
- Diagnostic export previews the included metadata and excludes reading text by default.
- Extension settings stay local. No browsing-history collection or passive page-text harvesting.

---

[Previous](13-native-bridge.md) · [Next](15-errors-and-empty-states.md)
