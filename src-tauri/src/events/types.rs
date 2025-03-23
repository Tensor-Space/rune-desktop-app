use serde::{Deserialize, Serialize};

// Events from Rust to UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UIEvent {
    // Recording status events
    RecordingStarted,
    RecordingStopped,
    ProcessingStarted,
    ProcessingCompleted,

    // Transcription events
    TranscriptionInterim { text: String },
    TranscriptionFinal { text: String },

    // Audio events
    AudioLevels { levels: Vec<f32> },

    // Result events
    TextProcessed { text: String },

    // Error events
    Error { code: String, message: String },
}

// Events from UI to Rust (outside of commands)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    CancelProcessing,
    RetryTranscription,
    RequestAudioStatus,
}

// Internal events for service communication
#[derive(Debug, Clone)]
pub enum ServiceEvent {
    RecordingStarted,
    RecordingStopped,
    TranscriptionReceived { text: String, is_final: bool },
    AudioLevelsUpdated { levels: Vec<f32> },
    TextProcessed { text: String },
    Error { message: String },
    ProcessingCompleted,
}

// Commands sent to the recording service
#[derive(Debug, Clone)]
pub enum RecordingCommand {
    Start,
    Stop,
    SetDevice(String),
    ProcessText { app_name: String, text: String },
}

impl ServiceEvent {
    const fn ProcessingCompleted() -> Self {
        ServiceEvent::ProcessingCompleted
    }
}
