import React from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.jsx";
import {
  applyDocumentTheme,
  resolvePreviewThemePreference,
} from "./theme";
import "./styles.css";

// Apply theme before first paint to avoid a light-theme flash in dark mode.
applyDocumentTheme(resolvePreviewThemePreference());

createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
