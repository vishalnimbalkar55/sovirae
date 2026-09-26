import { StrictMode } from "react";
import { installDevMock } from "./lib/devMock";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles/tokens.css";
import "./styles/app.css";

installDevMock();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
