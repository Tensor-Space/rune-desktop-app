import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { X, GripVertical } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import posthog from "posthog-js"; // Make sure you have posthog-js installed

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
  const [transcript, setTranscript] = useState<string>("");
  const [sessionId, setSessionId] = useState<string>("");

  // Generate a unique session ID when the component mounts
  useEffect(() => {
    setSessionId(
      `session_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`,
    );
  }, []);

  const stopRecording = useCallback(async () => {
    try {
      // Track when recording is stopped by user
      posthog.capture("recording_stopped", {
        session_id: sessionId,
        transcript: transcript,
        processing_status: processingStatus,
      });

      await invoke("cancel_recording");
      await getCurrentWindow().hide();
    } catch (error) {
      console.error("Error stopping recording:", error);

      // Track errors
      posthog.capture("recording_stop_error", {
        session_id: sessionId,
        error: String(error),
      });
    }
  }, [processingStatus, sessionId, transcript]);

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

  // Track status changes with PostHog
  useEffect(() => {
    if (processingStatus !== "idle") {
      posthog.capture(`status_changed_${processingStatus}`, {
        session_id: sessionId,
        timestamp: new Date().toISOString(),
        transcript: transcript,
      });
    }
  }, [processingStatus, sessionId, transcript]);

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
        const newStatus = event.payload as ProcessingStatus;
        setProcessingStatus(newStatus);

        // For completed status, we need to track the full session completion
        if (newStatus === "completed") {
          posthog.capture("recording_completed", {
            session_id: sessionId,
            transcript: transcript,
            duration: new Date().getTime() - parseInt(sessionId.split("_")[1]),
          });
        }

        // For error status, we track the error
        if (newStatus === "error") {
          posthog.capture("recording_error", {
            session_id: sessionId,
            transcript: transcript,
          });
        }
      },
    );

    // Listen for transcript updates
    const unlistenTranscript = listen("transcription-result", (event: any) => {
      const newTranscript = event.payload as string;
      setTranscript(newTranscript);

      posthog.capture("transcript_updated", {
        session_id: sessionId,
        transcript: newTranscript,
        processing_status: processingStatus,
      });
    });

    // Track when recording starts
    posthog.capture("recording_started", {
      session_id: sessionId,
      timestamp: new Date().toISOString(),
    });

    return () => {
      unlisten.then((unlistenFn) => unlistenFn());
      unlistenProcessingStatus.then((unlistenFn) => unlistenFn());
      unlistenTranscript.then((unlistenFn) => unlistenFn());
    };
  }, [sessionId, processingStatus, transcript]);

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
          onClick={() =>
            posthog.capture("menu_button_clicked", { session_id: sessionId })
          }
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
