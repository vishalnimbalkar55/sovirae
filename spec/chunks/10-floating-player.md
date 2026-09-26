# 10. Playback behavior and floating player

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: development in progress (macOS first).** This topic is synchronized with the full specification; original numbering is preserved.

### 10.1 Session states

`Idle → Preparing → Buffering → Playing ↔ Paused → Completed`

Any active state may transition to `Stopping → Idle` or `Error`. Device changes introduce a visible `Recovering` state. Distinguish Loading voice, Preparing text, and Buffering audio in user-facing labels.

A new validated reading replaces the old session, cancels old work, and receives a new ID. Empty or rejected input must not stop a working reading. Preview is a temporary use of the same player/output, never simultaneous playback.

### 10.2 Compact player

Target approximately 520 × 112, adapting to display scaling and available width.

```text
┌───────────────────────────────────────────────────────────────┐
│ example.com                  Heart       CPU        Expand  × │
│ [Back 10] [Play/Pause] [Forward 10]    [1.0×]       [Volume]    │
│ ▂▃▅▃▂▅▆▃──── waveform / seek ───────────────  02:14 / ~11:38 │
└───────────────────────────────────────────────────────────────┘
```

Requirements:

- One native player for every entry path; no competing in-page playback widget.
- Appears without stealing source-app focus.
- Topmost behavior toggle; no taskbar clutter in compact mode.
- Dedicated drag handle; remember position per monitor and clamp after monitor removal.
- Close stops and hides. Collapse preserves reading. Pause keeps position.
- Speed control supports direct numeric/menu selection; a right-click-only action is insufficient.
- Source chip focuses the original tab/window only on an explicit click and only if still identifiable.
- If the source disappears or navigates, continue speaking the accepted snapshot and mark the source unavailable.
- No remote favicon fetch; prefer a local generic icon or already available browser-provided data.

### 10.3 Expanded player

Target approximately 620 × 500 with a resizable reading pane.

- Display the actual normalized reading text with the current sentence highlighted.
- Click or keyboard-activate a sentence to seek to its beginning.
- Virtualize long text where necessary.
- Auto-scroll only while Follow reading is enabled; manual scrolling temporarily disables it and shows “Return to current sentence.”
- Provide keyboard focus intentionally when opened by the user. Non-activation must not make the expanded player inaccessible.
- Sentence highlighting is required. Exact word highlighting is deferred unless the engine exposes reliable alignment; do not fabricate timestamps.

### 10.4 Seeking and duration

- Skip ±10 seconds in source-audio time, clamp to valid range, and indicate sentence snapping if enabled.
- Known synthesized intervals allow precise seeks.
- Unsynthesized intervals show approximate progress and buffer after selection; prioritize the corresponding sentence.
- Distinguish elapsed source time from estimated remaining listening time at the current rate.
- Reading estimate: `words / (180 × rate)` minutes, explicitly approximate. For languages without reliable word boundaries, use a tested locale-specific estimate or omit it.

### 10.5 Waveform

- Compute a downsampled static amplitude envelope off the audio thread as segments complete.
- Unknown audio renders as a dim placeholder; never invent an exact waveform for unsynthesized text.
- Use up to three subtle layered curves, with the played region clearly distinguishable.
- Tap output amplitude after time stretching into a preallocated ring buffer.
- Audio callback must make no heap allocations, acquire no contended locks, and never touch React, WebView, or the Tauri event system.
- React renders waveform updates at most 30 fps while visible; use Canvas or a bounded SVG path and measure frame time/allocations on each webview. Keep amplitude transfer bounded and off the audio callback.
- No redraw timer while hidden. Paused view settles to a static state.
- Expose an accessible seek slider with text time values independent of the visual waveform.

### 10.6 Audio recovery

On device removal, pause, rebuild output on a valid device, and resume from the last reliable source position. Show recovery status; if recovery fails, remain paused with “Choose an audio device.” Test headphone unplugging, Bluetooth changes, sleep/wake, and sample-rate changes. Never loop indefinitely or restart the article from the beginning.

---

[Previous](09-text-pipeline.md) · [Next](11-shortcuts-and-background.md)
