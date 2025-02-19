import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ScrollArea } from "@/components/ui/scroll-area";
import { HotkeySettings } from "./HotkeySettings/HotkeySettings";
import { AudioSettings } from "./AudioSettings/AudioSettings";
import { AccessibilitySettings } from "./AccessibilitySettings/AccessibilitySettings";
import { MicrophoneSettings } from "./MicrophoneSettings/MicrophoneSettings";
import { Settings } from "./types";

export const SettingsWindow = () => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const initializeSettings = async () => {
      try {
        // Load initial settings
        await invoke<Settings>("get_settings");
        setError(null);
      } catch (error) {
        setError("Failed to initialize settings");
        console.error("Settings initialization error:", error);
      } finally {
        setIsLoading(false);
      }
    };

    initializeSettings();
  }, []);

  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="text-lg text-muted-foreground">Loading settings...</div>
      </div>
    );
  }

  return (
    <ScrollArea className="h-screen w-full bg-background">
      <div className="container mx-auto max-w-3xl py-8 px-4 space-y-8">
        {error && <div className="text-red-500 text-center mb-4">{error}</div>}

        <AccessibilitySettings />
        <MicrophoneSettings />
        <HotkeySettings />
        <AudioSettings />

        <div className="text-center text-sm text-muted-foreground">
          <p>Rune v1.0.0</p>
        </div>
      </div>
    </ScrollArea>
  );
};
