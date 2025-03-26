import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Keyboard, Plus, AlertCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@/components/ui/card";
import { Settings, ShortcutConfig } from "../types";

interface HotkeyRecorderProps {
  onShortcutChange?: (hasShortcut: boolean) => void;
}

type RecordingState =
  | "idle"
  | "recording-modifier"
  | "recording-key"
  | "complete";

const MODIFIER_KEYS = ["Control", "Alt", "Shift", "Meta"];

const modifierToCode = (key: string): string => {
  const modifierMap: Record<string, string> = {
    Control: "CONTROL",
    Shift: "SHIFT",
    Alt: "ALT",
    Meta: "SUPER",
  };
  return modifierMap[key] || key.toUpperCase();
};

const keyToCode = (key: string): string => {
  const specialKeys: Record<string, string> = {
    " ": "Space",
    ArrowUp: "ArrowUp",
    ArrowDown: "ArrowDown",
    ArrowLeft: "ArrowLeft",
    ArrowRight: "ArrowRight",
    Control: "Control",
    Shift: "Shift",
    Alt: "Alt",
    Meta: "Super",
    Enter: "Enter",
    Escape: "Escape",
    Backspace: "Backspace",
    Delete: "Delete",
    Tab: "Tab",
  };

  if (key in specialKeys) {
    return specialKeys[key];
  }

  if (/^[A-Z]$/.test(key)) {
    return `Key${key}`;
  }

  if (/^[0-9]$/.test(key)) {
    return `Digit${key}`;
  }

  if (/^F\d+$/.test(key)) {
    return key.toUpperCase();
  }

  return key.toUpperCase();
};

const getModifierLabel = (modifier: string): string => {
  switch (modifier) {
    case "CONTROL":
      return "Ctrl";
    case "ALT":
      return "Alt";
    case "SHIFT":
      return "Shift";
    case "SUPER":
      return "⌘";
    default:
      return modifier;
  }
};

const getKeyLabel = (key: string): string => {
  return key
    .replace(/^KEY/, "")
    .replace(/^DIGIT/, "")
    .replace(/^ARROW/, "Arrow");
};

