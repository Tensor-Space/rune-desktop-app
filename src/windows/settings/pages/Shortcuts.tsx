import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { HotkeyRecorder } from "../components/HotkeyRecorder";
import { Settings } from "../types";

interface ShortcutsProps {
  onComplete: () => void;
  isStepComplete?: boolean;
}

export const Shortcuts = ({ onComplete }: ShortcutsProps) => {
  const [, setHasSetShortcut] = useState(false);

  useEffect(() => {
    const checkShortcuts = async () => {
      try {
        const settings = await invoke<Settings>("get_settings");
        const hasShortcut = !!settings.shortcuts.record_key;
        setHasSetShortcut(hasShortcut);
        if (hasShortcut) {
          onComplete();
        }
      } catch (error) {
        console.error(error);
      }
    };

    checkShortcuts();
  }, [onComplete]);

  return (
    <div className="container mx-auto">
      <HotkeyRecorder
        onShortcutChange={(shortcutSet) => {
          setHasSetShortcut(shortcutSet);
          if (shortcutSet) {
            onComplete();
          }
        }}
      />
    </div>
  );
};
