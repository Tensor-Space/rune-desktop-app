use crate::core::{app::AppState, error::AppError};
use std::{str::FromStr, sync::Arc};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutEvent, ShortcutState,
};

pub struct ShortcutManager {
    app_state: Arc<AppState>,
}

impl ShortcutManager {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }

    pub fn register_shortcuts(&self, app: &tauri::App) -> Result<(), AppError> {
        let handle = app.handle();
        let settings = self.app_state.settings.read().clone();

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

        let recording_pipeline = match &*self.app_state.audio_pipeline.lock() {
            Some(pipeline) => Arc::clone(pipeline),
            None => {
                return Err(AppError::Generic(
                    "Audio pipeline not initialized".to_string(),
                ))
            }
        };

        let app_state = Arc::clone(&self.app_state);

        handle.plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(
                    move |_app_handle: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent| {
                        if shortcut == &record_shortcut {
                            match event.state {
                                ShortcutState::Pressed => {
                                    let _ = app_state
                                        .runtime
                                        .block_on(recording_pipeline.start())
                                        .unwrap();
                                }
                                ShortcutState::Released => {
                                    let _ = app_state.runtime.block_on(recording_pipeline.stop());
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

        Ok(())
    }
}
