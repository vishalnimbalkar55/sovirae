# 13. Native messaging bridge and protocol

[Spec index](../README.md) · [Full specification](../SpeakIt-Product-Spec.md)

**Status: planning only.** This file is a topic-sized extract of the full specification. Original requirement numbering is preserved.

### 13.1 Transport

Use `chrome.runtime.connectNative("com.speakit.bridge")`. Chrome communicates with the host using length-prefixed UTF-8 JSON; on Windows, the native-order length is little-endian. Chrome's documented limits are 1 MB host-to-browser and 64 MiB browser-to-host, correcting the pasted reversed limit. Content scripts communicate through the extension worker. [Native messaging documentation](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)

SpeakIt imposes stricter independent limits: 2 MiB serialized UTF-8 per incoming `speak` message, 64 KiB per outgoing state/control message, and the separate 200,000-code-unit text cap. Check actual encoded JSON size, including escaping and metadata, before allocation and send.

No loopback HTTP/WebSocket server is needed for this release. Native host stdout contains protocol bytes only; diagnostics go to redacted stderr/log files.

### 13.2 Authorization and lifetime

- Host manifest contains the expected extension origin allowlist.
- Validate Chrome-provided caller origin rather than trusting an `extId` supplied in JSON.
- Desktop pipe is scoped to the current user, with restricted ACLs and a validated installed-host connection handshake.
- Native messaging and pipe permissions reduce exposure; they do not defend against all malicious software already running as the same user.
- First use asks the user to allow the extension in a focusable desktop prompt. Do not accept `speak` while approval is pending.
- Deny and Revoke close authorized connections and reject future requests until explicitly allowed again.
- Host may start the installed app from a fixed validated installation path; never execute a path or command from page input.
- Startup wait is bounded to five seconds, then returns a recoverable error.
- Treat extension workers and connections as restartable. Reconnect on demand and query a snapshot instead of retaining authoritative state in the worker. [Service-worker lifecycle](https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/lifecycle)
- Use a bounded reconnect backoff; never create an infinite rapid relaunch loop.

### 13.3 Message schema

Every message includes protocol version `v` and type `t`. Requests include an opaque ID. Validate message types, field types, finite numeric values, ranges, and allowed enum members.

Example request:

```json
{
  "v": 1,
  "t": "speak",
  "id": "req-123",
  "text": "The text explicitly selected by the user.",
  "source": {
    "kind": "chrome",
    "origin": "https://example.com",
    "title": "Example article"
  },
  "languageHint": "en-US"
}
```

Example acceptance:

```json
{
  "v": 1,
  "t": "speak.accepted",
  "id": "req-123",
  "sessionId": "session-456",
  "status": "preparing",
  "estimatedDurationMs": null,
  "durationIsFinal": false
}
```

| Type | Direction | Purpose |
|---|---|---|
| hello / hello.ack | Both | Negotiate protocol, report app readiness and pairing |
| speak | Extension → app | Request reading |
| speak.accepted / speak.rejected | App → extension | Correlated result |
| control | Extension → app | Play, pause, stop, skip, set rate for a specified session |
| state.get / state | Both | Recover or update session snapshot |
| ping / pong | Both | Bounded diagnostic liveness check |
| error | App → extension | Structured error code and safe message |

Control fields: `sessionId`, `action`, optional `deltaMs`, optional `rate`. Reject controls for stale sessions. State includes session ID, request ID, status, source position, estimated/final duration, rate, and current sentence ID. Emit transitions immediately and steady playback updates at most 4 Hz.

Duplicate request IDs within a bounded reconnect window return the existing result instead of restarting audio. Cache at most 100 request outcomes for five minutes; reject reuse of the same ID with a different payload. A crash that loses deduplication state requires explicit retry rather than blind repeated sends.

Error codes include `NOT_AUTHORIZED`, `NO_TEXT`, `TEXT_TOO_LONG`, `PAYLOAD_TOO_LARGE`, `NO_VOICE`, `MODEL_LOADING`, `APP_UNAVAILABLE`, `UNSUPPORTED_PROTOCOL`, `STALE_SESSION`, `INVALID_REQUEST`, and `INTERNAL`.

---

[Previous](12-chrome-extension.md) · [Next](14-privacy-and-storage.md)
