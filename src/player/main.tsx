import { StrictMode } from "react";
import { installDevMock } from "../lib/devMock";
import { createRoot } from "react-dom/client";
import Player from "./Player";
import "../styles/tokens.css";
import "./player.css";

installDevMock();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Player />
  </StrictMode>,
);
