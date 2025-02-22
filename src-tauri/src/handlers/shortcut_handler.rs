use std::{process::Command, sync::Arc};

use crate::{
    core::{app::AppState, utils::audio::get_recordings_path},
    io::text_injector::TextInjector,
    prompts::{
        action_checker::TextTransformer as ActionChecker, text_generator::TextGenerator,
        text_transformer::TextTransformer,
    },
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

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

pub async fn handle_record_press(
    window: WebviewWindow,
    state: Arc<AppState>,
    previous_app: &Arc<parking_lot::Mutex<Option<String>>>,
    app_handle: &AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(app_name) = get_frontmost_app_name() {
        *previous_app.lock() = Some(app_name);
    }

    window.show().unwrap();
    window.set_focus().unwrap();

    let settings = app_handle.state::<Arc<AppState>>().settings.read().clone();
    let device_id = settings.audio.default_device.clone();
    let recorder = state.recorder.lock();
    recorder.set_device_id(device_id);

    if let Err(e) = recorder.start_recording(app_handle).await {
        log::error!("Failed to start recording: {}", e);
        window
            .emit("transcription-status", "error")
            .unwrap_or_else(|e| {
                log::error!("Failed to emit error status: {}", e);
            });
    }

    Ok(())
}

pub async fn handle_record_release(
    window: WebviewWindow,
    state: Arc<AppState>,
    previous_app: Arc<parking_lot::Mutex<Option<String>>>,
    app_handle: AppHandle,
) {
    let temp_path = get_recordings_path(&app_handle).join("rune_recording.wav");
    let recorder = state.recorder.lock();

    if let Err(e) = recorder.stop_recording(temp_path.clone()).await {
        log::error!("Failed to stop recording: {}", e);
        window
            .emit("transcription-status", "error")
            .unwrap_or_else(|e| {
                log::error!("Failed to emit error status: {}", e);
            });
        return;
    }

    // Emit transcription start
    window
        .emit("transcription-status", "started")
        .unwrap_or_else(|e| {
            log::error!("Failed to emit start status: {}", e);
        });

    println!("Audio Recorded now transcribing");
    let temp_path = get_recordings_path(&app_handle).join("rune_recording.wav");
    let mut transcriber = state.transcriber.lock();
    let app_name = previous_app.lock().clone().unwrap_or_default();

    if temp_path.exists() {
        match transcriber.transcribe(temp_path.clone()) {
            Ok(transcription) => {
                println!("Transcription: {:?}", transcription);
                if let Some(text) = transcription.first() {
                    // Emit completion status
                    window
                        .emit("transcription-status", "completed")
                        .unwrap_or_else(|e| {
                            log::error!("Failed to emit completion status: {}", e);
                        });

                    let llm_client = state.llm_client.lock();

                    // First check if action is required
                    let action_schema = ActionChecker::get_schema();
                    let action_prompt = ActionChecker::get_prompt(text);

                    match llm_client
                        .execute_prompt(&action_prompt, "action_checker", Some(action_schema))
                        .await
                    {
                        Ok(action_response) => {
                            let action_required = action_response["action_required"]
                                .as_bool()
                                .unwrap_or(false);

                            println!("Action required: {}", action_required);

                            // Choose prompt based on action_required
                            let (prompt, schema) = if action_required {
                                (
                                    TextGenerator::get_prompt(&app_name, text),
                                    TextGenerator::get_schema(),
                                )
                            } else {
                                (
                                    TextTransformer::get_prompt(&app_name, text),
                                    TextTransformer::get_schema(),
                                )
                            };

                            match llm_client
                                .execute_prompt(
                                    &prompt,
                                    if action_required {
                                        "text_generator"
                                    } else {
                                        "text_transformer"
                                    },
                                    Some(schema),
                                )
                                .await
                            {
                                Ok(response) => {
                                    let transformed_text =
                                        response["output"].as_str().unwrap_or(text).to_string();
                                    println!("Transformed text: {:?}", transformed_text);

                                    // Activate previous app
                                    if let Some(app_name) = previous_app.lock().take() {
                                        activate_app(&app_name);
                                    }

                                    // Inject transformed text
                                    if let Ok(mut injector) = TextInjector::new() {
                                        if let Err(e) = injector.inject_text(&transformed_text) {
                                            log::error!("Failed to inject text: {}", e);
                                            window
                                                .emit("transcription-status", "error")
                                                .unwrap_or_else(|e| {
                                                    log::error!(
                                                        "Failed to emit error status: {}",
                                                        e
                                                    );
                                                });
                                        }
                                    }

                                    window.hide().unwrap();
                                }
                                Err(e) => {
                                    log::error!("LLM transformation error: {}", e);
                                    // Fall back to original text if LLM fails
                                    if let Ok(mut injector) = TextInjector::new() {
                                        if let Err(e) = injector.inject_text(text) {
                                            log::error!("Failed to inject text: {}", e);
                                        }
                                    }
                                    window.hide().unwrap();
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Action checker error: {}", e);
                            // Fall back to text_transformer if action check fails
                            let prompt = TextTransformer::get_prompt(&app_name, text);
                            let schema = TextTransformer::get_schema();

                            match llm_client
                                .execute_prompt(&prompt, "text_transformer", Some(schema))
                                .await
                            {
                                Ok(response) => {
                                    let transformed_text =
                                        response["output"].as_str().unwrap_or(text).to_string();

                                    if let Some(app_name) = previous_app.lock().take() {
                                        activate_app(&app_name);
                                    }

                                    if let Ok(mut injector) = TextInjector::new() {
                                        if let Err(e) = injector.inject_text(&transformed_text) {
                                            log::error!("Failed to inject text: {}", e);
                                        }
                                    }
                                    window.hide().unwrap();
                                }
                                Err(e) => {
                                    log::error!("Fallback transformation error: {}", e);
                                    // Use original text as last resort
                                    if let Ok(mut injector) = TextInjector::new() {
                                        if let Err(e) = injector.inject_text(text) {
                                            log::error!("Failed to inject text: {}", e);
                                        }
                                    }
                                    window.hide().unwrap();
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Transcription error: {}", e);
                window
                    .emit("transcription-status", "error")
                    .unwrap_or_else(|e| {
                        log::error!("Failed to emit error status: {}", e);
                    });
            }
        }
    }
}

pub fn handle_escape(
    window: &WebviewWindow,
    previous_app: &Arc<parking_lot::Mutex<Option<String>>>,
) {
    window.hide().unwrap();
    if let Some(app_name) = previous_app.lock().take() {
        activate_app(&app_name);
    }
}
