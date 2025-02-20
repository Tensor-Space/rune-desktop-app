use crate::core::error::AudioError;
use rune_whisper::{Whisper as WhisperModel, WhisperConfig};

pub struct AudioTranscriber {
    model: WhisperModel,
}

impl AudioTranscriber {
    pub fn new() -> Result<Self, AudioError> {
        let config = WhisperConfig::default();
        Ok(Self {
            model: WhisperModel::new(config)
                .map_err(|e| AudioError::Transcription(e.to_string()))?,
        })
    }

    pub fn transcribe(
        &mut self,
        audio_path: std::path::PathBuf,
    ) -> Result<Vec<String>, AudioError> {
        self.model
            .transcribe(audio_path)
            .map_err(|e| AudioError::Transcription(format!("Transcription failed: {}", e)))
    }
}
