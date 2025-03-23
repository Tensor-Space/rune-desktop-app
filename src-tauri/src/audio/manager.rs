use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::core::error::AudioError;

const RECORDINGS_PATH_TYPE: &str = "app_data";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub id: String,
}

pub struct AudioManager {}

impl AudioManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn list_input_devices(&self) -> Result<Vec<AudioDevice>, AudioError> {
        let host = cpal::default_host();
        Ok(host
            .input_devices()
            .map_err(|e| AudioError::Device(e.to_string()))?
            .filter_map(|device| {
                device.name().ok().map(|name| AudioDevice {
                    name: name.clone(),
                    id: name,
                })
            })
            .collect())
    }

    pub fn get_default_input_device(&self) -> Result<Option<AudioDevice>, AudioError> {
        let host = cpal::default_host();
        if let Some(device) = host.default_input_device() {
            if let Ok(name) = device.name() {
                return Ok(Some(AudioDevice {
                    name: name.clone(),
                    id: name,
                }));
            }
        }
        Ok(None)
    }

    // Recording path management methods
    pub fn get_recordings_path(&self, app_handle: &AppHandle) -> PathBuf {
        let recordings_path = match RECORDINGS_PATH_TYPE {
            "local" => {
                let current_dir = std::env::current_dir().expect("Failed to get current directory");
                current_dir.join("recordings")
            }
            _ => {
                let app_dir = app_handle
                    .path()
                    .app_data_dir()
                    .expect("Failed to get app data directory");

                app_dir.join("recordings")
            }
        };

        fs::create_dir_all(&recordings_path).expect("Failed to create recordings directory");

        recordings_path
    }

    pub fn ensure_recordings_directory(
        &self,
        app_handle: &AppHandle,
    ) -> Result<PathBuf, AudioError> {
        let path = self.get_recordings_path(app_handle);

        if !path.exists() {
            fs::create_dir_all(&path).map_err(|e| {
                AudioError::Recording(format!("Failed to create recordings directory: {}", e))
            })?;
        }

        Ok(path)
    }

    pub fn list_recordings(&self, app_handle: &AppHandle) -> Result<Vec<String>, AudioError> {
        let recordings_path = self.ensure_recordings_directory(app_handle)?;

        let entries = fs::read_dir(&recordings_path).map_err(|e| {
            AudioError::Recording(format!("Failed to read recordings directory: {}", e))
        })?;

        let recordings: Vec<String> = entries
            .filter_map(Result::ok)
            .filter(|entry| {
                if let Some(ext) = entry.path().extension() {
                    ext == "wav" || ext == "mp3" || ext == "m4a"
                } else {
                    false
                }
            })
            .filter_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
            .collect();

        Ok(recordings)
    }

    pub fn get_recording_path(
        &self,
        app_handle: &AppHandle,
        filename: &str,
    ) -> Result<PathBuf, AudioError> {
        let recordings_path = self.ensure_recordings_directory(app_handle)?;
        Ok(recordings_path.join(filename))
    }

    pub fn delete_recording(
        &self,
        app_handle: &AppHandle,
        filename: &str,
    ) -> Result<(), AudioError> {
        let file_path = self.get_recording_path(app_handle, filename)?;

        if file_path.exists() {
            fs::remove_file(&file_path)
                .map_err(|e| AudioError::Recording(format!("Failed to delete recording: {}", e)))?;
            Ok(())
        } else {
            Err(AudioError::Recording(format!(
                "Recording file not found: {}",
                filename
            )))
        }
    }
}
