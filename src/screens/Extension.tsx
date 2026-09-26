import { Section } from "../lib/ui";

const CHECKS: [string, string, string][] = [
  ["Chrome extension", "Not installed", "Reads selections and picked paragraphs from web pages."],
  ["Desktop connection", "Not set up", "Lets the extension hand text to this app on your computer."],
  ["Sovirae", "Ready", "The player that reads text from every source."],
];

export default function ExtensionScreen() {
  return (
    <div className="screen" aria-labelledby="ext-title">
      <header className="screen-head">
        <h1 id="ext-title">Extension</h1>
        <p>Send text from Chrome to the same floating player.</p>
      </header>
      <Section title="Connection">
        <div className="group">
          {CHECKS.map(([label, value, hint]) => (
            <div className="row" key={label}>
              <div className="row-label">
                <span>{label}</span>
                <span className="hint">{hint}</span>
              </div>
              <span className={`pill ${value === "Ready" ? "ok" : ""}`}>{value === "Ready" && <span className="led" />}{value}</span>
            </div>
          ))}
        </div>
      </Section>
      <div className="empty">
        <h3>The Chrome extension isn't available yet</h3>
        <p>Until it is, copy text in Chrome and use Speak clipboard.</p>
      </div>
    </div>
  );
}
