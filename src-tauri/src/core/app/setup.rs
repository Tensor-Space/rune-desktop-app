use super::state::AppState;
use crate::controllers::audio_pipleine_controller::AudioPipelineController;
use crate::core::error::AppError;
use crate::core::system::window_manager::WindowManager;
use crate::core::{
    config::Settings,
    system::{shortcut_manager::ShortcutManager, system_tray_manager::SystemTrayManager},
};
use log::error;
use std::sync::Arc;
use tauri::LogicalPosition;
use tauri::{App as TauriApp, Manager};
use tauri_plugin_store::StoreExt;

const SETTINGS_FILE: &str = "settings.json";

pub fn setup_app(app: &TauriApp, state: Arc<AppState>) -> Result<(), AppError> {
    setup_settings(app, &state)?;

    initialize_audio_pipeline(app, &state)?;

    configure_main_window(app)?;

    setup_shortcuts(app, &state)?;

    setup_system_tray(app, state)?;

    Ok(())
}

fn setup_settings(app: &TauriApp, state: &Arc<AppState>) -> Result<(), AppError> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|e| AppError::Config(format!("Failed to create store: {}", e).into()))?;

    let settings = if let Some(stored_settings) = store.get("settings") {
        serde_json::from_value(stored_settings)
            .map_err(|e| AppError::Config(format!("Failed to parse settings: {}", e).into()))?
    } else {
        let default_settings = Settings::default();
        store.set("settings", serde_json::json!(default_settings.clone()));
        store
            .save()
            .map_err(|e| AppError::Config(format!("Failed to persist settings: {}", e).into()))?;
        default_settings
    };

    {
        let mut state_settings = state.settings.write();
        *state_settings = settings.clone();
    }

    Ok(())
}

fn configure_main_window(app: &TauriApp) -> Result<(), AppError> {
    let monitor = app.primary_monitor().unwrap().unwrap();
    let scale_factor = monitor.scale_factor();
    let monitor_size = monitor.size();

    let x_pos = ((monitor_size.width as f64 / scale_factor) / 2.0) - (150.0 / 2.0);
    let y_pos = (monitor_size.height as f64 / scale_factor) - (40.0 + 80.0);

    if let Some(main_window) = app.get_webview_window("main") {
        main_window.set_position(LogicalPosition::new(x_pos, y_pos))?;

        WindowManager::remove_titlebar_and_traffic_lights(main_window)?;
    } else {
        error!("Window not found: main")
    }

    Ok(())
}

fn setup_shortcuts(app: &TauriApp, state: &Arc<AppState>) -> Result<(), AppError> {
    let shortcut_manager = ShortcutManager::new(Arc::clone(state));
    shortcut_manager.register_shortcuts(app)?;

    Ok(())
}

fn setup_system_tray(app: &TauriApp, state: Arc<AppState>) -> Result<(), AppError> {
    let tray_manager = SystemTrayManager::new(state, app.app_handle().clone())?;
    tray_manager.setup()?;

    Ok(())
}

fn initialize_audio_pipeline(app: &TauriApp, state: &Arc<AppState>) -> Result<(), AppError> {
    let app_handle = app.app_handle();

    let audio_pipeline = Arc::new(AudioPipelineController::new(
        Arc::clone(state),
        app_handle.clone(),
    ));

    *state.audio_pipeline.lock() = Some(audio_pipeline);

    Ok(())
}
