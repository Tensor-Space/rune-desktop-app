use std::{str::FromStr, sync::Arc};

use crate::core::{app::AppState, config::Settings, system::permission_manager::PermissionManager};
use tauri::{command, AppHandle};
use tauri_plugin_global_shortcut::{Code, Modifiers};

#[command]
pub async fn check_accessibility_permissions() -> Result<bool, String> {
    PermissionManager::check_accessibility_permissions().map_err(|e| e.to_string())
}

#[command]
pub async fn request_accessibility_permissions() -> Result<bool, String> {
    PermissionManager::request_accessibility_permissions().map_err(|e| e.to_string())
}

#[command]
pub async fn check_microphone_permissions() -> Result<bool, String> {
    PermissionManager::check_microphone_permissions().map_err(|e| e.to_string())
}

#[command]
pub async fn request_microphone_permissions() -> Result<bool, String> {
    PermissionManager::request_microphone_permissions().map_err(|e| e.to_string())
}

#[command]
pub async fn set_window_visibility(visible: bool, app_handle: AppHandle) -> Result<(), String> {
    if visible {
        app_handle.show().map_err(|e| e.to_string())?;
    } else {
        app_handle.hide().map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_settings(app_handle: AppHandle) -> Result<Settings, String> {
    Settings::load(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_shortcuts(
    key: String,
    modifier: String,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let _parsed_modifier = Modifiers::from_name(&modifier)
        .ok_or_else(|| format!("Failed to parse shortcut modifier '{}'", modifier))?;
    let _parsed_key = Code::from_str(&key)
        .map_err(|e| format!("Failed to parse shortcut key '{}': {}", key, e))?;

    let mut settings = state.settings.write();
    settings
        .update_shortcuts(&app_handle, key, modifier)
        .map_err(|e| e.to_string())?;
    settings
        .save(&app_handle)
        .map_err(|e| format!("Failed to persist settings: {}", e))?;

    Ok(())
}
