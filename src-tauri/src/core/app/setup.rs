use super::state::AppState;
use crate::controllers::audio_pipleine_controller::AudioPipelineController;
use crate::core::error::AppError;
use crate::core::system::window_manager::WindowManager;
use crate::core::{config::Settings, system::shortcut_manager::ShortcutManager};
use std::sync::Arc;
use tauri::WebviewWindow;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    App as TauriApp, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;
use tokio::runtime::Runtime;

const SETTINGS_FILE: &str = "settings.json";

pub fn setup_app(app: &TauriApp, state: Arc<AppState>) -> Result<(), AppError> {
    // Load settings
    setup_settings(app, &state)?;

    // Create windows
    create_settings_window(app)?;
    create_history_window(app)?;
    create_main_window(app)?;

    // Setup shortcuts
    setup_shortcuts(app, &state)?;

    // Setup tray
    setup_tray(app, state)?;

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

fn create_settings_window(app: &TauriApp) -> Result<WebviewWindow, AppError> {
    let settings_win_builder =
        WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings".into()))
            .title("Rune Settings")
            .visible(false)
            .inner_size(800.0, 800.0)
            .hidden_title(true);

    let settings_window = settings_win_builder.build()?;
    // WindowStyler::remove_titlebar(settings_window)?;

    Ok(settings_window)
}

fn create_history_window(app: &TauriApp) -> Result<WebviewWindow, AppError> {
    let history_win_builder =
        WebviewWindowBuilder::new(app, "history", WebviewUrl::App("history".into()))
            .title("Rune History")
            .visible(false)
            .inner_size(800.0, 600.0)
            .hidden_title(true);

    let history_window = history_win_builder.build()?;

    Ok(history_window)
}

fn create_main_window(app: &TauriApp) -> Result<WebviewWindow, AppError> {
    let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("Rune")
        .inner_size(200.0, 60.0)
        .position(
            {
                let monitor = app.primary_monitor().unwrap().unwrap();
                let scale_factor = monitor.scale_factor();
                let monitor_size = monitor.size();

                let logical_width = (monitor_size.width as f64 / scale_factor) - (200.0 + 20.0);
                logical_width
            },
            40.0,
        )
        .visible(false)
        .shadow(false)
        .title_bar_style(tauri::TitleBarStyle::Transparent)
        .decorations(true);

    let main_window = win_builder.build()?;
    WindowManager::remove_titlebar_and_traffic_lights(main_window.clone())?;

    Ok(main_window)
}

fn setup_shortcuts(app: &TauriApp, state: &Arc<AppState>) -> Result<(), AppError> {
    let shortcut_manager = ShortcutManager::new(Arc::clone(state));
    shortcut_manager.register_shortcuts(app)?;

    Ok(())
}

fn setup_tray(app: &TauriApp, state: Arc<AppState>) -> Result<(), AppError> {
    let tray_menu = build_tray_menu(&app.app_handle(), true, false)?;
    let recording_pipeline =
        AudioPipelineController::new(Arc::clone(&state), app.app_handle().clone());
    let rt = Runtime::new().unwrap();

    let _tray = TrayIconBuilder::with_id("tray")
        .icon(load_tray_icon(app)?)
        .menu(&tray_menu)
        .on_menu_event(move |app, event| {
            handle_tray_menu_event(app, &event.id.as_ref(), &recording_pipeline, &rt);
        })
        .build(app)
        .map_err(|e| AppError::Config(format!("Failed to create tray icon: {}", e).into()))?;

    if let Some(tray_handle) = app.tray_by_id("stop_recording") {
        let _ = tray_handle.set_visible(false);
    } else {
        print!("Stop Recording not found")
    }

    Ok(())
}

fn load_tray_icon(app: &TauriApp) -> Result<Image, AppError> {
    Image::from_path(
        app.path()
            .resource_dir()
            .unwrap()
            .join("icons/tray-icon.ico"),
    )
    .map_err(|e| AppError::Config(format!("Failed to load tray icon: {}", e).into()))
}

fn handle_tray_menu_event(
    app: &AppHandle,
    event_id: &str,
    recording_pipeline: &AudioPipelineController,
    rt: &Runtime,
) {
    match event_id {
        "start_recording" => {
            if let Ok(_) = rt.block_on(recording_pipeline.start()) {
                if let Some(tray) = app.tray_by_id("tray") {
                    if let Ok(new_menu) = build_tray_menu(app, false, true) {
                        let _ = tray.set_menu(Some(new_menu.clone()));
                        let _ = app.set_menu(new_menu);
                    }
                }
            }
        }
        "stop_recording" => {
            rt.block_on(recording_pipeline.stop());
            if let Some(tray) = app.tray_by_id("tray") {
                if let Ok(new_menu) = build_tray_menu(app, true, false) {
                    let _ = tray.set_menu(Some(new_menu.clone()));
                    let _ = app.set_menu(new_menu);
                }
            }
        }
        "settings" => {
            if let Some(settings_window) = app.get_webview_window("settings") {
                let _ = settings_window.show();
                let _ = settings_window.set_focus();
            }
        }
        "history" => {
            if let Some(history_window) = app.get_webview_window("history") {
                let _ = history_window.show();
                let _ = history_window.set_focus();
            }
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

pub fn build_tray_menu(
    app: &AppHandle,
    start_enabled: bool,
    stop_enabled: bool,
) -> Result<Menu<tauri::Wry>, AppError> {
    // Create menu items
    let start_recording_item =
        create_menu_item(app, "start_recording", "Start Recording", start_enabled)?;
    let stop_recording_item =
        create_menu_item(app, "stop_recording", "Stop Recording", stop_enabled)?;
    let history_item = create_menu_item(app, "history", "History", true)?;
    let separator = PredefinedMenuItem::separator(app)
        .map_err(|e| AppError::Config(format!("Failed to create separator: {}", e).into()))?;
    let settings_item = create_menu_item(app, "settings", "Rune Settings", true)?;
    let quit_item = create_menu_item(app, "quit", "Quit App", true)?;

    // Assemble menu
    Menu::with_items(
        app,
        &[
            &start_recording_item,
            &stop_recording_item,
            &history_item,
            &separator,
            &settings_item,
            &quit_item,
        ],
    )
    .map_err(|e| AppError::Config(format!("Failed to create menu: {}", e).into()))
}

fn create_menu_item(
    app: &AppHandle,
    id: &str,
    label: &str,
    enabled: bool,
) -> Result<MenuItem<tauri::Wry>, AppError> {
    MenuItem::with_id(app, id, label, enabled, None::<&str>)
        .map_err(|e| AppError::Config(format!("Failed to create menu item '{}': {}", id, e).into()))
}
