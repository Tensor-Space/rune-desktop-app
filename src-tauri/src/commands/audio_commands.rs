use std::sync::Arc;
use tauri::{command, AppHandle, State};
use tauri_plugin_store::StoreExt;

use crate::{
    core::app::AppState,
    services::audio_device_service::{AudioDevice, AudioDeviceService},
};

#[command]
pub async fn get_devices() -> Result<Vec<AudioDevice>, String> {
    let service = AudioDeviceService::new();
    service.list_devices().map_err(|e| e.to_string())
}

#[command]
pub async fn get_default_device(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<AudioDevice>, String> {
    let service = AudioDeviceService::new();
    let settings = state.settings.read();

    if let Some(device_id) = &settings.audio.default_device {
        service
            .find_device_by_id(device_id)
            .map_err(|e| e.to_string())
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
    let service = AudioDeviceService::new();

    let device = service
        .find_device_by_id(&device_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Device with id {} not found", device_id))?;

    let mut settings = state.settings.write();
    settings.audio.default_device = Some(device.id);
    settings
        .save(&app_handle)
        .map_err(|e| format!("Failed to persist settings: {}", e))?;

    Ok(())
}

#[command]
pub async fn get_transcription_history(
    app_handle: AppHandle,
) -> Result<Vec<serde_json::Value>, String> {
    println!("Getting transcription history directly from store...");

    let store = match app_handle.store("transcription_history.json") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to access store: {}", e);
            app_handle
                .store("transcription_history.json")
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
