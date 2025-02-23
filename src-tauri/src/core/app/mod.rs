mod state;

use crate::{
    commands,
    core::{config::Settings, error::AppError, system::window_styler::WindowStyler},
    handlers::recording_pipeline_handler::RecordingPipeline,
};
pub use state::AppState;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;
use tokio::runtime::Runtime;

use super::system::shortcut_manager::ShortcutManager;

const SETTINGS_FILE: &str = "settings.json";

pub struct App {
    state: Arc<AppState>,
}

impl App {
    pub fn new() -> Result<Self, AppError> {
        let settings = Settings::default();

        let state = Arc::new(AppState::new(settings));

        Ok(Self { state })
    }

    pub fn run(self) -> Result<(), AppError> {
        let state = self.state.clone();

        let builder = tauri::Builder::default()
            .manage(state)
            .plugin(tauri_plugin_store::Builder::default().build())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
                println!("App already running, skipping creation of new instance");
            }))
            .invoke_handler(tauri::generate_handler![
                // Audio commands
                commands::audio_commands::get_devices,
                commands::audio_commands::set_default_device,
                commands::audio_commands::transcribe,
                commands::audio_commands::get_default_device,
                // System commands
                commands::system_commands::check_accessibility_permissions,
                commands::system_commands::request_accessibility_permissions,
                commands::system_commands::set_window_visibility,
                commands::system_commands::get_settings,
                commands::system_commands::update_shortcuts,
            ])
            .setup(move |app| {
                #[cfg(desktop)]
                {
                    use tauri_plugin_autostart::MacosLauncher;
                    use tauri_plugin_autostart::ManagerExt;

                    app.handle()
                        .plugin(tauri_plugin_autostart::init(
                            MacosLauncher::LaunchAgent,
                            Some(vec!["--flag1", "--flag2"]),
                        ))
                        .unwrap();

                    let autostart_manager = app.autolaunch();
                    let _ = autostart_manager.enable();
                    println!(
                        "registered for autostart? {}",
                        autostart_manager.is_enabled().unwrap()
                    );
                    let _ = autostart_manager.disable();
                }

                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                self.setup_app(app)?;

                Ok(())
            });

        builder
            .run(tauri::generate_context!())
            .map_err(|e| e.into())
    }

    fn setup_app(&self, app: &tauri::App) -> Result<(), AppError> {
        let store = app
            .store(SETTINGS_FILE)
            .map_err(|e| AppError::Config(format!("Failed to create store: {}", e).into()))?;

        let settings = if let Some(stored_settings) = store.get("settings") {
            serde_json::from_value(stored_settings)
                .map_err(|e| AppError::Config(format!("Failed to parse settings: {}", e).into()))?
        } else {
            let default_settings = Settings::default();
            store.set("settings", serde_json::json!(default_settings.clone()));
            store.save().map_err(|e| {
                AppError::Config(format!("Failed to persist settings: {}", e).into())
            })?;
            default_settings
        };

        {
            let mut state_settings = self.state.settings.write();
            *state_settings = settings.clone();
        }

        let settings_win_builder =
            WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings".into()))
                .title("Rune Settings")
                .visible(false)
                .inner_size(800.0, 1000.0);

        let _settings_window = settings_win_builder.build()?;

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
        WindowStyler::setup_window_style(main_window)?;
        let shortcut_manager = ShortcutManager::new(Arc::clone(&self.state));
        shortcut_manager.register_shortcuts(app)?;
        let start_recording_item = MenuItem::with_id(
            app,
            "start_recording",
            "Start Recording",
            true,
            None::<&str>,
        )
        .map_err(|e| {
            AppError::Config(format!("Failed to create settings menu item: {}", e).into())
        })?;
        let stop_recording_item =
            MenuItem::with_id(app, "stop_recording", "Stop Recording", true, None::<&str>)
                .map_err(|e| {
                    AppError::Config(format!("Failed to create settings menu item: {}", e).into())
                })?;
        let settings_item = MenuItem::with_id(app, "settings", "Rune Settings", true, None::<&str>)
            .map_err(|e| {
                AppError::Config(format!("Failed to create settings menu item: {}", e).into())
            })?;
        let quit_item =
            MenuItem::with_id(app, "quit", "Quit App", true, None::<&str>).map_err(|e| {
                AppError::Config(format!("Failed to create quit menu item: {}", e).into())
            })?;

        let tray_menu = Menu::with_items(
            app,
            &[
                &start_recording_item,
                &stop_recording_item,
                &settings_item,
                &quit_item,
            ],
        )
        .map_err(|e| AppError::Config(format!("Failed to create tray menu: {}", e).into()))?;
        let recording_pipeline =
            RecordingPipeline::new(Arc::clone(&self.state), app.app_handle().clone());
        let rt = Runtime::new().unwrap();

        let _tray = TrayIconBuilder::new()
            .icon(app.default_window_icon().unwrap().clone())
            .menu(&tray_menu)
            .on_menu_event(move |app, event| match event.id.as_ref() {
                "start_recording" => {
                    if let Ok(_) = rt.block_on(recording_pipeline.start()) {
                        if let Some(tray_handle) = app.tray_by_id("start_recording") {
                            let _ = tray_handle.set_visible(false);
                        }

                        if let Some(tray_handle) = app.tray_by_id("stop_recording") {
                            let _ = tray_handle.set_visible(true);
                        }
                    }
                }
                "stop_recording" => {
                    rt.block_on(recording_pipeline.stop());
                    if let Some(tray_handle) = app.tray_by_id("start_recording") {
                        let _ = tray_handle.set_visible(true);
                    }

                    if let Some(tray_handle) = app.tray_by_id("stop_recording") {
                        let _ = tray_handle.set_visible(false);
                    }
                }
                "settings" => {
                    if let Some(settings_window) = app.get_webview_window("settings") {
                        let _ = settings_window.show();
                        let _ = settings_window.set_focus();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
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
}
