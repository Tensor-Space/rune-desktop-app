use crate::events::types::UIEvent;
use tauri::{AppHandle, Emitter};

pub struct EventEmitter {
    app_handle: AppHandle,
}

impl EventEmitter {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub fn emit_event(&self, event: UIEvent) {
        // Determine the event name based on type
        let event_name = match &event {
            UIEvent::RecordingStarted => "recording-started",
            UIEvent::RecordingStopped => "recording-stopped",
            UIEvent::ProcessingStarted => "processing-started",
            UIEvent::ProcessingCompleted => "processing-completed",
            UIEvent::TranscriptionInterim { .. } => "transcription-interim",
            UIEvent::TranscriptionFinal { .. } => "transcription-final",
            UIEvent::AudioLevels { .. } => "audio-levels",
            UIEvent::TextProcessed { .. } => "text-processed",
            UIEvent::Error { .. } => "error",
        };

        // Emit the event to all windows
        if let Err(e) = self.app_handle.emit(event_name, event) {
            log::error!("Failed to emit event {}: {}", event_name, e);
        }
    }

    // Convenience methods for common events
    pub fn emit_error(&self, code: &str, message: &str) {
        self.emit_event(UIEvent::Error {
            code: code.to_string(),
            message: message.to_string(),
        });
    }

    pub fn emit_recording_started(&self) {
        self.emit_event(UIEvent::RecordingStarted);
    }

    pub fn emit_recording_stopped(&self) {
        self.emit_event(UIEvent::RecordingStopped);
    }

    pub fn emit_processing_started(&self) {
        self.emit_event(UIEvent::ProcessingStarted);
    }

    pub fn emit_processing_completed(&self) {
        self.emit_event(UIEvent::ProcessingCompleted);
    }

    pub fn emit_transcription(&self, text: &str, is_final: bool) {
        if is_final {
            self.emit_event(UIEvent::TranscriptionFinal {
                text: text.to_string(),
            });
        } else {
            self.emit_event(UIEvent::TranscriptionInterim {
                text: text.to_string(),
            });
        }
    }

    pub fn emit_audio_levels(&self, levels: Vec<f32>) {
        self.emit_event(UIEvent::AudioLevels { levels });
    }

    pub fn emit_text_processed(&self, text: &str) {
        self.emit_event(UIEvent::TextProcessed {
            text: text.to_string(),
        });
    }
}
