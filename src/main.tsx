import React from "react";
import ReactDOM from "react-dom/client";
import { Route, Routes } from "react-router";
import { BrowserRouter } from "react-router";
import "./global.css";
import { SettingsWindow } from "./windows/settings/SettingsWindow";
import { HistoryView } from "./windows/history/HistoryView";
import MainWindow from "./windows/main/MainWindow";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<MainWindow />} />
        <Route path="settings" element={<SettingsWindow />} />
        <Route path="history" element={<HistoryView />} />
      </Routes>
    </BrowserRouter>
  </React.StrictMode>,
);
