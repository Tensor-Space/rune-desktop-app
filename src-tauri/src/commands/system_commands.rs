use std::{str::FromStr, sync::Arc};

use crate::core::{
    app::settings::Settings, app::AppState, system::permission_manager::PermissionManager,
    system::window_manager::WindowManager,
};
use tauri::{command, AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, Modifiers};

#[command]
pub async fn check_accessibility_permissions(app_handle: AppHandle) -> Result<bool, String> {
    let permission_manager = app_handle.state::<PermissionManager>();
    permission_manager
        .check_accessibility_permissions()
        .map_err(|e| e.to_string())
}

#[command]
pub async fn request_accessibility_permissions(app_handle: AppHandle) -> Result<bool, String> {
    let permission_manager = app_handle.state::<PermissionManager>();
    permission_manager
        .request_accessibility_permissions()
        .map_err(|e| e.to_string())
}

#[command]
pub async fn check_microphone_permissions(app_handle: AppHandle) -> Result<bool, String> {
    let permission_manager = app_handle.state::<PermissionManager>();
    permission_manager
        .check_microphone_permissions()
        .map_err(|e| e.to_string())
}

#[command]
pub async fn request_microphone_permissions(app_handle: AppHandle) -> Result<bool, String> {
    let permission_manager = app_handle.state::<PermissionManager>();
    permission_manager
        .request_microphone_permissions()
        .map_err(|e| e.to_string())
}

#[command]
pub async fn check_all_permissions(app_handle: AppHandle) -> Result<bool, String> {
    let permission_manager = app_handle.state::<PermissionManager>();
    permission_manager
        .check_all_permissions()
        .map_err(|e| e.to_string())
}

#[command]
pub async fn request_all_permissions(app_handle: AppHandle) -> Result<bool, String> {
    let permission_manager = app_handle.state::<PermissionManager>();
    permission_manager
        .request_all_permissions()
        .map_err(|e| e.to_string())
}

#[command]
pub async fn set_window_visibility(
    window_label: String,
    visible: bool,
    app_handle: AppHandle,
) -> Result<(), String> {
    let window_manager = app_handle.state::<WindowManager>();

    if visible {
        if window_label == "settings" {
            window_manager
                .show_settings_window(&app_handle)
                .map_err(|e| e.to_string())?;
        } else if window_label == "main" {
            window_manager
                .show_main_window(&app_handle)
                .map_err(|e| e.to_string())?;
        } else {
            return Err(format!("Unknown window label: {}", window_label));
        }
    } else {
        if let Some(window) = app_handle.get_webview_window(&window_label) {
            window.hide().map_err(|e| e.to_string())?;
        } else {
            return Err(format!("Window not found: {}", window_label));
        }
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

#[cfg(target_os = "macos")]
#[command]
pub async fn check_screen_recording_permissions(app_handle: AppHandle) -> Result<bool, String> {
    let permission_manager = app_handle.state::<PermissionManager>();
    permission_manager
        .check_screen_recording_permissions()
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
#[command]
pub async fn request_screen_recording_permissions(app_handle: AppHandle) -> Result<bool, String> {
    let permission_manager = app_handle.state::<PermissionManager>();
    permission_manager
        .request_screen_recording_permissions()
        .map_err(|e| e.to_string())
}
