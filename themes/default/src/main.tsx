import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import { loadCustomLocales } from "@/lib/i18n";
import "./app/globals.css";

// Load custom locales from window.__CUSTOM_LOCALES__ (injected by backend)
loadCustomLocales();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </React.StrictMode>
);
