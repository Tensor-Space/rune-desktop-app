import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";
import { cn } from "@/lib/utils";

type TranscriptionStatus = "idle" | "started" | "completed" | "error";

function MainWindow() {
  const [levels, setLevels] = useState(new Array(8).fill(0));
  const [hasMicPermission, setHasMicPermission] = useState<boolean | null>(
    null,
  );
  const [transcriptionStatus, setTranscriptionStatus] =
    useState<TranscriptionStatus>("idle");

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
      console.log("Received audio levels:", newLevels);

      if (Array.isArray(newLevels) && newLevels.length === 8) {
        // Apply some smoothing to prevent jarring transitions
        setLevels((prevLevels) =>
          newLevels.map((level, i) => {
            const smoothingFactor = 0.7; // Adjust this value to change smoothing amount
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

    return () => {
      unlisten.then((unlistenFn) => unlistenFn());
      unlistenTranscriptionStatus.then((unlistenFn) => unlistenFn());
    };
  }, []);

  if (hasMicPermission === false) {
    return (
      <div className="p-2 bg-gray-900">
        <Alert variant="destructive" className="py-2">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>
            Please enable microphone access to continue
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "relative h-screen w-screen bg-gray-900 border-2 border-transparent rounded-lg",
        transcriptionStatus === "started" && "animate-border",
        transcriptionStatus === "completed" && "border-transparent",
        transcriptionStatus === "error" && "border-transparent",
        transcriptionStatus === "idle" && "border-transparent",
      )}
    >
      <main className="h-full w-full p-3">
        <div className="flex h-full items-end justify-center gap-1">
          {levels.map((level, index) => {
            // Calculate a color based on the level intensity
            const intensity = Math.min(1, level * 2);
            const hue = 200 + intensity * 60; // Range from blue to purple
            const lightness = 40 + intensity * 20; // Brighter as level increases

            return (
              <div
                key={index}
                className="w-12 rounded-t-lg transition-all duration-75 ease-out"
                style={{
                  height: `${Math.max(2, Math.min(100, level * 1000))}%`,
                  backgroundColor: `hsl(${hue}, 100%, ${lightness}%)`,
                  boxShadow: `0 0 ${10 + intensity * 10}px hsl(${hue}, 100%, ${lightness}%)`,
                }}
              />
            );
          })}
        </div>
      </main>
    </div>
  );
}

export default MainWindow;
