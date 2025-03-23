pub mod settings;
mod setup;
mod state;

use crate::{
    commands,
    core::{app::settings::Settings, error::AppError},
    services::recording_service::RecordingService,
};
use setup::AppSetup;
pub use state::AppState;
use std::sync::Arc;

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

        let (tx, rx) = RecordingService::create_channels();

        tauri::async_runtime::block_on(async {
            let mut tx_guard = state.recording_tx.lock().await;
            *tx_guard = Some(tx);
        });

        let builder = tauri::Builder::default()
            .manage(state.clone())
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
                commands::audio_commands::get_default_device,
                commands::audio_commands::start_recording,
                commands::audio_commands::stop_recording,
                // System commands
                commands::system_commands::check_accessibility_permissions,
                commands::system_commands::request_accessibility_permissions,
                commands::system_commands::check_microphone_permissions,
                commands::system_commands::request_microphone_permissions,
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

                let app_setup = AppSetup::new(self.state.clone());

                app_setup.setup_app(app, rx)?;

                Ok(())
            });

        builder
            .run(tauri::generate_context!())
            .map_err(|e| e.into())
    }
}
