import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { HotkeyRecorder } from "./HotkeyRecorder";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@/components/ui/card";
import { Settings, ShortcutConfig } from "../types";

export const HotkeySettings = () => {
  const [shortcuts, setShortcuts] = useState<ShortcutConfig>({
    record_key: "",
    record_modifier: "",
  });
  const [hasAccessibilityPermissions, setHasAccessibilityPermissions] =
    useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadHotkeySettings = async () => {
      try {
        const settings = await invoke<Settings>("get_settings");
        setShortcuts(settings.shortcuts);

        const hasPermissions = await invoke<boolean>(
          "check_accessibility_permissions",
        );
        setHasAccessibilityPermissions(hasPermissions);
      } catch (error) {
        setError("Failed to load hotkey settings");
        console.error("Failed to load hotkey settings:", error);
      }
    };

    loadHotkeySettings();
  }, []);

  const handleHotkeySet = async (key: string, modifier?: string) => {
    try {
      const newShortcuts: ShortcutConfig = {
        record_key: key,
        record_modifier: modifier || "", // Now just a string
      };

      await invoke("update_shortcuts", {
        modifier: newShortcuts.record_modifier, // Changed from modifiers
        key: newShortcuts.record_key,
      });

      setShortcuts(newShortcuts);
      setError(null);
    } catch (error) {
      setError("Failed to update shortcuts");
      console.error("Failed to update shortcuts:", error);
    }
  };

  const handleHotkeyRemove = async () => {
    try {
      await invoke("update_shortcuts", {
        key: "",
        modifiers: "",
      });
      setShortcuts({ record_key: "", record_modifier: "" });
      setError(null);
    } catch (error) {
      setError("Failed to remove shortcut");
      console.error("Failed to remove shortcut:", error);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Keyboard Shortcuts</CardTitle>
        <CardDescription>
          Configure keyboard shortcuts for quick actions
        </CardDescription>
      </CardHeader>
      <CardContent>
        {hasAccessibilityPermissions === false && (
          <Alert variant="destructive" className="mb-4">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription className="flex items-center justify-between">
              <span>
                Accessibility permissions are required for keyboard shortcuts.
              </span>
              <button
                onClick={async () => {
                  try {
                    const granted = await invoke<boolean>(
                      "request_accessibility_permissions",
                    );
                    setHasAccessibilityPermissions(granted);
                  } catch (error) {
                    setError("Failed to request permissions");
                  }
                }}
                className="text-sm underline hover:no-underline"
              >
                Grant Permissions
              </button>
            </AlertDescription>
          </Alert>
        )}

        {error && (
          <Alert variant="destructive" className="mb-4">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        <HotkeyRecorder
          onHotkeySet={handleHotkeySet}
          onHotkeyRemove={handleHotkeyRemove}
          shortcut={shortcuts}
        />
      </CardContent>
    </Card>
  );
};
