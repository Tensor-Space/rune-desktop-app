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
use tauri::{App as TauriApp, Manager};
use tauri::{Listener, LogicalPosition};
use tauri_plugin_store::StoreExt;
use tauri_plugin_updater::UpdaterExt;

const SETTINGS_FILE: &str = "settings.json";

pub fn setup_app(app: &TauriApp, state: Arc<AppState>) -> Result<(), AppError> {
    setup_settings(app, &state)?;

    state.init_state_machine(app.app_handle().clone());

    initialize_audio_pipeline(app, &state)?;

    configure_windows(app)?;

    setup_shortcuts(app, &state)?;

    setup_system_tray(app, state.clone())?;

    setup_event_listeners(app, state.clone())?;

    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        update(handle).await.unwrap();
    });

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

fn configure_windows(app: &TauriApp) -> Result<(), AppError> {
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

    if let Some(settings_window) = app.get_webview_window("settings") {
        let settings_window_clone = settings_window.clone();
        settings_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = settings_window_clone.hide();
            }
        });
    } else {
        error!("Window not found: settings");
    }

    if let Some(history_window) = app.get_webview_window("history") {
        let history_window_clone = history_window.clone();
        history_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = history_window_clone.hide();
            }
        });
    } else {
        error!("Window not found: history");
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

fn setup_event_listeners(app: &TauriApp, state: Arc<AppState>) -> Result<(), AppError> {
    let state_clone = state.clone();
    app.listen("cancel-recording", move |_| {
        log::info!("Cancel event received");
        state_clone.cancel_current_operation();
    });

    let state_clone = state.clone();
    let app_handle = app.app_handle().clone();
    app.listen("tauri://close-requested", move |_event| {
        log::info!("Close requested");

        state_clone.cancel_current_operation();

        std::thread::sleep(std::time::Duration::from_millis(200));

        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.close();
        }
    });

    Ok(())
}

async fn update(app: tauri::AppHandle) -> tauri_plugin_updater::Result<()> {
    if let Some(update) = app.updater()?.check().await? {
        let mut downloaded = 0;

        // alternatively we could also call update.download() and update.install() separately
        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    println!("downloaded {downloaded} from {content_length:?}");
                },
                || {
                    println!("download finished");
                },
            )
            .await?;

        println!("update installed");
        app.restart();
    }

    Ok(())
}
