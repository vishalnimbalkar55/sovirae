import { StrictMode } from "react";
import { installDevMock } from "../lib/devMock";
import { createRoot } from "react-dom/client";
import Player from "./Player";
import { ErrorBoundary, installErrorReporting } from "../lib/crash";
import "../styles/tokens.css";
import "./player.css";

installErrorReporting("player");
installDevMock();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary name="player">
      <Player />
    </ErrorBoundary>
  </StrictMode>,
);
