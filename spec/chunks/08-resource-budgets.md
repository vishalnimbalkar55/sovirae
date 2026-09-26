# 8. Resource controls and measurable performance

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: development in progress (macOS first).** This topic is synchronized with the full specification; original numbering is preserved.

All numbers below are proposed engineering targets. No benchmark has been run for this planning task. Final supported hardware requires measurements and a published compatibility table.

### 8.1 Reference hardware classes

| Class | Proposed minimum test setup | Mode |
|---|---|---|
| Modest laptop | Four physical CPU cores, 8 GB RAM, SSD, no discrete GPU | Eco and Balanced CPU |
| Typical desktop | Six or more CPU cores, 16 GB RAM | Balanced CPU |
| GPU desktop | Compatible NVIDIA GPU with at least 4 GB VRAM, 16 GB RAM | Explicit supported GPU path |
| Integrated GPU | Supported Intel/AMD Windows or Linux GPU, or Apple Silicon | Optional provider validation by exact model/export |

Record exact CPU/GPU model, OS build, driver, power mode, RAM, runtime, and model hash. Do not claim universal support from these broad classes alone.

### 8.2 Resource profiles

Let `P` be detected physical CPU cores, conservatively falling back to logical cores if detection fails.

| Control | Eco | Balanced — default | Performance |
|---|---|---|---|
| Inference threads | `max(1, min(2, P-1))` | `max(1, min(4, floor(P/2)))` | `max(1, P-1)` |
| Concurrent synthesis jobs | 1 | 1 | 1 initially |
| Desired lookahead at current speed | 8 seconds | 15 seconds | 30 seconds |
| Maximum buffered PCM | 32 MiB | 64 MiB | 128 MiB |
| Worker priority | Below normal | Below normal | Normal |
| Idle model unload | 2 minutes | 5 minutes | 10 minutes |
| Optional waveform motion | Minimal | 30 fps playing | 30 fps playing |

Thread counts are controls, not hard CPU-percentage guarantees. Cap library-owned thread pools as well as application workers. ONNX Runtime exposes thread configuration and spinning controls; explicitly disable idle spinning where supported and verify the chosen binding's behavior. [Threading documentation](https://onnxruntime.ai/docs/performance/tune-performance/threading.html)

### 8.3 Acceptance targets

| Metric | Target on declared reference hardware |
|---|---|
| Warm first audible output | p95 ≤ 1.5 s for a 20–40-word English opening sentence |
| Cold first audible output | p95 ≤ 5 s with installed model, excluding download |
| CPU real-time factor | ≤ 0.7 at 1× on the supported Balanced CPU reference |
| GPU real-time factor | Target ≤ 0.35 on the validated GPU reference |
| Stop/pause response | ≤ 150 ms perceived audio response |
| UI interaction latency | p95 ≤ 100 ms during synthesis |
| Idle CPU, no work | Average < 1% of total machine capacity over 60 s |
| Active CPU | Target average ≤ 50% total capacity in Balanced; retain responsive foreground apps |
| Desktop + inference private memory | Target ≤ 1.5 GiB steady state for recommended Kokoro CPU configuration |
| GPU memory | Target ≤ 2 GiB dedicated allocation for recommended GPU configuration |
| Hidden player | No continuous waveform redraw |
| Long reading | No buffer underruns in 30 min at sustainable speed |

Real-time factor = synthesis wall time / produced audio duration. At playback speed `r`, sustained reading requires approximately `RTF < 1/r`; a model passing at 1× may fail at 3×. Show “This voice cannot keep up at this speed” with options to reduce speed, prebuffer longer, or change voice. Do not silently exceed the resource profile.

### 8.4 Scheduler, pressure, and cancellation

- Generate the opening sentence first; never synthesize a whole long article before playback.
- Use one bounded work queue and stop lookahead when either duration or byte budget is reached.
- Resume synthesis below a low-water mark of half the desired lookahead.
- Prioritize a user seek over speculative future segments.
- Discard stale results using session ID and configuration generation.
- On Stop, silence output immediately and cancel scheduled work. If native inference ignores cancellation, discard its result and terminate/restart the worker after a bounded two-second grace period.
- If memory pressure rises, release unused PCM and reduce lookahead before considering a fallback model.
- Keep only one loaded model in normal operation. Switching may unload the old worker first on low-memory machines, accepting a loading delay.
- Process isolation provides crash recovery; soft memory targets are not guaranteed allocation ceilings. Apply provider limits where available and report when unavailable.
- A GPU can briefly reach high utilization during a kernel. Bound duty cycle and memory; do not promise a universal GPU-percent cap.

---

[Previous](07-local-tts-and-models.md) · [Next](09-text-pipeline.md)
