import { useEffect, useState } from "react";
import { api, on } from "../lib/api";
import { Alert, Check } from "../lib/icons";
import type { BridgeStatus } from "../lib/types";
import { Row, Section, Switch } from "../lib/ui";
import type { AppModel } from "../App";

function ago(secs: number | null): string {
  if (secs === null) return "Not used yet";
  if (secs < 60) return "Used just now";
  if (secs < 3600) return `Last used ${Math.round(secs / 60)} min ago`;
  return `Last used ${Math.round(secs / 3600)} h ago`;
}

export default function ExtensionScreen({ app }: { app: AppModel }) {
  const [status, setStatus] = useState<BridgeStatus | null>(null);
  const [test, setTest] = useState<{ ok: boolean; message: string } | null>(null);
  const [testing, setTesting] = useState(false);
  const [copied, setCopied] = useState(false);
  const settings = app.settings!;

  useEffect(() => {
    api.bridgeStatus().then(setStatus);
    const off = on("bridge", setStatus);
    const timer = window.setInterval(() => api.bridgeStatus().then(setStatus), 15000);
    return () => {
      off();
      clearInterval(timer);
    };
  }, []);

  const runTest = async () => {
    setTesting(true);
    setTest(null);
    try {
      setTest(await api.bridgeTest());
    } catch (e) {
      setTest({ ok: false, message: String(e) });
    } finally {
      setTesting(false);
    }
  };

  const copyAddress = async () => {
    await navigator.clipboard.writeText("chrome://extensions");
    setCopied(true);
    setTimeout(() => setCopied(false), 1600);
  };

  const registered = status?.browsers.filter((b) => b.registered) ?? [];
  const official = status?.extensionId;

  return (
    <div className="screen" aria-labelledby="ext-title">
      <header className="screen-head">
        <h1 id="ext-title">Extension</h1>
        <p>Read selections and picked parts of web pages in Chrome, with the same voices and player.</p>
      </header>

      <Section title="Connection">
        <div className="group">
          <Row label="Chrome connection" hint="Lets the Sovirae extension send page text to this app." htmlFor="bridge-on">
            <Switch id="bridge-on" checked={settings.chromeBridge} onChange={(chromeBridge) => app.update({ chromeBridge })} />
          </Row>
          {settings.chromeBridge && status && (
            <>
              <Row label="Status" hint={status.connected > 0 ? "Chrome is connected now." : ago(status.lastConnectionSecs)}>
                {status.listening && status.hostInstalled ? (
                  <span className="pill ok"><span className="led" />Ready</span>
                ) : (
                  <span className="pill">Not ready</span>
                )}
              </Row>
              <Row
                label="Browsers"
                hint={
                  status.browsers.length === 0
                    ? "No Chromium-based browser was found."
                    : "Restart a browser after it is added here for the first time."
                }
              >
                <span className="browser-pills">
                  {status.browsers.map((b) => (
                    <span key={b.name} className={`pill ${b.registered ? "ok" : ""}`}>
                      {b.registered && <Check />} {b.name}
                    </span>
                  ))}
                </span>
              </Row>
              <Row label="Test connection" hint={test ? test.message : "Runs the same path Chrome uses and names any problem."}>
                {test && (test.ok ? <span className="pill ok"><Check /> Works</span> : <span className="pill warn"><Alert /> Failed</span>)}
                <button className="btn" onClick={runTest} disabled={testing}>{testing ? "Testing…" : "Test"}</button>
              </Row>
            </>
          )}
        </div>
        {settings.chromeBridge && status && !status.hostInstalled && (
          <p className="field-error">The Chrome connection component is missing from this build.</p>
        )}
      </Section>

      {settings.chromeBridge && (
        <Section title="Install the extension" description="Needed once per browser profile.">
          <ol className="steps">
            <li>
              <div>
                <b>Open Chrome's extensions page</b> and turn on <b>Developer mode</b> in the top-right corner.
              </div>
              <button className="btn small" onClick={copyAddress}>{copied ? "Copied" : "Copy chrome://extensions"}</button>
            </li>
            <li>
              <div>
                Click <b>Load unpacked</b> and choose the Sovirae extension folder.
                {status?.extensionFolder && <code className="path">{status.extensionFolder}</code>}
              </div>
              <button className="btn small" onClick={() => api.openExtensionFolder()} disabled={!status?.extensionFolder}>
                Show folder
              </button>
            </li>
            <li>
              <div>
                Select text on any page and click the Sovirae toolbar button, or right-click and choose <b>Read with Sovirae</b>.
                The first time, allow Chrome in the prompt that appears here.
              </div>
            </li>
          </ol>
          {registered.length === 0 && status && status.browsers.length > 0 && (
            <p className="footnote">Turn the connection off and on again if a browser shows as not added.</p>
          )}
        </Section>
      )}

      <Section title="Allowed extensions" description="Only these extensions can send text to Sovirae.">
        {status && status.paired.length > 0 ? (
          <ul className="model-list">
            {status.paired.map((id) => (
              <li key={id} className="model-row">
                <div className="model-info">
                  <div className="model-title">
                    <span className="model-name">{id === official ? "Sovirae for Chrome" : "Other extension"}</span>
                    {id !== official && <span className="pill warn">Not official</span>}
                  </div>
                  <p className="model-meta"><code className="path">{id}</code></p>
                </div>
                <div className="model-buttons">
                  <button className="btn ghost" onClick={() => api.bridgeRevoke(id)}>Revoke</button>
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <div className="empty">
            <h3>No extension allowed yet</h3>
            <p>You'll be asked the first time the extension connects.</p>
          </div>
        )}
      </Section>

      <Section title="Reading web pages">
        <div className="group">
          <Row
            label="Match the page's language"
            hint="Reads a French page with a French voice from the same model, when one exists."
            htmlFor="match-lang"
          >
            <Switch id="match-lang" checked={settings.matchPageLanguage} onChange={(matchPageLanguage) => app.update({ matchPageLanguage })} />
          </Row>
        </div>
      </Section>
    </div>
  );
}