export const HotkeyRecorder = ({ onShortcutChange }: HotkeyRecorderProps) => {
  const [shortcuts, setShortcuts] = useState<ShortcutConfig>({
    record_key: "",
    record_modifier: "",
  });
  const [hasAccessibilityPermissions, setHasAccessibilityPermissions] =
    useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [recordingState, setRecordingState] = useState<RecordingState>("idle");
  const [tempModifier, setTempModifier] = useState<string | null>(null);
  const [tempKey, setTempKey] = useState<string | null>(null);
  const [recorderError, setRecorderError] = useState<string | null>(null);

  useEffect(() => {
    const loadHotkeyRecorder = async () => {
      try {
        const settings = await invoke<Settings>("get_settings");
        setShortcuts(settings.shortcuts);

        // Notify parent component about shortcut status
        onShortcutChange?.(!!settings.shortcuts.record_key);

        const hasPermissions = await invoke<boolean>(
          "check_accessibility_permissions",
        );
        setHasAccessibilityPermissions(hasPermissions);
      } catch (error) {
        setError("Failed to load hotkey settings");
        console.error("Failed to load hotkey settings:", error);
      }
    };

    loadHotkeyRecorder();
  }, [onShortcutChange]);

  // Reset to idle when shortcut changes externally
  useEffect(() => {
    setRecordingState("idle");
    setTempModifier(null);
    setTempKey(null);
    setRecorderError(null);
  }, [shortcuts]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (recordingState === "idle" || recordingState === "complete") return;

      e.preventDefault();

      if (e.key === "Escape") {
        setRecordingState("idle");
        setTempModifier(null);
        setTempKey(null);
        setRecorderError(null);
        return;
      }

      if (recordingState === "recording-modifier") {
        if (MODIFIER_KEYS.includes(e.key)) {
          const modifier = modifierToCode(e.key);
          setTempModifier(modifier);
          setRecordingState("recording-key");
          setRecorderError(null);
        } else {
          // For non-modifier keys, we'll just use them without a modifier
          const key = keyToCode(e.key.toUpperCase());
          setTempKey(key);
          setTempModifier(null);
          setRecordingState("complete");
          handleHotkeySet(key);
        }
      } else if (recordingState === "recording-key") {
        if (!MODIFIER_KEYS.includes(e.key)) {
          const key = keyToCode(e.key.toUpperCase());
          setTempKey(key);
          setRecordingState("complete");

          // Submit the hotkey
          if (tempModifier) {
            handleHotkeySet(key, tempModifier);
          } else {
            handleHotkeySet(key);
          }
        } else {
          setRecorderError("Please press a non-modifier key");
        }
      }
    },
    [recordingState, tempModifier],
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  // Auto-transition from complete state back to idle
  useEffect(() => {
    if (recordingState === "complete") {
      const timer = setTimeout(() => {
        setRecordingState("idle");
      }, 1000);
      return () => clearTimeout(timer);
    }
  }, [recordingState]);

  const startRecording = () => {
    setRecordingState("recording-modifier");
    setTempModifier(null);
    setTempKey(null);
    setRecorderError(null);
  };

  const cancelRecording = () => {
    setRecordingState("idle");
    setTempModifier(null);
    setTempKey(null);
    setRecorderError(null);
  };

  const handleHotkeySet = async (key: string, modifier?: string) => {
    try {
      const newShortcuts: ShortcutConfig = {
        record_key: key,
        record_modifier: modifier || "",
      };

      await invoke("update_shortcuts", {
        modifier: newShortcuts.record_modifier,
        key: newShortcuts.record_key,
      });

      setShortcuts(newShortcuts);
      onShortcutChange?.(true);
      setError(null);
    } catch (error) {
      setError("Failed to update shortcuts");
      console.error("Failed to update shortcuts:", error);
    }
  };

  const renderKey = (
    text: string,
    active: boolean = false,
    highlight: boolean = false,
  ) => (
    <kbd
      className={cn(
        "px-2 py-1.5 text-sm font-semibold",
        "border rounded-md shadow-sm",
        "bg-muted text-muted-foreground",
        active && "bg-primary text-primary-foreground",
        highlight && "border-primary animate-pulse",
      )}
    >
      {text}
    </kbd>
  );

  const getInstructions = () => {
    switch (recordingState) {
      case "recording-modifier":
        return "Press a modifier key (Ctrl, Alt, Shift) or any key";
      case "recording-key":
        return "Now press any key";
      case "complete":
        return "Hotkey saved!";
      default:
        return shortcuts.record_key ? "" : "No hotkey set";
    }
  };

  const isRecording =
    recordingState === "recording-modifier" ||
    recordingState === "recording-key";

  return (
    <Card className="bg-card">
      <CardHeader className="pb-3 flex flex-row items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="flex flex-col gap-1">
            <CardTitle>Keyboard Shortcuts</CardTitle>
            <CardDescription>
              Configure global hotkeys to invoke RuneAI
            </CardDescription>
          </div>
        </div>

        {/* Action buttons at top right */}
        {recordingState === "idle" && (
          <div className="flex items-center gap-2">
            {shortcuts.record_key ? (
              <>
                <Button variant="secondary" onClick={startRecording}>
                  Change Hotkeys
                </Button>
              </>
            ) : (
              <Button variant="ghost" onClick={startRecording}>
                <Plus className="h-3 w-3" /> Add
              </Button>
            )}
          </div>
        )}
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

        <div className="my-2 flex flex-col gap-2">
          <div
            className={cn(
              "flex justify-center items-center p-8 rounded-lg min-h-[80px] gap-2",
              isRecording
                ? "bg-primary/10 border-2 border-dashed border-primary/50"
                : "bg-muted/50",
              recordingState === "complete" &&
                "bg-green-100/20 border border-green-500/50",
            )}
          >
            {/* Center-aligned keys */}
            <div className="flex justify-center items-center h-full w-full">
              {/* Display existing shortcut */}
              {recordingState === "idle" && shortcuts.record_key && (
                <div className="flex items-center justify-center gap-2">
                  {shortcuts.record_modifier &&
                    renderKey(
                      getModifierLabel(shortcuts.record_modifier),
                      true,
                    )}
                  {shortcuts.record_modifier && <span>+</span>}
                  {renderKey(getKeyLabel(shortcuts.record_key), true)}
                </div>
              )}

              {/* Display in-progress recording */}
              {recordingState === "recording-modifier" && (
                <div className="text-sm text-primary animate-pulse flex items-center gap-2">
                  <Keyboard className="h-4 w-4" />
                  {getInstructions()}
                </div>
              )}

              {recordingState === "recording-key" && (
                <div className="flex items-center gap-2">
                  {tempModifier &&
                    renderKey(getModifierLabel(tempModifier), true)}
                  {tempModifier && <span>+</span>}
                  <div className="text-sm text-primary animate-pulse">
                    {getInstructions()}
                  </div>
                </div>
              )}

              {recordingState === "complete" && (
                <div className="flex items-center justify-center gap-2">
                  {tempModifier &&
                    renderKey(getModifierLabel(tempModifier), true, true)}
                  {tempModifier && <span>+</span>}
                  {tempKey && renderKey(getKeyLabel(tempKey), true, true)}
                </div>
              )}

              {/* Empty state */}
              {recordingState === "idle" && !shortcuts.record_key && (
                <div className="text-sm text-muted-foreground">
                  {getInstructions()}
                </div>
              )}
            </div>
          </div>

          {/* Error message */}
          {recorderError && (
            <div className="text-sm text-destructive text-center">
              {recorderError}
            </div>
          )}

          {/* Cancel button while recording */}
          {isRecording && (
            <Button
              variant="destructive"
              className="w-full"
              onClick={cancelRecording}
            >
              Cancel
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
};
