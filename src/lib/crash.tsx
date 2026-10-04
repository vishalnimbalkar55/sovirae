// A render or startup error must never leave a blank window. Errors are
// reported to the Rust log (Logs/com.sovirae.desktop/sovirae.log) and the
// window shows a short notice with a reload button instead.
import { Component, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./crash.css";

function describe(e: unknown): string {
  if (e instanceof Error) return `${e.name}: ${e.message}\n${e.stack ?? ""}`.trim();
  try {
    return typeof e === "string" ? e : JSON.stringify(e);
  } catch {
    return String(e);
  }
}

export function reportError(context: string, e: unknown) {
  const message = `${context}: ${describe(e)}`;
  console.error(message);
  if ("__TAURI_INTERNALS__" in window) {
    invoke("report_ui_error", { message }).catch(() => {});
  }
}

/** Sends uncaught errors and rejected promises to the app log. */
export function installErrorReporting(window_: string) {
  window.addEventListener("error", (ev) => reportError(`${window_} uncaught`, ev.error ?? ev.message));
  window.addEventListener("unhandledrejection", (ev) => reportError(`${window_} unhandled rejection`, ev.reason));
}

export class ErrorBoundary extends Component<{ name: string; children: ReactNode }, { error: string | null }> {
  state = { error: null as string | null };

  static getDerivedStateFromError(e: unknown) {
    return { error: describe(e) };
  }

  componentDidCatch(e: unknown) {
    reportError(`${this.props.name} render`, e);
  }

  render() {
    if (this.state.error === null) return this.props.children;
    return (
      <div className="crash" role="alert">
        <p className="crash-title">Something went wrong</p>
        <p className="crash-sub">The error was written to the log.</p>
        <button onClick={() => location.reload()}>Reload</button>
        <pre className="crash-detail">{this.state.error}</pre>
      </div>
    );
  }
}
