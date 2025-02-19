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

    const unlisten = listen("audio-levels", (event: any) => {
      console.log("Received audio levels:", event.payload);
      setLevels(event.payload as number[]);
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
      {/* Main Content */}
      <main className="h-full w-full p-3">
        <div className="flex h-full items-end justify-center gap-[4px]">
          {levels.map((level, index) => (
            <div
              key={index}
              className="w-[20px] rounded-t-lg transition-all duration-75 ease-out"
              style={{
                height: `${Math.max(5, level * 1000)}%`,
                backgroundColor: `hsl(${level * 240}, 100%, 50%)`,
                boxShadow: `0 0 20px hsl(${level * 240}, 100%, 50%)`,
              }}
            />
          ))}
        </div>
      </main>
    </div>
  );
}

export default MainWindow;
