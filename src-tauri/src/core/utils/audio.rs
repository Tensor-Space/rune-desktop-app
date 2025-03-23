use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const RECORDINGS_PATH_TYPE: &str = "app_data";

pub fn get_recordings_path(app_handle: &AppHandle) -> PathBuf {
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

            log::info!("App data directory: {:?}", app_dir);
            app_dir.join("recordings")
        }
    };

    fs::create_dir_all(&recordings_path).expect("Failed to create recordings directory");

    recordings_path
}
