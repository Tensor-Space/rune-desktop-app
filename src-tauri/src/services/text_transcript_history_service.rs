use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;

use crate::core::error::AudioError;

const HISTORY_FILE: &str = "transcription_history.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TranscriptionHistory {
    pub id: u32,
    pub timestamp: String,
    pub text: String,
}

pub struct TextTranscriptHistoryService;

impl TextTranscriptHistoryService {
    pub fn save_processed_text(app_handle: &AppHandle, text: &str) -> Result<(), AudioError> {
        let new_entry = TranscriptionHistory {
            id: Self::generate_id(),
            timestamp: Utc::now().to_rfc3339(),
            text: text.to_string(),
        };

        let store = app_handle
            .store(HISTORY_FILE)
            .map_err(|e| AudioError::Transcription(format!("Failed to access store: {}", e)))?;

        let mut history: Vec<TranscriptionHistory> = store
            .get("transcriptions")
            .map(|value| serde_json::from_value(value).unwrap_or_else(|_| Vec::new()))
            .unwrap_or_else(|| Vec::new());

        history.push(new_entry.clone());

        store.set("transcriptions", serde_json::json!(history));

        store
            .save()
            .map_err(|e| AudioError::Transcription(format!("Failed to save history: {}", e)))?;

        match app_handle.emit("transcription-added", serde_json::json!(new_entry)) {
            Ok(_) => log::info!("Successfully emitted transcription-added event"),
            Err(e) => log::error!("Failed to emit transcription-added event: {}", e),
        }

        if let Some(history_window) = app_handle.get_webview_window("history") {
            if let Ok(true) = history_window.is_visible() {}
        }

        Ok(())
    }

    pub fn get_transcription_history(
        app_handle: &AppHandle,
    ) -> Result<Vec<TranscriptionHistory>, AudioError> {
        let store = app_handle
            .store(HISTORY_FILE)
            .map_err(|e| AudioError::Transcription(format!("Failed to access store: {}", e)))?;

        if !store.has("transcriptions") {
            store.set("transcriptions", serde_json::json!([]));
            store.save().map_err(|e| {
                AudioError::Transcription(format!("Failed to initialize store: {}", e))
            })?;
        }

        let history: Vec<TranscriptionHistory> = store
            .get("transcriptions")
            .map(|value| serde_json::from_value(value).unwrap_or_else(|_| Vec::new()))
            .unwrap_or_else(|| Vec::new());

        Ok(history)
    }

    fn generate_id() -> u32 {
        rand::thread_rng().gen()
    }
}
