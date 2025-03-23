use std::sync::Arc;
use tauri::{command, AppHandle, State};
use tauri_plugin_store::StoreExt;

use crate::{
    core::{app::AppState, state_machine::AppCommand},
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
    log::info!("Getting transcription history directly from store...");

    let store = match app_handle.store("transcription_history.json") {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to access store: {}", e);
            app_handle
                .store("transcription_history.json")
                .map_err(|e| format!("Failed to create store: {}", e))?
        }
    };

    let history: Vec<serde_json::Value> = match store.get("transcriptions") {
        Some(value) => match value {
            serde_json::Value::Array(arr) => arr.clone(),
            _ => {
                log::info!("Found malformed transcriptions data, returning empty array");
                Vec::new()
            }
        },
        None => {
            log::info!("No transcriptions found in store, returning empty array");
            Vec::new()
        }
    };

    log::info!("Found {} transcriptions", history.len());
    Ok(history)
}

#[command]
pub fn start_recording(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    log::info!("Start recording command received");

    if let Some(machine) = &*state.state_machine.lock() {
        machine.send_command(AppCommand::StartRecording);
    }

    Ok(())
}

#[command]
pub fn stop_recording(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    log::info!("Stop recording command received");

    if let Some(machine) = &*state.state_machine.lock() {
        machine.send_command(AppCommand::StopRecording);
    }

    Ok(())
}

#[command]
pub async fn cancel_recording(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.cancel_current_operation();

    // Update the system tray menu after cancellation
    if let Some(tray) = app_handle.tray_by_id("tray") {
        if let Ok(new_menu) =
            crate::core::system::system_tray_manager::SystemTrayManager::build_tray_menu_static(
                &app_handle,
                true,
                false,
            )
        {
            if let Err(e) = tray.set_menu(Some(new_menu)) {
                log::warn!("Failed to update tray menu after cancellation: {}", e);
            }
        } else {
            log::warn!("Failed to build new tray menu after cancellation");
        }
    }
    Ok(())
}
