import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import { loadCustomLocales } from "@/lib/i18n";
import "./app/globals.css";

// Load custom locales from window.__CUSTOM_LOCALES__ (injected by backend)
loadCustomLocales();

// Console logo
console.log('%c Noteva ', 'background: #4a90e2; color: white; font-size: 24px; font-weight: bold; padding: 10px 20px; border-radius: 5px;');
console.log('%c🔗 Github: https://github.com/noteva26/Noteva', 'color: #666; font-size: 14px; margin-top: 10px;');

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </React.StrictMode>
);
