import { StrictMode } from "react";
import { installDevMock } from "./lib/devMock";
import { createRoot } from "react-dom/client";
import App from "./App";
import { ErrorBoundary, installErrorReporting } from "./lib/crash";
import "./styles/tokens.css";
import "./styles/app.css";

installErrorReporting("main");
installDevMock();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary name="main">
      <App />
    </ErrorBoundary>
  </StrictMode>,
);
