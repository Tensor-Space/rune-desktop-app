use std::{path::PathBuf, process::Command, sync::Arc};
use tokio::sync::oneshot;

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
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{path::BaseDirectory, AppHandle, Emitter, Manager};

#[derive(Debug, Clone)]
pub enum ProcessingStatus {
    Idle,
    Recording,
    Transcribing,
    ThinkingAction,
    GeneratingText,
    Completed,
    Cancelled,
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
            ProcessingStatus::Cancelled => "cancelled",
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
    cancellation_token: Arc<AtomicBool>,
    is_processing: Arc<AtomicBool>,
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
                                    cancellation_token: Arc::new(AtomicBool::new(false)),
                                    is_processing: Arc::new(AtomicBool::new(false)),
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
            cancellation_token: Arc::new(AtomicBool::new(false)),
            is_processing: Arc::new(AtomicBool::new(false)),
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

    pub fn is_processing(&self) -> bool {
        self.is_processing.load(Ordering::SeqCst)
    }

    pub async fn cancel(&self) {
        log::info!("Cancelling audio processing pipeline");
        self.cancellation_token.store(true, Ordering::SeqCst);

        if self.is_processing() {
            log::info!("Pipeline is in processing state, stopping immediately");

            {
                let recording_service = self.recording_service.lock();
                if let Err(e) = recording_service.stop_recording_without_save().await {
                    log::error!("Error stopping recording during cancellation: {}", e);
                }
            }

            if let Some(window) = self.app_handle.get_webview_window("main") {
                if let Ok(true) = window.is_visible() {
                    let _ = window.emit(
                        "audio-processing-status",
                        ProcessingStatus::Cancelled.as_str(),
                    );
                    let _ = window.hide();
                }
            }

            self.is_processing.store(false, Ordering::SeqCst);

            if let Some(app) = self.previous_app.lock().take() {
                Self::activate_app(&app);
            }
        }
    }

