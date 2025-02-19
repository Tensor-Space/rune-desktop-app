use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use tauri_plugin_store::StoreExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioDevice {
    name: String,
    id: String,
}

#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    let host = cpal::default_host();

    let devices = host
        .input_devices()
        .map_err(|e| e.to_string())?
        .filter_map(|device| {
            let name = device.name().ok()?;
            Some(AudioDevice {
                name: name.clone(),
                id: name, // Using name as ID since cpal doesn't provide stable IDs
            })
        })
        .collect::<Vec<_>>();

    Ok(devices)
}

#[tauri::command]
pub async fn set_default_device(
    device_id: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let store = app_handle.store("settings").map_err(|e| e.to_string())?;

    store.set("default_microphone", device_id);

    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_default_device(app_handle: tauri::AppHandle) -> Result<Option<String>, String> {
    let store = app_handle.store("settings").map_err(|e| e.to_string())?;

    let device_id: Option<String> = store
        .get("default_microphone")
        .map(|value| value.as_str().map(String::from))
        .flatten();

    Ok(device_id)
}
