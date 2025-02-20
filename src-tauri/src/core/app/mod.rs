mod state;

use crate::{
    audio::AudioState,
    commands,
    core::{config::Settings, error::AppError, system::window::WindowStyler},
    io::shortcuts::ShortcutManager,
};
pub use state::AppState;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;

const SETTINGS_FILE: &str = "settings.json";

pub struct App {
    state: Arc<AppState>,
}

impl App {
    pub fn new() -> Result<Self, AppError> {
        let settings = Settings::default();
        let audio_state = Arc::new(AudioState::new());

        let state = Arc::new(AppState::new(settings, Arc::clone(&audio_state)));

        Ok(Self { state })
    }

    pub fn run(self) -> Result<(), AppError> {
        let state = self.state.clone();

        let builder = tauri::Builder::default()
            .manage(state)
            .plugin(tauri_plugin_store::Builder::default().build())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_opener::init())
            .invoke_handler(tauri::generate_handler![
                // Audio commands
                commands::audio::start_recording,
                commands::audio::stop_recording,
                commands::audio::get_devices,
                commands::audio::set_default_device,
                commands::audio::transcribe,
                commands::audio::get_default_device,
                // System commands
                commands::system::check_accessibility_permissions,
                commands::system::request_accessibility_permissions,
                commands::system::set_window_visibility,
                commands::system::get_settings,
                commands::system::update_shortcuts,
            ])
            .setup(move |app| {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                self.setup_app(app)?;

                Ok(())
            });

        builder
            .run(tauri::generate_context!())
            .map_err(|e| e.into())
    }

    fn setup_app(&self, app: &tauri::App) -> Result<(), AppError> {
        // Load or create settings store
        let store = app
            .store(SETTINGS_FILE)
            .map_err(|e| AppError::Config(format!("Failed to create store: {}", e).into()))?;

        // Load settings from store or use defaults
        let settings = if let Some(stored_settings) = store.get("settings") {
            serde_json::from_value(stored_settings)
                .map_err(|e| AppError::Config(format!("Failed to parse settings: {}", e).into()))?
        } else {
            // If no settings exist, save defaults
            let default_settings = Settings::default();
            store.set("settings", serde_json::json!(default_settings.clone()));
            store.save().map_err(|e| {
                AppError::Config(format!("Failed to persist settings: {}", e).into())
            })?;
            default_settings
        };

        // Update state with loaded settings
        {
            let mut state_settings = self.state.settings.write();
            *state_settings = settings.clone();
        }

        // Create settings window
        let settings_win_builder =
            WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings".into()))
                .title("Settings")
                .visible(false)
                .inner_size(800.0, 1000.0);

        let _settings_window = settings_win_builder.build()?;

        // Create main window using settings
        let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
            .title("Rune")
            .inner_size(200.0, 60.0)
            .position(
                {
                    let monitor = app.primary_monitor().unwrap().unwrap();
                    let scale_factor = monitor.scale_factor();
                    let monitor_size = monitor.size();

                    // Convert to logical pixels
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
        // Setup shortcuts using loaded settings
        let shortcut_manager = ShortcutManager::new(Arc::clone(&self.state));
        shortcut_manager.register_shortcuts(app)?;

        let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)
            .map_err(|e| {
                AppError::Config(format!("Failed to create settings menu item: {}", e).into())
            })?;
        let quit_item =
            MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).map_err(|e| {
                AppError::Config(format!("Failed to create quit menu item: {}", e).into())
            })?;

        let tray_menu = Menu::with_items(app, &[&settings_item, &quit_item])
            .map_err(|e| AppError::Config(format!("Failed to create tray menu: {}", e).into()))?;

        // Create tray icon
        let _tray = TrayIconBuilder::new()
            .icon(app.default_window_icon().unwrap().clone())
            .menu(&tray_menu)
            .on_menu_event(move |app, event| match event.id.as_ref() {
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

        Ok(())
    }
}
