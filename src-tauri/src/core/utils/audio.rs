use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const RECORDINGS_PATH_TYPE: &str = "local";

pub fn get_recordings_path(app_handle: &AppHandle) -> PathBuf {
    match RECORDINGS_PATH_TYPE {
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
    }
}
