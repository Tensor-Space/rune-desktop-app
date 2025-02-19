import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { X } from "lucide-react";
import { ShortcutConfig } from "../types";

interface HotkeyRecorderProps {
  onHotkeySet: (key: string, modifier?: string) => void;
  onHotkeyRemove: () => void;
  shortcut: ShortcutConfig;
}

// Map JS key names to the format expected by the Rust backend
const keyToCode = (key: string): string => {
  // Handle special keys
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

  // Check if it's a special key
  if (key in specialKeys) {
    return specialKeys[key];
  }

  // Handle letter keys
  if (/^[A-Z]$/.test(key)) {
    return `Key${key}`;
  }

  // Handle number keys
  if (/^[0-9]$/.test(key)) {
    return `Digit${key}`;
  }

  // Handle F keys
  if (/^F\d+$/.test(key)) {
    return key.toUpperCase();
  }

  return key.toUpperCase();
};

// Map JS modifier keys to the format expected by the Rust backend
const modifierToCode = (key: string): string => {
  const modifierMap: Record<string, string> = {
    Control: "CONTROL",
    Shift: "SHIFT",
    Alt: "ALT",
    Meta: "SUPER", // Command on macOS, Windows key on Windows
  };

  return modifierMap[key] || key.toUpperCase();
};

export const HotkeyRecorder = ({
  onHotkeySet,
  onHotkeyRemove,
  shortcut,
}: HotkeyRecorderProps) => {
  const [isRecording, setIsRecording] = useState(false);
  const [recordingStep, setRecordingStep] = useState<"key" | "modifier">("key");
  const [tempKey, setTempKey] = useState("");

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isRecording) return;
      e.preventDefault();

      // Only accept modifier keys for the modifier step
      if (recordingStep === "modifier") {
        if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) {
          const formattedModifier = modifierToCode(e.key);
          setIsRecording(false);
          setRecordingStep("key");
          onHotkeySet(tempKey, formattedModifier);
        }
        // Ignore non-modifier keys in modifier step
        return;
      }

      // Key recording step
      if (recordingStep === "key") {
        const formattedKey = keyToCode(e.key.toUpperCase());
        setTempKey(formattedKey);
        setRecordingStep("modifier");
      }
    };

    if (isRecording) {
      window.addEventListener("keydown", handleKeyDown);
      const timeout = setTimeout(() => setIsRecording(false), 5000);
      return () => {
        window.removeEventListener("keydown", handleKeyDown);
        clearTimeout(timeout);
      };
    }
  }, [isRecording, recordingStep, tempKey, onHotkeySet]);

  const formatHotkeyDisplay = (shortcut: ShortcutConfig): string => {
    if (!shortcut.record_key) return "";

    const modifier = shortcut.record_modifier
      ? shortcut.record_modifier.charAt(0).toUpperCase() +
        shortcut.record_modifier.slice(1).toLowerCase()
      : "";

    const key = shortcut.record_key
      .replace(/^KEY/, "")
      .replace(/^DIGIT/, "")
      .replace(/^ARROW/, "");

    return modifier ? `${modifier}+${key}` : key;
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <Button
          variant={isRecording ? "destructive" : "default"}
          onClick={() => {
            setIsRecording(true);
            setTempKey("");
          }}
          disabled={isRecording || Boolean(shortcut.record_key)}
        >
          {isRecording
            ? `Press ${recordingStep === "key" ? "key" : "modifier"}...`
            : "Add New Hotkey"}
        </Button>
      </div>

      {shortcut.record_key && (
        <div className="flex items-center gap-2 bg-secondary p-2 rounded-md">
          <Badge variant="secondary">{formatHotkeyDisplay(shortcut)}</Badge>
          <Button
            variant="ghost"
            size="sm"
            onClick={onHotkeyRemove}
            className="ml-auto"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      )}
    </div>
  );
};
