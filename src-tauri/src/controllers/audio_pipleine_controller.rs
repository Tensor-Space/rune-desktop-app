use std::{path::PathBuf, process::Command, sync::Arc, thread};

use crate::{
    core::{app::AppState, utils::audio::get_recordings_path},
    services::{
        audio_recording_service::AudioRecordingService,
        text_processing_service::TextProcessingService,
        text_transcript_history_service::TextTranscriptHistoryService,
        text_transcription_service::TextTranscriptionService,
    },
};
use parking_lot::{Mutex, MutexGuard};
use tauri::{path::BaseDirectory, AppHandle, Emitter, Manager};

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

pub struct AudioPipelineController {
    state: Arc<AppState>,
    previous_app: parking_lot::Mutex<Option<String>>,
    app_handle: AppHandle,
    recording_service: Arc<Mutex<AudioRecordingService>>,
    transcription_service: Arc<Mutex<TextTranscriptionService>>,
}

impl AudioPipelineController {
    pub fn new(state: Arc<AppState>, app_handle: AppHandle) -> Self {
        let recording_service = Arc::new(Mutex::new(AudioRecordingService::new()));

        // Ensure we resolve the model directory properly
        let resource_dir = app_handle
            .path()
            .resolve("models/whisper-base", BaseDirectory::Resource)
            .ok();

        println!("Using model directory: {:?}", resource_dir);

        let transcription_service =
            match TextTranscriptionService::new(resource_dir, Some(app_handle.clone())) {
                Ok(t) => Arc::new(Mutex::new(t)),
                Err(e) => {
                    log::error!("Failed to create transcriber with custom path: {}", e);

                    // Try to find model in common locations as a fallback
                    let fallback_paths = [
                        dirs::data_dir().map(|p| p.join("rune/models/whisper-base")),
                        Some(PathBuf::from("./models/whisper-base")),
                        Some(PathBuf::from("../models/whisper-base")),
                    ];

                    for path in fallback_paths.iter().flatten() {
                        if path.exists() {
                            log::info!("Trying fallback model path: {:?}", path);
                            if let Ok(t) = TextTranscriptionService::new(
                                Some(path.clone()),
                                Some(app_handle.clone()),
                            ) {
                                return Self {
                                    state,
                                    previous_app: parking_lot::Mutex::new(None),
                                    app_handle,
                                    recording_service,
                                    transcription_service: Arc::new(Mutex::new(t)),
                                };
                            }
                        }
                    }

                    // Last resort - create a transcriber without a model
                    // It won't be able to transcribe but can handle history operations
                    log::warn!(
                        "Creating transcriber without model - will not be able to transcribe"
                    );
                    match TextTranscriptionService::new(None, Some(app_handle.clone())) {
                        Ok(t) => Arc::new(Mutex::new(t)),
                        Err(e) => {
                            log::error!("Failed to create transcriber: {}", e);
                            panic!("Cannot initialize transcriber: {}", e);
                        }
                    }
                }
            };

        Self {
            state,
            previous_app: parking_lot::Mutex::new(None),
            app_handle,
            recording_service,
            transcription_service,
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

    pub fn activate_app(app_name: &str) {
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

        let recording_service = self.recording_service.lock();
        recording_service.set_device_id(device_id);
        recording_service.set_app_handle(self.app_handle.clone());

        match recording_service.start_recording(&self.app_handle).await {
            Ok(_) => window.emit(
                "audio-processing-status",
                ProcessingStatus::Recording.as_str(),
            ),
            Err(e) => {
                log::error!("Failed to start recording: {}", e);
                window.emit(
                    "audio-processing-status",
                    ProcessingStatus::Error(format!("Failed to start recording: {}", e)).as_str(),
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
            .recording_service
            .lock()
            .stop_recording(temp_path.clone())
            .await
        {
            log::error!("Failed to stop recording: {}", e);
            window
                .emit(
                    "audio-processing-status",
                    ProcessingStatus::Error(format!("Failed to stop recording: {}", e)).as_str(),
                )
                .unwrap_or_else(|e| log::error!("Failed to emit error status: {}", e));
            return;
        }

        // Clone what we need for the background thread
        let app_handle = self.app_handle.clone();
        let transcription_service = self.transcription_service.clone();
        let state = self.state.clone();
        let previous_app_copy = self.previous_app.lock().clone();
        let mut previous_app_mutex = self.previous_app.lock().clone();

        // Use a regular thread for CPU-intensive transcription
        thread::spawn(move || {
            // Update UI status to transcribing
            app_handle
                .emit(
                    "audio-processing-status",
                    ProcessingStatus::Transcribing.as_str(),
                )
                .unwrap_or_else(|e| log::error!("Failed to emit status: {}", e));

            if !temp_path.exists() {
                app_handle
                    .emit(
                        "audio-processing-status",
                        ProcessingStatus::Error("No recording found to transcribe".to_string())
                            .as_str(),
                    )
                    .unwrap_or_else(|e| log::error!("Failed to emit error status: {}", e));
                return;
            }

            // Get the app name that was captured
            let app_name = previous_app_copy.unwrap_or_default();

            // Perform transcription (CPU-intensive)
            let transcription_result = {
                let mut transcription_service_guard = transcription_service.lock();
                transcription_service_guard.transcribe(temp_path.clone())
            };

            match transcription_result {
                Ok(transcription) => {
                    if let Some(text) = transcription.first() {
                        // Update UI status to thinking
                        app_handle
                            .emit(
                                "audio-processing-status",
                                ProcessingStatus::ThinkingAction.as_str(),
                            )
                            .unwrap_or_else(|e| log::error!("Failed to emit status: {}", e));

                        // Now run the text processing (which is also async) in a task runner
                        let text_clone = text.clone();
                        let app_name_clone = app_name.clone();

                        // Create a one-shot channel for communicating results
                        let (tx, rx) = std::sync::mpsc::channel();

                        // Spawn a tokio runtime for the async processing
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap();

                        // Run the async processing
                        rt.block_on(async {
                            let result = TextProcessingService::process_text(
                                &state,
                                &app_name_clone,
                                &text_clone,
                            )
                            .await;
                            tx.send(result).unwrap();
                        });

                        // Get the processing result
                        match rx.recv().unwrap() {
                            Ok(processed_text) => {
                                // Activate the previous app
                                if let Some(app) = previous_app_mutex.take() {
                                    AudioPipelineController::activate_app(&app);
                                }

                                // Inject the processed text
                                if let Err(e) = TextProcessingService::inject_text(&processed_text)
                                {
                                    log::error!("Failed to inject text: {}", e);
                                }

                                // Save the processed text to history using TextTranscriptHistoryService
                                if let Err(e) = TextTranscriptHistoryService::save_processed_text(
                                    &app_handle,
                                    &processed_text,
                                ) {
                                    log::error!("Failed to save processed text to history: {}", e);
                                }

                                // Update UI status to completed and hide window
                                app_handle
                                    .emit(
                                        "audio-processing-status",
                                        ProcessingStatus::Completed.as_str(),
                                    )
                                    .unwrap_or_else(|e| {
                                        log::error!("Failed to emit status: {}", e)
                                    });

                                if let Some(window) = app_handle.get_webview_window("main") {
                                    window.hide().unwrap_or_else(|e| {
                                        log::error!("Failed to hide window: {}", e)
                                    });
                                }

                                // Update history window if it's open
                                if let Some(_history_window) =
                                    app_handle.get_webview_window("history")
                                {
                                    // Instead of checking if the window is visible, just emit a global event
                                    // This will be picked up by any open history windows
                                    // We already emit the transcription-added event in save_transcription,
                                    // but emit a refresh event as a fallback
                                    if let Err(e) = app_handle.emit("refresh-history", ()) {
                                        log::error!("Failed to emit history refresh event: {}", e);
                                    } else {
                                        log::info!("Emitted history refresh event");
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Text processing error: {}", e);

                                // Activate the previous app
                                if let Some(app) = previous_app_mutex.take() {
                                    AudioPipelineController::activate_app(&app);
                                }

                                // Fall back to injecting the original text
                                TextProcessingService::inject_text(text).unwrap_or_else(|e| {
                                    log::error!("Failed to inject original text: {}", e)
                                });

                                // Update UI status to error
                                app_handle
                                    .emit(
                                        "audio-processing-status",
                                        ProcessingStatus::Error(format!(
                                            "Processing failed: {}",
                                            e
                                        ))
                                        .as_str(),
                                    )
                                    .unwrap_or_else(|e| {
                                        log::error!("Failed to emit error status: {}", e)
                                    });
                            }
                        }
                    } else {
                        log::error!("No transcription text available");
                        app_handle
                            .emit(
                                "audio-processing-status",
                                ProcessingStatus::Error("No text transcribed".to_string()).as_str(),
                            )
                            .unwrap_or_else(|e| log::error!("Failed to emit error status: {}", e));
                    }
                }
                Err(e) => {
                    log::error!("Transcription error: {}", e);
                    app_handle
                        .emit(
                            "audio-processing-status",
                            ProcessingStatus::Error(format!("Transcription failed: {}", e))
                                .as_str(),
                        )
                        .unwrap_or_else(|e| log::error!("Failed to emit error status: {}", e));
                }
            }
        });
    }

    // Getter methods for the services
    pub fn get_recording_service(&self) -> MutexGuard<AudioRecordingService> {
        self.recording_service.lock()
    }

    pub fn get_transcription_service(&self) -> MutexGuard<TextTranscriptionService> {
        self.transcription_service.lock()
    }
}
