"""End-to-end check of the Chrome bridge without Chrome.

Acts exactly like Chrome: starts the registered native host with an extension
origin and speaks Chrome's framing (u32 native-endian length + JSON).

Needs Sovirae running (or startable) and the Sovirae extension allowed in
Sovirae › Extension. It reads a short French sentence aloud, then stops.

    python3 scripts/check-bridge.py
"""
import json, os, struct, subprocess, sys, threading, time, queue

MANIFEST = os.path.expanduser("~/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.sovirae.bridge.json")
HOST = json.load(open(MANIFEST))["path"]
OFFICIAL = "chrome-extension://jhbbmlhbjhjdepmaebpniefhaoljfgoe/"

class Port:
    def __init__(self, origin):
        self.p = subprocess.Popen([HOST, origin], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.q = queue.Queue()
        threading.Thread(target=self._read, daemon=True).start()
    def _read(self):
        while True:
            h = self.p.stdout.read(4)
            if len(h) < 4: self.q.put(None); return
            n = struct.unpack("=I", h)[0]
            self.q.put(json.loads(self.p.stdout.read(n)))
    def send_raw(self, b):
        self.p.stdin.write(struct.pack("=I", len(b)) + b); self.p.stdin.flush()
    def send(self, m): self.send_raw(json.dumps(m).encode())
    def recv(self, want=None, timeout=8):
        end = time.time() + timeout
        while time.time() < end:
            try: m = self.q.get(timeout=max(0.01, end - time.time()))
            except queue.Empty: break
            if m is None: return None
            if want is None or m.get("t") in want: return m
        raise TimeoutError(f"no {want}")
    def close(self): self.p.kill()

results = []
def check(name, cond, detail=""):
    results.append(cond); print(("PASS " if cond else "FAIL ") + name + (f"  [{detail}]" if detail else ""))

# 1. Unknown origin is never paired and cannot speak.
p = Port("chrome-extension://not-a-real-id/")
p.send({"t": "hello", "v": 1})
ack = p.recv({"hello.ack"})
check("invalid origin is refused", ack["pairing"] == "declined", ack["pairing"])
p.send({"t": "speak", "v": 1, "id": "x1", "text": "hello"})
r = p.recv({"speak.rejected", "speak.accepted"})
check("invalid origin cannot speak", r["t"] == "speak.rejected", r.get("code"))
p.close()

# 2. The official extension (allowed for this test).
p = Port(OFFICIAL)
p.send({"t": "hello", "v": 1, "ext": "test"})
ack = p.recv({"hello.ack"})
check("hello.ack paired", ack["pairing"] == "paired" and ack["ready"], json.dumps(ack))

p.send({"t": "ping", "v": 1}); check("ping/pong", p.recv({"pong"})["t"] == "pong")
p.send_raw(b"{not json")
e = p.recv({"error"}); check("malformed JSON rejected", e["code"] == "INVALID_REQUEST", e["message"][:60])
p.send({"t": "hello", "v": 9}); e = p.recv({"error"}); check("unsupported version", e["code"] == "UNSUPPORTED_PROTOCOL")
p.send({"t": "control", "v": 1, "action": "rate", "rate": 50}); e = p.recv({"error"}); check("out-of-range rate", e["code"] == "INVALID_REQUEST")
p.send({"t": "speak", "v": 1, "id": "e1", "text": "   "}); r = p.recv({"speak.rejected"}); check("empty text", r["code"] == "NO_TEXT")
p.send({"t": "speak", "v": 1, "id": "big1", "text": "a" * 200001}); r = p.recv({"speak.rejected"}); check("over 200,000 units", r["code"] == "TEXT_TOO_LONG")
p.send_raw(b'{"t":"speak","v":1,"id":"huge","text":"' + b"a" * (3 << 20) + b'"}')
e = p.recv({"error"}); check("3 MB frame refused by host", e["code"] == "PAYLOAD_TOO_LARGE")
p.send({"t": "ping", "v": 1}); check("stream still in sync after oversize", p.recv({"pong"})["t"] == "pong")

t0 = time.time()
p.send({"t": "speak", "v": 1, "id": "req-1", "text": "Bonjour tout le monde. Ceci est un test du pont Chrome.",
        "source": {"origin": "https://example.fr", "title": "Exemple"}, "languageHint": "fr-FR"})
acc = p.recv({"speak.accepted", "speak.rejected"}, timeout=10)
check("speak accepted", acc["t"] == "speak.accepted", json.dumps(acc))
check("duration is an estimate at acceptance", acc.get("estimatedDurationMs") is None and acc.get("durationIsFinal") is False)
sid = acc.get("sessionId")
playing = None
end = time.time() + 8
while time.time() < end:
    s = p.recv({"state"}, timeout=8)
    if s["sessionId"] == sid and s["status"] == "playing": playing = s; break
check("state reaches playing", playing is not None, f"{time.time()-t0:.2f}s after speak")
if playing:
    check("state carries request id", playing.get("requestId") == "req-1")
    check("page language chose a French voice", bool(playing.get("voice")), playing.get("voice"))
time.sleep(1.2)
s = None
p.send({"t": "state.get", "v": 1}); s = p.recv({"state"})
check("position advances", s["positionMs"] > 300, f'{s["positionMs"]} ms')

p.send({"t": "speak", "v": 1, "id": "req-1", "text": "Bonjour tout le monde. Ceci est un test du pont Chrome.",
        "source": {"origin": "https://example.fr", "title": "Exemple"}, "languageHint": "fr-FR"})
dup = p.recv({"speak.accepted", "speak.rejected"})
check("duplicate id replays original result", dup.get("sessionId") == sid)
p.send({"t": "speak", "v": 1, "id": "req-1", "text": "different text"})
c = p.recv({"speak.accepted", "speak.rejected"})
check("reused id with different text refused", c["t"] == "speak.rejected" and c["code"] == "INVALID_REQUEST")

p.send({"t": "control", "v": 1, "sessionId": sid + 100, "action": "pause"})
e = p.recv({"error"}); check("stale session control refused", e["code"] == "STALE_SESSION")
p.send({"t": "control", "v": 1, "sessionId": sid, "action": "pause"})
st = p.recv({"state"}); 
while st["status"] != "paused": st = p.recv({"state"})
check("pause via bridge", st["status"] == "paused")
p.send({"t": "control", "v": 1, "sessionId": sid, "action": "stop"})
st = p.recv({"state"})
while st["status"] != "idle": st = p.recv({"state"})
check("stop via bridge", st["status"] == "idle")
p.close()
print(f"\n{sum(results)}/{len(results)} checks passed")
sys.exit(0 if all(results) else 1)
