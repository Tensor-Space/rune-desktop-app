use rune_whisper_local::{Whisper as WhisperModel, WhisperConfig};
use std::path::PathBuf;
use tauri::AppHandle;

use crate::core::error::AudioError;

pub struct TextTranscriptionService {
    model: Option<WhisperModel>,
}

impl TextTranscriptionService {
    pub fn new(
        model_dir: Option<PathBuf>,
        _app_handle: Option<AppHandle>,
    ) -> Result<Self, AudioError> {
        let model = if let Some(dir) = model_dir {
            let config = WhisperConfig::new(Some(dir));
            Some(WhisperModel::new(config).map_err(|e| AudioError::Transcription(e.to_string()))?)
        } else {
            None
        };

        Ok(Self { model })
    }

    pub fn transcribe(&mut self, audio_path: PathBuf) -> Result<Vec<String>, AudioError> {
        let model = self.model.as_mut().ok_or_else(|| {
            AudioError::Transcription("No model loaded for transcription".to_string())
        })?;

        let transcription_result = model
            .transcribe(audio_path.clone())
            .map_err(|e| AudioError::Transcription(format!("Transcription failed: {}", e)))?;

        Ok(transcription_result)
    }
}
