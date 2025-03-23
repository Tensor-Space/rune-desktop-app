use super::SETTINGS_FILE;
use crate::{
    core::{
        app::settings::Settings,
        app::AppState,
        error::AppError,
        system::{
            permission_manager::PermissionManager, shortcut_manager::ShortcutManager,
            tray_manager::TrayManager, window_manager::WindowManager,
        },
    },
    events::{listeners, types::RecordingCommand},
    services::recording_service::RecordingService,
};
use std::sync::Arc;
use tauri::{path::BaseDirectory, App as TauriApp, Manager};
use tauri_plugin_store::StoreExt;
use tokio::sync::mpsc::Receiver;

pub struct AppSetup {
    state: Arc<AppState>,
}

impl AppSetup {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub fn setup_app(
        &self,
        app: &TauriApp,
        rx: Receiver<RecordingCommand>,
    ) -> Result<(), AppError> {
        self.setup_settings(app)?;

        self.setup_permissions(app)?;

        self.setup_windows(app)?;

        self.setup_shortcuts(app)?;

        self.setup_tray_menu(app)?;

        self.setup_whisper_model(app)?;

        self.setup_recording_service(app, rx)?;

        Ok(())
    }

    fn setup_settings(&self, app: &TauriApp) -> Result<(), AppError> {
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

        Ok(())
    }

    fn setup_windows(&self, app: &TauriApp) -> Result<(), AppError> {
        let window_manager = WindowManager::new();
        window_manager.setup_windows(app)?;

        Ok(())
    }

    fn setup_shortcuts(&self, app: &TauriApp) -> Result<(), AppError> {
        let shortcut_manager = ShortcutManager::new(Arc::clone(&self.state));
        shortcut_manager.register_shortcuts(app)?;
        Ok(())
    }

    fn setup_tray_menu(&self, app: &TauriApp) -> Result<(), AppError> {
        let tray_manager = TrayManager::new(Arc::clone(&self.state));
        tray_manager.setup_tray(&app.app_handle())?;
        Ok(())
    }

    fn setup_recording_service(
        &self,
        app: &TauriApp,
        rx: Receiver<RecordingCommand>,
    ) -> Result<(), AppError> {
        let recording_service = Arc::new(RecordingService::new(
            self.state.clone(),
            app.app_handle().clone(),
        ));

        listeners::setup_app_event_listeners(&app.app_handle(), self.state.recording_tx.clone());

        let service_clone = recording_service.clone();
        std::thread::spawn(move || {
            tauri::async_runtime::block_on(async {
                service_clone.set_command_receiver(rx).await;
                service_clone.run().await;
            });
        });
        app.manage(Arc::clone(&recording_service));

        Ok(())
    }

    fn setup_permissions(&self, app: &TauriApp) -> Result<(), AppError> {
        let permission_manager = PermissionManager::new();

        match permission_manager.check_all_permissions() {
            Ok(all_granted) => {
                if !all_granted {
                    log::warn!("Not all required permissions are granted. Some features may not work properly.");
                }
            }
            Err(e) => {
                log::error!("Failed to check permissions: {}", e);
            }
        }

        app.manage(permission_manager);
        Ok(())
    }

    fn setup_whisper_model(&self, app: &TauriApp) -> Result<(), AppError> {
        // Get the app data directory
        let whisper_dir = app
            .app_handle()
            .path()
            .resolve("models/whisper-base", BaseDirectory::Resource)
            .map_err(|e| AppError::Generic(format!("Failed to resolve models directory: {}", e)))?;

        // Check if model files exist
        let model_file = whisper_dir.join("model.safetensors");
        let config_file = whisper_dir.join("config.json");
        let tokenizer_file = whisper_dir.join("tokenizer.json");

        let files_missing =
            !model_file.exists() || !config_file.exists() || !tokenizer_file.exists();

        if files_missing {
            log::warn!(
                "Whisper model files not found. Please download the following files to: {}",
                whisper_dir.display()
            );
            log::warn!("Required files:");
            log::warn!("  1. model.safetensors");
            log::warn!("  2. config.json");
            log::warn!("  3. tokenizer.json");
            log::warn!("These can be downloaded from https://huggingface.co/openai/whisper-base");

            // For automatic download (optional)
            // self.download_whisper_model(&whisper_dir)?;
        } else {
            log::info!("Whisper model files found at: {}", whisper_dir.display());
        }

        Ok(())
    }
}
