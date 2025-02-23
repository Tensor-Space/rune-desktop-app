use std::{process::Command, sync::Arc};

use crate::{
    core::{app::AppState, utils::audio::get_recordings_path},
    text::text_processor_pipeline::TextProcessorPipeline,
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

#[derive(Debug, Clone)]
pub enum ProcessingStatus {
    Idle,
    Recording,
    Transcribing,
    ThinkingAction,
    GeneratingText,
    Completed,
    Error(String),
}

impl ProcessingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProcessingStatus::Idle => "idle",
            ProcessingStatus::Recording => "recording",
            ProcessingStatus::Transcribing => "transcribing",
            ProcessingStatus::ThinkingAction => "thinking_action",
            ProcessingStatus::GeneratingText => "generating_text",
            ProcessingStatus::Completed => "completed",
            ProcessingStatus::Error(_) => "error",
        }
    }
}

pub struct RecordingPipeline {
    state: Arc<AppState>,
    previous_app: parking_lot::Mutex<Option<String>>,
    app_handle: AppHandle,
}

impl RecordingPipeline {
    pub fn new(state: Arc<AppState>, app_handle: AppHandle) -> Self {
        Self {
            state,
            previous_app: parking_lot::Mutex::new(None),
            app_handle,
        }
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

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(app_name) = Self::get_frontmost_app_name() {
            *self.previous_app.lock() = Some(app_name);
        }

        let window = self.app_handle.get_webview_window("main").unwrap();
        window.show().unwrap();
        window.set_focus().unwrap();

        let settings = self.state.settings.read().clone();
        let device_id = settings.audio.default_device.clone();
        let recorder = self.state.recorder.lock();
        recorder.set_device_id(device_id);

        match recorder.start_recording(&self.app_handle).await {
            Ok(_) => window.emit(
                "audio-processing-status",
                ProcessingStatus::Recording.as_str(),
            ),
            Err(e) => {
                log::error!("Failed to start recording: {}", e);
                window.emit(
                    "audio-processing-status",
                    ProcessingStatus::Error("Failed to start recording".to_string()).as_str(),
                )
            }
        }
        .unwrap_or_else(|e| log::error!("Failed to emit status: {}", e));

        Ok(())
    }

    pub async fn stop(&self) {
        let window = self.app_handle.get_webview_window("main").unwrap();
        let temp_path = get_recordings_path(&self.app_handle).join("rune_recording.wav");

        if let Err(e) = self
            .state
            .recorder
            .lock()
            .stop_recording(temp_path.clone())
            .await
        {
            log::error!("Failed to stop recording: {}", e);
            window
                .emit(
                    "audio-processing-status",
                    ProcessingStatus::Error("Failed to stop recording".to_string()).as_str(),
                )
                .unwrap_or_else(|e| log::error!("Failed to emit error status: {}", e));
            return;
        }

        self.process_recording(window, temp_path).await;
    }

    async fn process_recording(&self, window: WebviewWindow, temp_path: std::path::PathBuf) {
        window
            .emit(
                "audio-processing-status",
                ProcessingStatus::Transcribing.as_str(),
            )
            .unwrap_or_else(|e| log::error!("Failed to emit status: {}", e));

        let mut transcriber = self.state.transcriber.lock();
        let app_name = self.previous_app.lock().clone().unwrap_or_default();

        if !temp_path.exists() {
            return;
        }

        match transcriber.transcribe(temp_path) {
            Ok(transcription) => {
                if let Some(text) = transcription.first() {
                    window
                        .emit(
                            "audio-processing-status",
                            ProcessingStatus::ThinkingAction.as_str(),
                        )
                        .unwrap_or_else(|e| log::error!("Failed to emit status: {}", e));

                    match TextProcessorPipeline::process_text(&self.state, &app_name, text).await {
                        Ok(processed_text) => {
                            if let Some(app_name) = self.previous_app.lock().take() {
                                Self::activate_app(&app_name);
                            }

                            if let Err(e) = TextProcessorPipeline::inject_text(&processed_text) {
                                log::error!("Failed to inject text: {}", e);
                            }

                            window
                                .emit(
                                    "audio-processing-status",
                                    ProcessingStatus::Completed.as_str(),
                                )
                                .unwrap_or_else(|e| log::error!("Failed to emit status: {}", e));

                            window
                                .hide()
                                .unwrap_or_else(|e| log::error!("Failed to hide window: {}", e));
                        }
                        Err(e) => {
                            log::error!("Text processing error: {}", e);
                            if let Some(app_name) = self.previous_app.lock().take() {
                                Self::activate_app(&app_name);
                            }
                            TextProcessorPipeline::inject_text(text).unwrap_or_else(|e| {
                                log::error!("Failed to inject original text: {}", e)
                            });
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Transcription error: {}", e);
                window
                    .emit(
                        "audio-processing-status",
                        ProcessingStatus::Error("Transcription failed".to_string()).as_str(),
                    )
                    .unwrap_or_else(|e| log::error!("Failed to emit error status: {}", e));
            }
        }
    }
}
