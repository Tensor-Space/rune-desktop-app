import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { X, GripVertical } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type TranscriptionStatus = "idle" | "started" | "completed" | "error";

function MainWindow() {
  const [levels, setLevels] = useState(new Array(8).fill(0));
  const [hasMicPermission, setHasMicPermission] = useState<boolean | null>(
    null,
  );
  const [, setTranscriptionStatus] = useState<TranscriptionStatus>("idle");

  const stopRecording = useCallback(async () => {
    try {
      await invoke("cancel_recording");
      await getCurrentWindow().hide();
    } catch (error) {
      console.error("Error stopping recording:", error);
    }
  }, []);

  useEffect(() => {
    const requestMic = async () => {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({
          audio: true,
        });
        stream.getTracks().forEach((track) => track.stop());
        setHasMicPermission(true);
      } catch (err) {
        console.error("Error accessing microphone:", err);
        setHasMicPermission(false);
      }
    };
    requestMic();

    // Listen for audio levels
    const unlisten = listen("audio-levels", (event: any) => {
      const newLevels = event.payload as number[];
      if (Array.isArray(newLevels) && newLevels.length === 8) {
        // Apply some smoothing to prevent jarring transitions
        setLevels((prevLevels) =>
          newLevels.map((level, i) => {
            const smoothingFactor = 0.7;
            return (
              level * smoothingFactor + prevLevels[i] * (1 - smoothingFactor)
            );
          }),
        );
      }
    });

    const unlistenTranscriptionStatus = listen(
      "transcription-status",
      (event: any) => {
        setTranscriptionStatus(event.payload as TranscriptionStatus);
      },
    );

    // Register Escape key shortcut
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        stopRecording();
      }
    };

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      unlisten.then((unlistenFn) => unlistenFn());
      unlistenTranscriptionStatus.then((unlistenFn) => unlistenFn());
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [stopRecording]);

  if (hasMicPermission === false) {
    return (
      <div className="p-2 bg-black rounded-full">
        <div className="text-white text-sm px-4 py-2">
          Please enable microphone access to continue
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen w-screen bg-transparent">
      <div className="flex items-center justify-between bg-[#1C1C1C] rounded-full px-3 py-1 w-auto max-w-[300px] h-10 border-2 border-[#FFFFFF]/10">
        {/* Menu button */}
        <button
          className="text-gray-400 hover:text-white p-1"
          data-tauri-drag-region
        >
          <GripVertical size={18} />
        </button>

        {/* Audio visualizer */}
        <div className="flex items-center gap-[2px] h-8">
          {levels.map((level, index) => {
            // Calculate height based on audio level
            const height = Math.max(10, Math.min(60, level * 2000));

            return (
              <div
                key={index}
                className="w-[3px] rounded-sm transition-all duration-75 ease-out bg-green-500"
                style={{
                  height: `${height}%`,
                }}
              />
            );
          })}
        </div>

        {/* Close button */}
        <button
          onClick={stopRecording}
          className="text-gray-400 hover:text-white p-1"
        >
          <X size={18} />
        </button>
      </div>
    </div>
  );
}

export default MainWindow;
