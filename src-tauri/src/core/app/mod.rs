mod setup;
mod state;

use crate::{
    commands,
    core::{config::Settings, error::AppError},
};
pub use state::AppState;
use std::sync::Arc;

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
                log::info!("App already running, skipping creation of new instance");
            }))
            .invoke_handler(tauri::generate_handler![
                // Audio commands
                commands::audio_commands::get_devices,
                commands::audio_commands::set_default_device,
                commands::audio_commands::get_default_device,
                commands::audio_commands::get_transcription_history,
                commands::audio_commands::cancel_recording,
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
                    log::info!(
                        "registered for autostart? {}",
                        autostart_manager.is_enabled().unwrap()
                    );
                    let _ = autostart_manager.disable();
                }

                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                setup::setup_app(app, self.state.clone())?;

                Ok(())
            });

        builder
            .run(tauri::generate_context!())
            .map_err(|e| e.into())
    }
}
