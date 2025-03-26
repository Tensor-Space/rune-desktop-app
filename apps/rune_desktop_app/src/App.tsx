import { Route, Routes } from "react-router";
import { BrowserRouter } from "react-router";
import { SettingsWindow } from "./windows/settings/SettingsWindow";
import { HistoryView } from "./windows/history/HistoryView";
import MainWindow from "./windows/main/MainWindow";
import { OnboardingWindow } from "./windows/onboarding/OnboardingWindow";
import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Settings } from "./windows/settings/types";
import posthog from "posthog-js";

export const App = () => {
  const fetchSettingsAndIdentify = async () => {
    const settings = await invoke<Settings>("get_settings");
    if (settings?.user_profile?.email) {
      posthog.identify(settings.user_profile.email, settings.user_profile);
    }
  };
  useEffect(() => {
    fetchSettingsAndIdentify();
  }, []);

  return (
    <>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<MainWindow />} />
          <Route path="settings" element={<SettingsWindow />} />
          <Route path="history" element={<HistoryView />} />
          <Route path="onboarding" element={<OnboardingWindow />} />
        </Routes>
      </BrowserRouter>
    </>
  );
};
