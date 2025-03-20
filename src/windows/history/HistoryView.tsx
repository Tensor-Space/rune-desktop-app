import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
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

    useEffect(() => {
        // Prevent scrolling on body and html
        document.body.style.overflow = "hidden";
        document.documentElement.style.overflow = "hidden";

        // Load transcriptions from the backend
        const loadTranscriptions = async () => {
            try {
                console.log("Fetching transcription history...");
                const result = await invoke<TranscriptionHistory[]>("get_transcription_history");
                console.log("Received transcriptions:", result);
                setTranscriptions(result);
            } catch (error) {
                console.error("Failed to load transcriptions:", error);
                setError(`Failed to load transcription history: ${error}`);
            } finally {
                setIsLoading(false);
            }
        };

        loadTranscriptions();

        return () => {
            // Cleanup
            document.body.style.overflow = "";
            document.documentElement.style.overflow = "";
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
                    {transcriptions.length === 0 ? (
                        <div className="text-center text-muted-foreground p-6">
                            No recordings yet. Start recording to see your history.
                        </div>
                    ) : (
                        <div className="space-y-4">
                            {transcriptions.map((transcription) => (
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