use std::path::PathBuf;
use rand::Rng;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use crate::core::error::AudioError;
use rune_whisper_local::{Whisper as WhisperModel, WhisperConfig};

const HISTORY_FILE: &str = "transcription_history.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TranscriptionHistory {
    pub id: u32,
    pub timestamp: String,
    pub audio_path: String,
    pub text: String,
}

pub struct AudioTranscriber {
    model: Option<WhisperModel>,
    app_handle: Option<AppHandle>,
}

impl AudioTranscriber {
    pub fn new(model_dir: Option<PathBuf>, app_handle: Option<AppHandle>) -> Result<Self, AudioError> {
        // If model_dir is None, create a transcriber without a model (only for history operations)
        let model = if let Some(dir) = model_dir {
            let config = WhisperConfig::new(Some(dir));
            Some(WhisperModel::new(config)
                .map_err(|e| AudioError::Transcription(e.to_string()))?)
        } else {
            None
        };
        
        Ok(Self {
            model,
            app_handle,
        })
    }

    pub fn transcribe(
        &mut self,
        audio_path: PathBuf,
    ) -> Result<Vec<String>, AudioError> {
        // Ensure we have a model before attempting to transcribe
        let model = self.model.as_mut()
            .ok_or_else(|| AudioError::Transcription("No model loaded for transcription".to_string()))?;
        
        let transcription_result = model
            .transcribe(audio_path.clone())
            .map_err(|e| AudioError::Transcription(format!("Transcription failed: {}", e)))?;

        // Save transcription history if we have an app handle
        if let Some(app_handle) = &self.app_handle {
            if let Err(e) = self.save_transcription(app_handle, &audio_path, &transcription_result) {
                eprintln!("Failed to save transcription history: {}", e);
            }
        }

        Ok(transcription_result)
    }

    fn save_transcription(&self, app_handle: &AppHandle, audio_path: &PathBuf, text: &Vec<String>) -> Result<(), AudioError> {
        let new_entry = TranscriptionHistory {
            id: self.generate_id(),
            timestamp: Utc::now().to_rfc3339(),
            audio_path: audio_path.to_string_lossy().to_string(),
            text: text.join(" "), // Convert Vec<String> to a single string
        };

        // Get the store from app_handle
        let store = app_handle
            .store(HISTORY_FILE)
            .map_err(|e| AudioError::Transcription(format!("Failed to access store: {}", e)))?;
        
        // Get existing history or create empty vec
        let mut history: Vec<TranscriptionHistory> = store
            .get("transcriptions")
            .map(|value| serde_json::from_value(value).unwrap_or_else(|_| Vec::new()))
            .unwrap_or_else(|| Vec::new());
        
        // Add new entry
        history.push(new_entry);
        
        // Save back to store
        store.set("transcriptions", serde_json::json!(history));
        
        store.save()
            .map_err(|e| AudioError::Transcription(format!("Failed to save history: {}", e)))?;

        Ok(())
    }

    pub fn get_transcriptions(&self, app_handle: &AppHandle) -> Result<Vec<TranscriptionHistory>, AudioError> {
        let store = app_handle
            .store(HISTORY_FILE)
            .map_err(|e| AudioError::Transcription(format!("Failed to access store: {}", e)))?;
        
        let history: Vec<TranscriptionHistory> = store
            .get("transcriptions")
            .map(|value| serde_json::from_value(value).unwrap_or_else(|_| Vec::new()))
            .unwrap_or_else(|| Vec::new());
        
        Ok(history)
    }

    fn generate_id(&self) -> u32 {
        rand::thread_rng().gen()
    }

    // Static method to get transcription history without needing a model
    pub fn get_transcription_history(app_handle: &AppHandle) -> Result<Vec<TranscriptionHistory>, AudioError> {
        let store = app_handle
            .store(HISTORY_FILE)
            .map_err(|e| AudioError::Transcription(format!("Failed to access store: {}", e)))?;
        
        // Ensure the store has a transcriptions key
        if !store.has("transcriptions") {
            // Initialize an empty array if it doesn't exist
            store.set("transcriptions", serde_json::json!([]));
            store.save()
                .map_err(|e| AudioError::Transcription(format!("Failed to initialize store: {}", e)))?;
        }
        
        let history: Vec<TranscriptionHistory> = store
            .get("transcriptions")
            .map(|value| serde_json::from_value(value).unwrap_or_else(|_| Vec::new()))
            .unwrap_or_else(|| Vec::new());
        
        Ok(history)
    }
}
