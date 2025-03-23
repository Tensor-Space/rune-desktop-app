use std::sync::Arc;
use tauri::{command, AppHandle, Manager, State};

use crate::{
    audio::{manager::AudioManager, AudioDevice},
    core::app::AppState,
    events::types::RecordingCommand,
};

#[command]
pub async fn get_devices(app_handle: AppHandle) -> Result<Vec<AudioDevice>, String> {
    let audio_manager = app_handle.state::<AudioManager>();
    audio_manager
        .list_input_devices()
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_default_device(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<AudioDevice>, String> {
    let settings = state.settings.read();
    let audio_manager = app_handle.state::<AudioManager>();

    if let Some(device_id) = &settings.audio.default_device {
        let devices = audio_manager
            .list_input_devices()
            .map_err(|e| e.to_string())?;
        devices
            .into_iter()
            .find(|d| &d.id == device_id)
            .map_or(Ok(None), |device| Ok(Some(device)))
    } else {
        audio_manager
            .get_default_input_device()
            .map_err(|e| e.to_string())
    }
}

#[command]
pub async fn set_default_device(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    device_id: String,
) -> Result<(), String> {
    let audio_manager = app_handle.state::<AudioManager>();
    let devices = audio_manager
        .list_input_devices()
        .map_err(|e| e.to_string())?;

    if !devices.iter().any(|d| d.id == device_id) {
        return Err(format!("Device with ID '{}' not found", device_id));
    }

    let mut settings = state.settings.write();
    settings.audio.default_device = Some(device_id.clone());
    settings
        .save(&app_handle)
        .map_err(|e| format!("Failed to persist settings: {}", e))?;

    let tx_clone = {
        let guard = tauri::async_runtime::block_on(async { state.recording_tx.lock().await });
        guard.clone()
    };

    if let Some(tx) = tx_clone {
        std::thread::spawn(move || {
            tauri::async_runtime::block_on(async {
                if let Err(e) = tx.send(RecordingCommand::SetDevice(device_id)).await {
                    log::error!("Failed to send device change command: {}", e);
                }
            });
        });
    }

    Ok(())
}

#[command]
pub async fn start_recording(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let guard = state.recording_tx.lock().await;
    let tx = match guard.as_ref() {
        Some(tx) => tx,
        None => return Err("Recording service not initialized".to_string()),
    };

    tx.send(RecordingCommand::Start)
        .await
        .map_err(|e| format!("Failed to send start command: {}", e))
}

#[command]
pub async fn stop_recording(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let guard = state.recording_tx.lock().await;
    let tx = match guard.as_ref() {
        Some(tx) => tx,
        None => return Err("Recording service not initialized".to_string()),
    };

    tx.send(RecordingCommand::Stop)
        .await
        .map_err(|e| format!("Failed to send stop command: {}", e))
}

#[command]
pub async fn list_recordings(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let audio_manager = app_handle.state::<AudioManager>();
    audio_manager
        .list_recordings(&app_handle)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn delete_recording(filename: String, app_handle: AppHandle) -> Result<(), String> {
    let audio_manager = app_handle.state::<AudioManager>();
    audio_manager
        .delete_recording(&app_handle, &filename)
        .map_err(|e| e.to_string())
}
