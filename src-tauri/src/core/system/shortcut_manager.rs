use crate::{
    core::{app::AppState, error::AppError},
    handlers::recording_pipeline_handler::RecordingPipeline,
};
use std::{process::Command, str::FromStr, sync::Arc};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutEvent, ShortcutState,
};
use tokio::runtime::Runtime;

pub struct ShortcutManager {
    app_state: Arc<AppState>,
}

impl ShortcutManager {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }

    fn activate_app(app_name: &str) {
        Command::new("osascript")
            .arg("-e")
            .arg(format!(r#"tell application "{}" to activate"#, app_name))
            .output()
            .ok();
    }

    pub fn register_shortcuts(&self, app: &tauri::App) -> Result<(), AppError> {
        let handle = app.handle();
        let settings = self.app_state.settings.read().clone();

        // Create the record shortcut from settings
        println!("{:?}", settings);
        let record_shortcut = {
            let modifier = settings
                .shortcuts
                .record_modifier
                .as_ref()
                .ok_or_else(|| AppError::Generic("Record modifier not set".to_string()))?;

            let parsed_modifier = Modifiers::from_name(modifier).ok_or_else(|| {
                AppError::Generic(format!("Failed to parse shortcut modifier '{}'", modifier))
            })?;

            let key = settings
                .shortcuts
                .record_key
                .as_ref()
                .ok_or_else(|| AppError::Generic("Record key not set".to_string()))?;

            println!("modifier: {:?}, key: {:?}", modifier, key);

            let parsed_key = Code::from_str(key).map_err(|e| {
                AppError::Generic(format!("Failed to parse shortcut key '{}': {}", key, e))
            })?;

            Shortcut::new(Some(parsed_modifier), parsed_key)
        };

        let escape_shortcut = Shortcut::new(None, Code::Escape);

        let previous_app = Arc::new(parking_lot::Mutex::new(None::<String>));
        let recording_pipeline = Arc::new(RecordingPipeline::new(
            Arc::clone(&self.app_state),
            handle.clone(),
        ));

        let rt = Runtime::new().unwrap();

        handle.plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(
                    move |app_handle: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent| {
                        if shortcut == &record_shortcut {
                            match event.state {
                                ShortcutState::Pressed => {
                                    let _ = rt.block_on(recording_pipeline.start()).unwrap();
                                }
                                ShortcutState::Released => {
                                    let _ = rt.block_on(recording_pipeline.stop());
                                }
                            }
                        } else if shortcut == &escape_shortcut {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                window.hide().unwrap();
                                if let Some(app_name) = previous_app.lock().take() {
                                    Self::activate_app(&app_name);
                                }
                            }
                        }
                    },
                )
                .build(),
        )?;

        app.global_shortcut()
            .register(record_shortcut)
            .map_err(|e| {
                log::error!("Failed to register record shortcut: {}", e);
                AppError::Generic("Failed to register record shortcut".to_string())
            })?;
        app.global_shortcut()
            .register(escape_shortcut)
            .map_err(|e| {
                log::error!("Failed to register escape shortcut: {}", e);
                AppError::Generic("Failed to register escape shortcut".to_string())
            })?;

        Ok(())
    }
}
