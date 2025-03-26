import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { format, parseISO } from "date-fns";

type TranscriptionHistory = {
  id: number;
  timestamp: string;
  text: string;
};

export const HistoryView = () => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [transcriptions, setTranscriptions] = useState<TranscriptionHistory[]>(
    [],
  );
  const [copiedId, setCopiedId] = useState<number | null>(null);

  const copyToClipboard = async (text: string, id: number) => {
    try {
      await navigator.clipboard.writeText(text);
      // Show copy feedback
      setCopiedId(id);
      // Hide copy feedback after 2 seconds
      setTimeout(() => {
        setCopiedId(null);
      }, 2000);
    } catch (err) {
      console.error("Failed to copy text:", err);
    }
  };

  // Load transcriptions without setting loading state (for refreshes)
  const refreshTranscriptions = async () => {
    try {
      console.log("Refreshing transcription history...");
      const result = await invoke<TranscriptionHistory[]>(
        "get_transcription_history",
      );
      console.log("Received transcriptions:", result);
      setTranscriptions(result);
      setError(null);
    } catch (error) {
      console.error("Failed to refresh transcriptions:", error);
      setError(`Failed to refresh transcription history: ${error}`);
    }
  };

  // Initial load of transcriptions with loading state
  const loadTranscriptions = async () => {
    try {
      console.log("Initial loading of transcription history...");
      setIsLoading(true);
      const result = await invoke<TranscriptionHistory[]>(
        "get_transcription_history",
      );
      console.log("Received transcriptions:", result);
      setTranscriptions(result);
      setError(null);
    } catch (error) {
      console.error("Failed to load transcriptions:", error);
      setError(`Failed to load transcription history: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    // Prevent scrolling on body and html
    document.body.style.overflow = "hidden";
    document.documentElement.style.overflow = "hidden";

    // Initialize with transcriptions
    loadTranscriptions();

    // Setup a periodic refresh every 5 seconds as a fallback
    // This ensures transcriptions appear even if events don't work
    const intervalId = setInterval(() => {
      refreshTranscriptions();
    }, 10000);

    // Try to set up the event listener, but don't fail if it doesn't work
    let unlistenFn: UnlistenFn | null = null;

    const setupEventListener = async () => {
      try {
        // Listen for the transcription-added event
        const unlisten = await listen<any>("transcription-added", () => {
          console.log("Received transcription-added event");
          refreshTranscriptions();
        });
        unlistenFn = unlisten;
        console.log("Event listener set up successfully");
      } catch (err) {
        console.warn(
          "Could not set up event listener, falling back to polling:",
          err,
        );
      }
    };

    setupEventListener();

    return () => {
      // Cleanup
      document.body.style.overflow = "";
      document.documentElement.style.overflow = "";

      // Clear the interval
      clearInterval(intervalId);

      // Remove event listener if it was set up
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, []);

  const formatTimestamp = (timestamp: string): string => {
    try {
      return format(parseISO(timestamp), "h:mm a");
    } catch (e) {
      return timestamp;
    }
  };

  // Sort transcriptions by timestamp (latest first)
  const sortedTranscriptions = [...transcriptions].sort((a, b) => {
    return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
  });

  if (isLoading) {
    return (
      <div className="dark flex h-screen items-center justify-center bg-background">
        <div className="text-lg text-muted-foreground">
          Loading transcriptions...
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="dark flex h-screen items-center justify-center bg-background">
        <div className="text-lg text-destructive">{error}</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen bg-background text-foreground ">
      <div
        className="h-[30px] flex items-center border-b border-neutral-800"
        data-tauri-drag-region
      ></div>

      <div className=" flex-1 overflow-auto border border-[#2A2A2A] rounded-t-[12px] bg-[#1F1F1F] divide-y divide-[#2A2A2A] mx-4 mt-4">
        {sortedTranscriptions.map((item) => (
          <div
            key={item.id}
            className="bg-[#1F1F1F] py-[28px] pr-[28px] pl-[28px] grid grid-cols-[150px_1fr_50px] gap-4 items-start"
          >
            <span className="text-[14px] leading-[24px] font-medium text-[#8A8A8A] whitespace-nowrap">
              {formatTimestamp(item.timestamp)}
            </span>
            <p className="text-[#C0C0C0] break-words font-medium text-[14px] leading-[24px] tracking-[-.35px]">
              {item.text}
            </p>
            <div className="relative">
              <button
                onClick={() => copyToClipboard(item.text, item.id)}
                className="text-[#8A8A8A] hover:text-[#C0C0C0] transition-colors w-6 h-6 flex items-center justify-center rounded-sm hover:bg-[#2A2A2A] mt-[-3px]"
                title="Copy to clipboard"
              >
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 16 16"
                  fill="none"
                  xmlns="http://www.w3.org/2000/svg"
                >
                  <path
                    d="M4 4.5H4.5V4V0.666667C4.5 0.622464 4.51756 0.580072 4.54882 0.548816C4.58007 0.517559 4.62246 0.5 4.66667 0.5H15.3333C15.3775 0.5 15.4199 0.51756 15.4512 0.548815C15.4824 0.58007 15.5 0.622463 15.5 0.666667V11.3333C15.5 11.3775 15.4824 11.4199 15.4512 11.4512C15.4199 11.4824 15.3775 11.5 15.3333 11.5H12H11.5V12V15.3333C11.5 15.3775 11.4824 15.4199 11.4512 15.4512C11.4199 15.4824 11.3775 15.5 11.3333 15.5H0.666667C0.622463 15.5 0.58007 15.4824 0.548815 15.4512C0.51756 15.4199 0.5 15.3775 0.5 15.3333V4.66667C0.5 4.62246 0.517559 4.58007 0.548816 4.54882C0.580072 4.51756 0.622464 4.5 0.666667 4.5H4ZM10.6667 15.1667H11.1667V14.6667V5.33333V4.83333H10.6667H1.33333H0.833333V5.33333V14.6667V15.1667H1.33333H10.6667ZM14.6667 11.1667H15.1667V10.6667V1.33333V0.833333H14.6667H5.33333H4.83333V1.33333V4V4.5H5.33333H11.3333C11.3775 4.5 11.4199 4.51756 11.4512 4.54882C11.4824 4.58007 11.5 4.62246 11.5 4.66667V10.6667V11.1667H12H14.6667Z"
                    fill="currentColor"
                    stroke="currentColor"
                  />
                </svg>
              </button>
              {copiedId === item.id && (
                <div className="absolute right-0 bottom-8  bg-[#2A2A2A] text-[#C0C0C0] px-2 py-1 rounded text-xs whitespace-nowrap">
                  Copied!
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