    pub fn cancel_sync(&self) {
        self.cancellation_token.store(true, Ordering::SeqCst);

        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.block_on(self.cancel());
        } else {
            self.is_processing.store(false, Ordering::SeqCst);

            if let Some(app) = self.previous_app.lock().take() {
                Self::activate_app(&app);
            }
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.cancellation_token.store(false, Ordering::SeqCst);
        self.is_processing.store(true, Ordering::SeqCst);

        if let Some(app_name) = Self::get_frontmost_app_name() {
            *self.previous_app.lock() = Some(app_name);
        }

        let window = self.app_handle.get_webview_window("main").unwrap();
        window.show().unwrap();
        window.set_focus().unwrap();

        let settings = self.state.settings.read().clone();
        let device_id = settings.audio.default_device.clone();

        {
            let recording_service = self.recording_service.lock();
            recording_service.set_device_id(device_id);
            recording_service.set_app_handle(self.app_handle.clone());
        }

        let result = {
            let recording_service = self.recording_service.lock();
            recording_service.start_recording(&self.app_handle).await
        };

        match result {
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

        self.cancellation_token.store(false, Ordering::SeqCst);

        let recording_result = {
            let recording_service = self.recording_service.lock();
            recording_service.stop_recording(temp_path.clone()).await
        };

        if let Err(e) = recording_result {
            log::error!("Failed to stop recording: {}", e);
            window
                .emit(
                    "audio-processing-status",
                    ProcessingStatus::Error(format!("Failed to stop recording: {}", e)).as_str(),
                )
                .unwrap_or_else(|e| log::error!("Failed to emit error status: {}", e));

            self.is_processing.store(false, Ordering::SeqCst);
            return;
        }

        if let Err(e) = window.emit(
            "audio-processing-status",
            ProcessingStatus::Transcribing.as_str(),
        ) {
            log::error!("Failed to emit status: {}", e);
        }

        if self.cancellation_token.load(Ordering::SeqCst) {
            log::info!("Processing cancelled during transcription setup");
            if let Err(e) = window.emit(
                "audio-processing-status",
                ProcessingStatus::Cancelled.as_str(),
            ) {
                log::error!("Failed to emit status: {}", e);
            }

            self.is_processing.store(false, Ordering::SeqCst);

            if let Some(app) = self.previous_app.lock().take() {
                Self::activate_app(&app);
            }
            return;
        }

        if !temp_path.exists() {
            if let Err(e) = window.emit(
                "audio-processing-status",
                ProcessingStatus::Error("No recording found to transcribe".to_string()).as_str(),
            ) {
                log::error!("Failed to emit error status: {}", e);
            }
            self.is_processing.store(false, Ordering::SeqCst);
            return;
        }

        let app_name = self.previous_app.lock().clone().unwrap_or_default();

        let (tx, rx) = oneshot::channel();

        let temp_path_clone = temp_path.clone();
        let transcription_service = self.transcription_service.clone();
        let cancellation_token = self.cancellation_token.clone();

        std::thread::spawn(move || {
            let transcription_result = {
                let mut transcription_service_guard = transcription_service.lock();
                transcription_service_guard.transcribe(temp_path_clone)
            };

            if cancellation_token.load(Ordering::SeqCst) {
                let _ = tx.send(Err(anyhow::anyhow!("Processing cancelled")));
                return;
            }

            let _ = tx.send(transcription_result.map_err(|e| anyhow::anyhow!("{}", e)));
        });

        let transcription_result: Result<Vec<std::string::String>, anyhow::Error> = match rx.await {
            Ok(result) => result,
            Err(e) => {
                log::error!("Failed to receive transcription result: {}", e);
                if let Err(e) = window.emit(
                    "audio-processing-status",
                    ProcessingStatus::Error(format!("Transcription failed: {}", e)).as_str(),
                ) {
                    log::error!("Failed to emit error status: {}", e);
                }
                self.is_processing.store(false, Ordering::SeqCst);
                return;
            }
        };

        if self.cancellation_token.load(Ordering::SeqCst) {
            log::info!("Processing cancelled after transcription");
            if let Err(e) = window.emit(
                "audio-processing-status",
                ProcessingStatus::Cancelled.as_str(),
            ) {
                log::error!("Failed to emit status: {}", e);
            }

            self.is_processing.store(false, Ordering::SeqCst);

            if let Some(app) = self.previous_app.lock().take() {
                Self::activate_app(&app);
            }
            return;
        }

        match transcription_result {
            Ok(transcription) => {
                if let Some(text) = transcription.first() {
                    // Update UI status to thinking
                    if let Err(e) = window.emit(
                        "audio-processing-status",
                        ProcessingStatus::ThinkingAction.as_str(),
                    ) {
                        log::error!("Failed to emit status: {}", e);
                    }

                    let text_clone = text.clone();
                    let app_name_clone = app_name.clone();
                    let state = self.state.clone();
                    let app_handle = self.app_handle.clone();
                    let cancellation_token = self.cancellation_token.clone();
                    let is_processing = self.is_processing.clone();
                    let mut previous_app_opt = self.previous_app.lock().clone();

                    let process_result = match state.runtime.block_on(async {
                        if cancellation_token.load(Ordering::SeqCst) {
                            return Err(anyhow::anyhow!("Processing cancelled"));
                        }

                        TextProcessingService::process_text(&state, &app_name_clone, &text_clone)
                            .await
                    }) {
                        Ok(processed_text) => {
                            if cancellation_token.load(Ordering::SeqCst) {
                                log::info!("Processing cancelled after text processing");
                                if let Err(e) = window.emit(
                                    "audio-processing-status",
                                    ProcessingStatus::Cancelled.as_str(),
                                ) {
                                    log::error!("Failed to emit status: {}", e);
                                }

                                is_processing.store(false, Ordering::SeqCst);

                                if let Some(app) = previous_app_opt.take() {
                                    Self::activate_app(&app);
                                }
                                return;
                            }

                            if let Some(app) = previous_app_opt.take() {
                                Self::activate_app(&app);
                            }

                            if let Err(e) = TextProcessingService::inject_text(&processed_text) {
                                log::error!("Failed to inject text: {}", e);
                            }

                            if let Err(e) = TextTranscriptHistoryService::save_processed_text(
                                &app_handle,
                                &processed_text,
                            ) {
                                log::error!("Failed to save processed text to history: {}", e);
                            }

                            if let Err(e) = window.emit(
                                "audio-processing-status",
                                ProcessingStatus::Completed.as_str(),
                            ) {
                                log::error!("Failed to emit status: {}", e);
                            }

                            if let Some(window) = app_handle.get_webview_window("main") {
                                if let Err(e) = window.hide() {
                                    log::error!("Failed to hide window: {}", e);
                                }
                            }

                            if let Some(_history_window) = app_handle.get_webview_window("history")
                            {
                                if let Err(e) = app_handle.emit("refresh-history", ()) {
                                    log::error!("Failed to emit history refresh event: {}", e);
                                } else {
                                    log::info!("Emitted history refresh event");
                                }
                            }

                            Ok(())
                        }
                        Err(e) => Err(e),
                    };

                    if let Err(e) = process_result {
                        log::error!("Text processing error: {}", e);

                        if self.cancellation_token.load(Ordering::SeqCst) {
                            if let Err(e) = window.emit(
                                "audio-processing-status",
                                ProcessingStatus::Cancelled.as_str(),
                            ) {
                                log::error!("Failed to emit status: {}", e);
                            }
                        } else {
                            if let Some(app) = self.previous_app.lock().take() {
                                Self::activate_app(&app);
                            }

                            if let Err(e) = TextProcessingService::inject_text(text) {
                                log::error!("Failed to inject original text: {}", e);
                            }

                            if let Err(e) = window.emit(
                                "audio-processing-status",
                                ProcessingStatus::Error(format!("Processing failed: {}", e))
                                    .as_str(),
                            ) {
                                log::error!("Failed to emit error status: {}", e);
                            }
                        }
                    }

                    self.is_processing.store(false, Ordering::SeqCst);
                } else {
                    log::error!("No transcription text available");
                    if let Err(e) = window.emit(
                        "audio-processing-status",
                        ProcessingStatus::Error("No text transcribed".to_string()).as_str(),
                    ) {
                        log::error!("Failed to emit error status: {}", e);
                    }

                    self.is_processing.store(false, Ordering::SeqCst);
                }
            }
            Err(e) => {
                log::error!("Transcription error: {}", e);
                if let Err(e) = window.emit(
                    "audio-processing-status",
                    ProcessingStatus::Error(format!("Transcription failed: {}", e)).as_str(),
                ) {
                    log::error!("Failed to emit error status: {}", e);
                }

                self.is_processing.store(false, Ordering::SeqCst);
            }
        }
    }

    pub fn get_recording_service(&self) -> MutexGuard<AudioRecordingService> {
        self.recording_service.lock()
    }

    pub fn get_transcription_service(&self) -> MutexGuard<TextTranscriptionService> {
        self.transcription_service.lock()
    }
}
