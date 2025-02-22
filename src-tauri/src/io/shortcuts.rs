use crate::{
    core::{app::AppState, error::AppError},
    handlers::shortcut_handler,
};
use std::{str::FromStr, sync::Arc};
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

        // Create escape shortcut
        let escape_shortcut = Shortcut::new(None, Code::Escape);

        let state = Arc::clone(&self.app_state);
        let previous_app = Arc::new(parking_lot::Mutex::new(None::<String>));
        let previous_app_clone = Arc::clone(&previous_app);

        // Create Tokio runtime
        let rt = Runtime::new().unwrap();

        handle.plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(
                    move |app_handle: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent| {
                        if shortcut == &record_shortcut {
                            let window = app_handle.get_webview_window("main").unwrap();

                            match event.state {
                                ShortcutState::Pressed => {
                                    let _ = rt
                                        .block_on(shortcut_handler::handle_record_press(
                                            window,
                                            Arc::clone(&state),
                                            &previous_app_clone,
                                            app_handle,
                                        ))
                                        .unwrap();
                                }
                                ShortcutState::Released => {
                                    let _ = rt.block_on(shortcut_handler::handle_record_release(
                                        window,
                                        Arc::clone(&state),
                                        Arc::clone(&previous_app_clone),
                                        app_handle.clone(),
                                    ));
                                }
                            }
                        } else if shortcut == &escape_shortcut {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                shortcut_handler::handle_escape(&window, &previous_app_clone);
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
