# 9. Text pipeline and document index

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: development in progress (macOS first).** This topic is synchronized with the full specification; original numbering is preserved.

### 9.1 Deterministic processing

1. Accept plain text plus source metadata.
2. Validate a 200,000 UTF-16-code-unit input limit consistently in extension/React JavaScript and Rust (which must count UTF-16 code units explicitly, not bytes or Unicode scalar values).
3. If over limit, ask whether to read the first portion; never silently truncate. Cut at a valid Unicode and preferably sentence boundary.
4. Normalize line endings, control characters, and repeated layout whitespace while retaining paragraph boundaries.
5. Preserve the original source string and a normalized-to-original span mapping.
6. Segment using deterministic sentence rules, not an LLM.
7. Handle abbreviations, decimals, quotes, and Unicode punctuation through explicit fixtures.
8. Split sentences exceeding the model's actual token/phoneme limit at sensible phrase boundaries.
9. Phonemize/tokenize using the model's required pipeline; never feed an incompatible phoneme alphabet.
10. Produce audio asynchronously and record actual sample lengths.

No summarization, paraphrasing, content deletion based on meaning, or inferred language rewriting. Optional URL/code reading behavior must be an explicit deterministic setting.

### 9.2 Data contracts

| Record | Required fields |
|---|---|
| SpeakRequest | Request ID, plain text, source kind, optional source title/origin, optional language hint |
| SpeechDocument | Document ID, original text, normalized text, span mapping, ordered segments |
| Segment | ID, sentence ID, original/normalized ranges, model input, synthesis state, PCM key, sample count |
| SourceReference | Source kind, display name, optional tab/frame/window identity, optional origin |
| PlaybackSnapshot | Session ID, status, current segment, source position, speed, volume, actual/estimated duration |

Source metadata is descriptive only. Page text, URLs, and titles are never executable commands.

### 9.3 Timing and cache rules

- Store source-audio sample positions at normal synthesis pace; do not store only wall-clock progress.
- Speed changes preserve source position while changing future output timing.
- Progress follows consumed audio plus output-device latency correction, not a periodic timer alone.
- Total duration is estimated until every segment has known sample counts.
- Cache keys include model/artifact version, voice, language, phonemizer version, normalized text, synthesis parameters, and audio format.
- Playback speed and volume do not invalidate synthesis cache.
- Advanced synthesis pace or expressiveness changes invalidate affected audio and create a new configuration generation.
- Keep segment metadata after PCM eviction; regenerate evicted audio on seek.
- Memory cache is bounded LRU. Persistent disk caching is opt-in, with a default 512 MiB quota if enabled later.

---

[Previous](08-resource-budgets.md) · [Next](10-floating-player.md)
