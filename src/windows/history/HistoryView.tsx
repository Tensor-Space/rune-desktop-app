import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import { format, parseISO } from "date-fns";

type TranscriptionHistory = {
    id: number;
    timestamp: string;
    audio_path: string;
    text: string;
};

export const HistoryView = () => {
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [transcriptions, setTranscriptions] = useState<TranscriptionHistory[]>([]);

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
        }, 5000);

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

    const handlePlay = (audioPath: string) => {
        console.log(`Playing recording: ${audioPath}`);
        // In future: implement actual audio playback using tauri-plugin-opener
    };

    const formatTimestamp = (timestamp: string): string => {
        try {
            return format(parseISO(timestamp), "MMM d, yyyy HH:mm");
        } catch (e) {
            return timestamp;
        }
    };
    
    // Helper function to extract filename from path
    const getFileName = (path: string): string => {
        const parts = path.split(/[\/\\]/);
        return parts[parts.length - 1];
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
        <div 
            className="flex flex-col h-screen bg-background text-foreground overflow-hidden"
            data-tauri-drag-region
        >
            <div className="flex flex-col h-full p-6 container mx-auto">
                {/* Header */}
                <div className="mb-10 text-center" data-tauri-drag-region>
                    <h1 className="text-2xl font-medium">Recording History</h1>
                </div>

                {/* Transcriptions List */}
                <div className="flex-1 overflow-auto">
                    {sortedTranscriptions.length === 0 ? (
                        <div className="text-center text-muted-foreground p-6">
                            No recordings yet. Start recording to see your history.
                        </div>
                    ) : (
                        <div className="space-y-4">
                            {sortedTranscriptions.map((transcription) => (
                                <div 
                                    key={transcription.id} 
                                    className="bg-card rounded-lg p-4 flex flex-col shadow-sm border border-border"
                                >
                                    <div className="flex justify-between items-center mb-2">
                                        <h3 className="font-medium">{getFileName(transcription.audio_path)}</h3>
                                        <div className="text-sm text-muted-foreground">
                                            {formatTimestamp(transcription.timestamp)}
                                        </div>
                                    </div>
                                    
                                    <div className="text-sm mb-3 max-h-20 overflow-y-auto">
                                        {transcription.text}
                                    </div>
                                    
                                    <div className="flex justify-end gap-2 mt-auto">
                                        <Button 
                                            variant="secondary" 
                                            size="sm"
                                            onClick={() => handlePlay(transcription.audio_path)}
                                        >
                                            Play Audio
                                        </Button>
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
};