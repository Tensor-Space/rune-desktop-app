use std::sync::Arc;
use tauri::{command, AppHandle, State};
use tauri_plugin_store::StoreExt;

use crate::{
    audio::{devices::AudioDevices, AudioDevice},
    core::app::AppState,
};

#[command]
pub async fn get_devices() -> Result<Vec<AudioDevice>, String> {
    AudioDevices::list().map_err(|e| e.to_string())
}

#[command]
pub async fn get_default_device(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<AudioDevice>, String> {
    let settings = state.settings.read();
    if let Some(device_id) = &settings.audio.default_device {
        AudioDevices::list()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|d| &d.id == device_id)
            .map_or(Ok(None), |device| Ok(Some(device)))
    } else {
        Ok(None)
    }
}

#[command]
pub async fn set_default_device(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    device_id: String,
) -> Result<(), String> {
    let devices = AudioDevices::list().map_err(|e| e.to_string())?;

    if !devices.iter().any(|d| d.id == device_id) {
        return Err(format!("Device with id {} not found", device_id));
    }

    let mut settings = state.settings.write();
    settings.audio.default_device = Some(device_id);
    settings
        .save(&app_handle)
        .map_err(|e| format!("Failed to persist settings: {}", e))?;

    Ok(())
}

#[command]
pub async fn get_transcription_history(
    app_handle: AppHandle
) -> Result<Vec<serde_json::Value>, String> {
    println!("Getting transcription history directly from store...");
    
    // Access the store directly without initializing the transcriber
    let store = match app_handle.store("transcription_history.json") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to access store: {}", e);
            // If we can't access the store, initialize an empty one
            app_handle.store("transcription_history.json")
                .map_err(|e| format!("Failed to create store: {}", e))?
        }
    };
    
    let history: Vec<serde_json::Value> = match store.get("transcriptions") {
        Some(value) => match value {
            serde_json::Value::Array(arr) => arr.clone(),
            _ => {
                println!("Found malformed transcriptions data, returning empty array");
                Vec::new() 
            }
        },
        None => {
            println!("No transcriptions found in store, returning empty array");
            Vec::new()
        }
    };
    
    println!("Found {} transcriptions", history.len());
    Ok(history)
}