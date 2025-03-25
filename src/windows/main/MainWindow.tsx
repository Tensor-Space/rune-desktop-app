import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { X, GripVertical } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type ProcessingStatus =
  | "idle"
  | "recording"
  | "transcribing"
  | "thinking_action"
  | "generating_text"
  | "completed"
  | "cancelled"
  | "error";

function MainWindow() {
  const [levels, setLevels] = useState(new Array(8).fill(0));
  const [processingStatus, setProcessingStatus] =
    useState<ProcessingStatus>("idle");
  const [dotPosition, setDotPosition] = useState(0);

  const stopRecording = useCallback(async () => {
    try {
      await invoke("cancel_recording");
      await getCurrentWindow().hide();
    } catch (error) {
      console.error("Error stopping recording:", error);
    }
  }, []);

  // Animation for the processing states
  useEffect(() => {
    let animationInterval: number | undefined;

    if (
      ["transcribing", "thinking_action", "generating_text"].includes(
        processingStatus,
      )
    ) {
      setLevels(new Array(8).fill(0));
      animationInterval = window.setInterval(() => {
        setDotPosition((prev) => (prev + 1) % 6);
      }, 150); // Speed of the animation
    }

    return () => {
      if (animationInterval) clearInterval(animationInterval);
    };
  }, [processingStatus]);

  useEffect(() => {
    const unlisten = listen("audio-levels", (event: any) => {
      const newLevels = event.payload as number[];
      if (Array.isArray(newLevels) && newLevels.length === 8) {
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

    const unlistenProcessingStatus = listen(
      "audio-processing-status",
      (event: any) => {
        setProcessingStatus(event.payload as ProcessingStatus);
      },
    );

    return () => {
      unlisten.then((unlistenFn) => unlistenFn());
      unlistenProcessingStatus.then((unlistenFn) => unlistenFn());
    };
  }, [stopRecording]);

  const getStatusText = () => {
    switch (processingStatus) {
      case "transcribing":
        return "Transcribing...";
      case "thinking_action":
        return "Thinking...";
      case "generating_text":
        return "Generating...";
      default:
        return null;
    }
  };

  const statusText = getStatusText();

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

        {["transcribing", "thinking_action", "generating_text"].includes(
          processingStatus,
        ) ? (
          <div className="flex items-center gap-[2px] h-8 relative">
            {statusText && (
              <div className="absolute -top-7 left-1/2 transform -translate-x-1/2 whitespace-nowrap text-xs text-gray-300 bg-[#1C1C1C] px-2 py-1 rounded-md">
                {statusText}
              </div>
            )}
            {Array.from({ length: 6 }).map((_, index) => (
              <div
                key={index}
                className={`w-[6px] h-[6px] rounded-full transition-all duration-100 ${
                  index === dotPosition
                    ? "bg-green-500 scale-125"
                    : "bg-gray-500 opacity-50"
                }`}
              />
            ))}
          </div>
        ) : (
          <div className="flex items-center gap-[2px] h-8">
            {levels.map((level, index) => {
              const height = Math.max(10, Math.min(60, level * 500));

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
        )}

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
