use crate::{
    core::{app::AppState, error::AppError, utils::audio::get_recordings_path},
    io::text_injector::TextInjector,
};
use std::{process::Command, str::FromStr, sync::Arc};
use tauri::{AppHandle, Emitter, Manager};
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

    fn get_frontmost_app_name() -> Option<String> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "System Events" to get name of first application process whose frontmost is true"#)
            .output()
            .ok()?;

        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
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
                            let recorder = &state.audio.recorder;

                            match event.state {
                                ShortcutState::Pressed => {
                                    // Store current app
                                    if let Some(app_name) = Self::get_frontmost_app_name() {
                                        *previous_app_clone.lock() = Some(app_name);
                                    }

                                    // Show window and start recording
                                    window.show().unwrap();
                                    window.set_focus().unwrap();

                                    let settings = app_handle.state::<Arc<AppState>>().settings.read().clone();
                                    let device_id = settings.audio.default_device.clone();
                                    recorder.set_device_id(device_id);

                                    if let Err(e) = rt.block_on(recorder.start_recording(app_handle)) {
                                        log::error!("Failed to start recording: {}", e);
                                        window.emit("transcription-status", "error").unwrap_or_else(|e| {
                                            log::error!("Failed to emit error status: {}", e);
                                        });
                                    }
                                }
                                ShortcutState::Released => {
                                    let temp_path = get_recordings_path(&app_handle).join("rune_recording.wav");

                                    if let Err(e) = rt.block_on(recorder.stop_recording(temp_path.clone())) {
                                        log::error!("Failed to stop recording: {}", e);
                                        window.emit("transcription-status", "error").unwrap_or_else(|e| {
                                            log::error!("Failed to emit error status: {}", e);
                                        });
                                        return;
                                    }

                                    // Emit transcription start
                                    window.emit("transcription-status", "started").unwrap_or_else(|e| {
                                        log::error!("Failed to emit start status: {}", e);
                                    });

                                    let state_clone = Arc::clone(&state);
                                    let previous_app_clone = Arc::clone(&previous_app_clone);
                                    let window_clone = window.clone();
                                    let app_handle_clone = app_handle.clone();

                                    // Spawn async task
                                    rt.spawn(async move {
                                        println!("Audio Recorded now transcribing");
                                        let temp_path = get_recordings_path(&app_handle_clone).join("rune_recording.wav");

                                        if temp_path.exists() {
                                            if let Ok(mut transcriber) = state_clone.audio.transcriber.lock() {
                                                match transcriber.transcribe(temp_path.clone()) {
                                                    Ok(transcription) => {
                                                        println!("Transcription: {:?}", transcription);
                                                        if let Some(text) = transcription.first() {
                                                            // Emit completion status
                                                            window_clone.emit("transcription-status", "completed").unwrap_or_else(|e| {
                                                                log::error!("Failed to emit completion status: {}", e);
                                                            });

                                                            // Activate previous app
                                                            if let Some(app_name) = previous_app_clone.lock().take() {
                                                                Self::activate_app(&app_name);
                                                            }

                                                            // Inject text
                                                            if let Ok(mut injector) = TextInjector::new() {
                                                                if let Err(e) = injector.inject_text(text) {
                                                                    log::error!("Failed to inject text: {}", e);
                                                                    window_clone.emit("transcription-status", "error").unwrap_or_else(|e| {
                                                                        log::error!("Failed to emit error status: {}", e);
                                                                    });
                                                                }
                                                            }

                                                            window_clone.hide().unwrap();
                                                        }
                                                    }
                                                    Err(e) => {
                                                        log::error!("Transcription error: {}", e);
                                                        window_clone.emit("transcription-status", "error").unwrap_or_else(|e| {
                                                            log::error!("Failed to emit error status: {}", e);
                                                        });
                                                    }
                                                }
                                            } else {
                                                log::error!("Failed to acquire transcriber lock");
                                                window_clone.emit("transcription-status", "error").unwrap_or_else(|e| {
                                                    log::error!("Failed to emit error status: {}", e);
                                                });
                                            }
                                        }
                                    });
                                }
                            }
                        } else if shortcut == &escape_shortcut {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                window.hide().unwrap();
                                if let Some(app_name) = previous_app_clone.lock().take() {
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
