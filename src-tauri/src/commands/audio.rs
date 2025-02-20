use std::{path::PathBuf, sync::Arc};
use tauri::{command, AppHandle, State};

use crate::{
    audio::{devices::AudioDevices, AudioDevice},
    core::{app::AppState, utils::audio::get_recordings_path},
};

#[command]
pub async fn start_recording(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .audio
        .recorder
        .start_recording(&app_handle)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn stop_recording(
    state: State<'_, Arc<AppState>>,
    output_path: PathBuf,
) -> Result<(), String> {
    state
        .audio
        .recorder
        .stop_recording(output_path)
        .await
        .map_err(|e| e.to_string())
}

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
pub async fn transcribe(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let audio_path = get_recordings_path(&app_handle).join("rune_recording.wav");

    if !audio_path.exists() {
        return Err("No recording found to transcribe".to_string());
    }

    let mut transcriber = state
        .audio
        .transcriber
        .lock()
        .map_err(|_| "Failed to acquire transcriber lock".to_string())?;

    let result = transcriber
        .transcribe(audio_path.clone())
        .map_err(|e| e.to_string())?;

    // if let Err(e) = std::fs::remove_file(&audio_path) {
    //     log::warn!("Failed to remove temporary audio file: {}", e);
    // }

    Ok(result.join(" "))
}
