import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { GameStateProvider } from "./hooks/useGameState";
import { initI18n } from "./i18n";
import "@fontsource/silkscreen";
import "./theme.css";
import "./overrides.css";

initI18n();

document.addEventListener("contextmenu", (e) => e.preventDefault());

document.addEventListener("keydown", (e) => {
  const key = e.key.toLowerCase();
  const blocked =
    e.key === "F12" ||
    (e.ctrlKey && e.shiftKey && ["i", "j", "c"].includes(key)) ||
    (e.ctrlKey && key === "u");
  if (blocked) e.preventDefault();
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <GameStateProvider>
      <App />
    </GameStateProvider>
  </React.StrictMode>,
);