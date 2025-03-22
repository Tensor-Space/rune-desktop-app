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
    const [transcriptions, setTranscriptions] = useState<TranscriptionHistory[]>([]);

    const copyToClipboard = async (text: string) => {
        try {
            await navigator.clipboard.writeText(text);
            // Optionally add toast notification here
        } catch (err) {
            console.error('Failed to copy text:', err);
        }
    };

    // Load transcriptions without setting loading state (for refreshes)
    const refreshTranscriptions = async () => {
        try {
            console.log("Refreshing transcription history...");
            const result = await invoke<TranscriptionHistory[]>("get_transcription_history");
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
            const result = await invoke<TranscriptionHistory[]>("get_transcription_history");
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
                console.warn("Could not set up event listener, falling back to polling:", err);
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
                <div className="text-lg text-muted-foreground">Loading transcriptions...</div>
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
        <div className="flex flex-col h-screen bg-background text-foreground px-28 pt-10 pb-0">
            <h1 className="text-2xl font-semibold w-[393px] h-[29px] mb-4 text-[#F6F6F6] tracking-[-0.35px]">History</h1>
            <span className="text-sm text-muted-foreground h-[17px] flex items-center gap-1.5 mb-16 mt-2 font-normal">
                <svg width="13" height="15" viewBox="0 0 13 15" fill="none" xmlns="http://www.w3.org/2000/svg">
                    <path fill-rule="evenodd" clip-rule="evenodd" d="M1.85166 5.09302C1.98871 2.53047 3.97284 0.417969 6.50001 0.417969C9.02717 0.417969 11.0113 2.53047 11.1483 5.09302C11.8972 5.52138 12.4573 6.23353 12.7256 7.07563C12.8481 7.46041 12.8799 7.86432 12.8744 8.32251C12.8691 8.77044 12.8265 9.32126 12.7732 10.0093L12.76 10.1803C12.677 11.2585 12.6209 11.9867 12.3507 12.5813C11.9909 13.373 11.361 14.0045 10.5725 14.3357C10.242 14.4745 9.89881 14.5317 9.51823 14.5587C9.15113 14.5846 8.70361 14.5846 8.15652 14.5846H4.84349C4.2964 14.5846 3.84889 14.5846 3.48178 14.5587C3.1012 14.5317 2.75799 14.4745 2.42748 14.3357C1.63898 14.0045 1.00913 13.373 0.649344 12.5813C0.379099 11.9867 0.323025 11.2585 0.240002 10.1803L0.226814 10.0094C0.173552 9.32134 0.130909 8.77047 0.125579 8.32252C0.120128 7.86432 0.151892 7.46041 0.274449 7.07563C0.542664 6.23353 1.10286 5.52138 1.85166 5.09302ZM3.31981 4.69593C3.34998 4.69388 3.38037 4.69199 3.41098 4.69024C3.80236 4.66796 4.27679 4.66797 4.84908 4.66797H8.15094C8.72322 4.66797 9.19766 4.66796 9.58903 4.69024C9.61964 4.69199 9.65003 4.69388 9.6802 4.69593C9.38675 3.03318 8.03785 1.83464 6.50001 1.83464C4.96216 1.83464 3.61326 3.03318 3.31981 4.69593ZM6.50009 7.5013C5.69719 7.5013 5.13383 8.18571 5.13383 8.91797C5.13383 9.40723 5.38534 9.87514 5.79176 10.1318V11.043C5.79176 11.4342 6.10889 11.7513 6.50009 11.7513C6.89129 11.7513 7.20843 11.4342 7.20843 11.043V10.1318C7.61485 9.87514 7.86635 9.40723 7.86635 8.91797C7.86635 8.18571 7.30299 7.5013 6.50009 7.5013Z" fill="#C0C0C0"/>
                </svg>
                All transcripts are private and stored locally on your device.
            </span>

            <p className="text-[15px] mb-4" style={{ letterSpacing: '-0.1px' }}>All history</p>

            <div className="overflow-auto border border-[#2A2A2A] rounded-t-[12px] bg-[#1F1F1F] divide-y divide-[#2A2A2A]">
                {sortedTranscriptions.map((item) => (
                    <div key={item.id} className="bg-[#1F1F1F] py-[28px] pr-[28px] pl-[28px] grid grid-cols-[150px_1fr_50px] gap-4 items-start">
                        <span className="text-[14px] leading-[24px] font-medium text-[#8A8A8A] whitespace-nowrap">
                            {formatTimestamp(item.timestamp)}
                        </span>
                        <p className="text-[#C0C0C0] break-words font-medium text-[14px] leading-[24px] tracking-[-.35px]">
                            {item.text}
                        </p>
                        <button 
                            onClick={() => copyToClipboard(item.text)}
                            className="text-[#8A8A8A] hover:text-[#C0C0C0] transition-colors w-6 h-6 flex items-center justify-center rounded-sm hover:bg-[#2A2A2A] mt-[-3px]"
                            title="Copy to clipboard"
                        >
                            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                                <path d="M4 4.5H4.5V4V0.666667C4.5 0.622464 4.51756 0.580072 4.54882 0.548816C4.58007 0.517559 4.62246 0.5 4.66667 0.5H15.3333C15.3775 0.5 15.4199 0.51756 15.4512 0.548815C15.4824 0.58007 15.5 0.622463 15.5 0.666667V11.3333C15.5 11.3775 15.4824 11.4199 15.4512 11.4512C15.4199 11.4824 15.3775 11.5 15.3333 11.5H12H11.5V12V15.3333C11.5 15.3775 11.4824 15.4199 11.4512 15.4512C11.4199 15.4824 11.3775 15.5 11.3333 15.5H0.666667C0.622463 15.5 0.58007 15.4824 0.548815 15.4512C0.51756 15.4199 0.5 15.3775 0.5 15.3333V4.66667C0.5 4.62246 0.517559 4.58007 0.548816 4.54882C0.580072 4.51756 0.622464 4.5 0.666667 4.5H4ZM10.6667 15.1667H11.1667V14.6667V5.33333V4.83333H10.6667H1.33333H0.833333V5.33333V14.6667V15.1667H1.33333H10.6667ZM14.6667 11.1667H15.1667V10.6667V1.33333V0.833333H14.6667H5.33333H4.83333V1.33333V4V4.5H5.33333H11.3333C11.3775 4.5 11.4199 4.51756 11.4512 4.54882C11.4824 4.58007 11.5 4.62246 11.5 4.66667V10.6667V11.1667H12H14.6667Z" fill="currentColor" stroke="currentColor"/>
                            </svg>
                        </button>
                    </div>
                ))}
            </div>
        </div>
    );
};