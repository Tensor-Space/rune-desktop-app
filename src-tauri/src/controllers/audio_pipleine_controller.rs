use std::{path::PathBuf, process::Command, sync::Arc};
use tokio::sync::oneshot;

use crate::{
    core::{app::AppState, state_machine::AppCommand, utils::audio::get_recordings_path},
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
    pub state: Arc<AppState>,
    pub previous_app: parking_lot::Mutex<Option<String>>,
    pub app_handle: AppHandle,
    pub recording_service: Arc<Mutex<AudioRecordingService>>,
    pub transcription_service: Arc<Mutex<TextTranscriptionService>>,
    pub cancellation_token: Arc<AtomicBool>,
    pub is_processing: Arc<AtomicBool>,
}

impl AudioPipelineController {
    pub fn new(state: Arc<AppState>, app_handle: AppHandle) -> Self {
        let recording_service = AudioRecordingService::new();
        recording_service.audio_check();

        let recording_service_mutex = Arc::new(Mutex::new(recording_service));

        let resource_dir = app_handle
            .path()
            .resolve("models/whisper-base", BaseDirectory::Resource)
            .ok();

        log::info!("Using model directory: {:?}", resource_dir);

        let transcription_service =
            match TextTranscriptionService::new(resource_dir, Some(app_handle.clone())) {
                Ok(t) => Arc::new(Mutex::new(t)),
                Err(e) => {
                    log::error!("Failed to create transcriber with custom path: {}", e);

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
                                    recording_service: recording_service_mutex,
                                    transcription_service: Arc::new(Mutex::new(t)),
                                    cancellation_token: Arc::new(AtomicBool::new(false)),
                                    is_processing: Arc::new(AtomicBool::new(false)),
                                };
                            }
                        }
                    }

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
            recording_service: recording_service_mutex,
            transcription_service,
            cancellation_token: Arc::new(AtomicBool::new(false)),
            is_processing: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn get_frontmost_app_name() -> Option<String> {
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
        log::info!("Activating app: {}", app_name);
        Command::new("osascript")
            .arg("-e")
            .arg(format!(r#"tell application "{}" to activate"#, app_name))
            .output()
            .ok();
    }

    pub fn is_processing(&self) -> bool {
        self.is_processing.load(Ordering::SeqCst)
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.load(Ordering::SeqCst)
    }

    pub fn signal_cancellation(&self) {
        log::info!("Cancellation signal received");
        self.cancellation_token.store(true, Ordering::SeqCst);
        self.is_processing.store(false, Ordering::SeqCst);
    }

    pub fn force_stop(&self) -> Result<(), anyhow::Error> {
        log::info!("Force stopping AudioPipelineController");

        self.cancellation_token.store(true, Ordering::SeqCst);
        self.is_processing.store(false, Ordering::SeqCst);

        {
            let recording_service = self.recording_service.lock();
            let _ = recording_service.force_stop();
        }

        if let Some(app) = self.previous_app.lock().take() {
            Self::activate_app(&app);
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

        Ok(())
    }

    pub async fn cancel(&self) {
        log::info!("Cancelling audio processing pipeline");

        self.cancellation_token.store(true, Ordering::SeqCst);
        self.is_processing.store(false, Ordering::SeqCst);

        if let Some(state_machine) = &*self.state.state_machine.lock() {
            state_machine.send_command(AppCommand::Cancel);
            return;
        }

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

        if let Some(app) = self.previous_app.lock().take() {
            Self::activate_app(&app);
        }
    }

    pub fn cancel_sync(&self) {
        log::info!("Synchronous cancellation requested");

        self.signal_cancellation();

        if let Some(state_machine) = &*self.state.state_machine.lock() {
            state_machine.send_command(AppCommand::Cancel);
            return;
        }

        let controller_copy = self.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                controller_copy.cancel().await;
            });
        });
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting audio pipeline recording");

        self.cancellation_token.store(false, Ordering::SeqCst);
        self.is_processing.store(true, Ordering::SeqCst);

        if let Some(app_name) = Self::get_frontmost_app_name() {
            *self.previous_app.lock() = Some(app_name.clone());
            log::info!("Previous app: {}", app_name);
        }

        if let Some(window) = self.app_handle.get_webview_window("main") {
            window.show()?;
            window.set_focus()?;
        }

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

        if let Some(state_machine) = &*self.state.state_machine.lock() {
            match &result {
                Ok(_) => {
                    state_machine.send_command(AppCommand::EmitStatus("recording".to_string()));
                }
                Err(e) => {
                    log::error!("Failed to start recording: {}", e);
                    state_machine.send_command(AppCommand::EmitStatus(format!("error: {}", e)));

                    // Transition back to idle
                    state_machine.send_command(AppCommand::Cancel);
                }
            }
        } else {
            if let Some(window) = self.app_handle.get_webview_window("main") {
                match &result {
                    Ok(_) => {
                        window.emit(
                            "audio-processing-status",
                            ProcessingStatus::Recording.as_str(),
                        )?;
                    }
                    Err(e) => {
                        window.emit(
                            "audio-processing-status",
                            ProcessingStatus::Error(e.to_string()).as_str(),
                        )?;
                    }
                }
            }
        }

        result.map_err(|e| e.into())
    }

    pub async fn stop(&self) {
        log::info!("Stopping recording and starting processing");

        let controller = self.clone();
        let app_handle = self.app_handle.clone();
        let state = Arc::clone(&self.state);

        std::thread::spawn(move || {
            log::info!("Processing in separate thread");

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.emit_to("main", "audio-processing-status", "transcribing");
                }

                let temp_path = get_recordings_path(&app_handle).join("rune_recording.wav");

                if controller.is_cancelled() {
                    log::info!("Cancellation detected during stop preparation");
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.emit_to("main", "audio-processing-status", "cancelled");
                    }

                    if let Some(app) = controller.previous_app.lock().take() {
                        Self::activate_app(&app);
                    }

                    controller
                        .is_processing
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                    return;
                }

                let recording_result = {
                    let recording_service = controller.recording_service.lock();
                    recording_service.stop_recording(temp_path.clone()).await
                };

                if let Err(e) = recording_result {
                    log::error!("Failed to stop recording: {}", e);

                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.emit_to(
                            "main",
                            "audio-processing-status",
                            format!("error: Failed to stop recording: {}", e),
                        );
                    }

                    controller
                        .is_processing
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                    return;
                }

                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.emit_to("main", "audio-processing-status", "transcribing");
                }

                if controller.is_cancelled() {
                    log::info!("Processing cancelled during transcription setup");
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.emit_to("main", "audio-processing-status", "cancelled");
                    }

                    controller
                        .is_processing
                        .store(false, std::sync::atomic::Ordering::SeqCst);

                    if let Some(app) = controller.previous_app.lock().take() {
                        Self::activate_app(&app);
                    }
                    return;
                }

                if !temp_path.exists() {
                    log::error!("No recording file found to transcribe");
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.emit_to(
                            "main",
                            "audio-processing-status",
                            "error: No recording found to transcribe",
                        );
                    }
                    controller
                        .is_processing
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                    return;
                }

                let app_name = controller.previous_app.lock().clone().unwrap_or_default();
                let (tx, rx) = oneshot::channel();
                let temp_path_clone = temp_path.clone();
                let transcription_service = controller.transcription_service.clone();
                let cancellation_token = controller.cancellation_token.clone();

                std::thread::spawn(move || {
                    log::info!("Starting transcription in separate thread");

                    let transcription_result = {
                        let mut transcription_service_guard = transcription_service.lock();
                        transcription_service_guard.transcribe(temp_path_clone)
                    };

                    if cancellation_token.load(std::sync::atomic::Ordering::SeqCst) {
                        log::info!("Transcription cancelled");
                        let _ = tx.send(Err(anyhow::anyhow!("Processing cancelled")));
                        return;
                    }

                    let _ = tx.send(transcription_result.map_err(|e| anyhow::anyhow!("{}", e)));
                    log::info!("Transcription thread completed");
                });

                let transcription_result: Result<Vec<std::string::String>, anyhow::Error> =
                    match rx.await {
                        Ok(result) => result,
                        Err(e) => {
                            log::error!("Failed to receive transcription result: {}", e);
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit_to(
                                    "main",
                                    "audio-processing-status",
                                    format!("error: Transcription failed: {}", e),
                                );
                            }
                            controller
                                .is_processing
                                .store(false, std::sync::atomic::Ordering::SeqCst);
                            return;
                        }
                    };

                if controller.is_cancelled() {
                    log::info!("Processing cancelled after transcription");
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.emit_to("main", "audio-processing-status", "cancelled");
                    }

                    controller
                        .is_processing
                        .store(false, std::sync::atomic::Ordering::SeqCst);

                    if let Some(app) = controller.previous_app.lock().take() {
                        Self::activate_app(&app);
                    }
                    return;
                }

                match transcription_result {
                    Ok(transcription) => {
                        if let Some(text) = transcription.first() {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit_to(
                                    "main",
                                    "audio-processing-status",
                                    "thinking_action",
                                );
                            }

                            let text_clone = text.clone();
                            let app_name_clone = app_name.clone();
                            let state = controller.state.clone();

                            let process_thread = std::thread::spawn(move || {
                                let rt = tokio::runtime::Builder::new_current_thread()
                                    .enable_all()
                                    .build()
                                    .unwrap();

                                rt.block_on(async {
                                    TextProcessingService::process_text(
                                        &state,
                                        &app_name_clone,
                                        &text_clone,
                                    )
                                    .await
                                })
                            });

                            let processed_text_result = process_thread.join().unwrap_or_else(|e| {
                                log::error!("Failed to join text processing thread: {:?}", e);
                                Err(anyhow::anyhow!("Thread panic during text processing"))
                            });

                            if controller.is_cancelled() {
                                log::info!("Processing cancelled after text processing");
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    let _ = window.emit_to(
                                        "main",
                                        "audio-processing-status",
                                        "cancelled",
                                    );
                                }
                                controller
                                    .is_processing
                                    .store(false, std::sync::atomic::Ordering::SeqCst);
                                if let Some(app) = controller.previous_app.lock().take() {
                                    Self::activate_app(&app);
                                }
                                return;
                            }

                            match processed_text_result {
                                Ok(processed_text) => {
                                    if let Some(app) = controller.previous_app.lock().take() {
                                        Self::activate_app(&app);
                                    }

                                    if let Err(e) =
                                        TextProcessingService::inject_text(&processed_text)
                                    {
                                        log::error!("Failed to inject text: {}", e);
                                    }

                                    if let Err(e) =
                                        TextTranscriptHistoryService::save_processed_text(
                                            &app_handle,
                                            &processed_text,
                                        )
                                    {
                                        log::error!(
                                            "Failed to save processed text to history: {}",
                                            e
                                        );
                                    }

                                    if let Some(window) = app_handle.get_webview_window("main") {
                                        let _ = window.emit_to(
                                            "main",
                                            "audio-processing-status",
                                            "completed",
                                        );
                                        let _ = window.hide();
                                    }

                                    if let Some(_history_window) =
                                        app_handle.get_webview_window("history")
                                    {
                                        let _ = app_handle.emit("refresh-history", ());
                                    }
                                }
                                Err(e) => {
                                    log::error!("Text processing error: {}", e);
                                    if controller.is_cancelled() {
                                        if let Some(window) = app_handle.get_webview_window("main")
                                        {
                                            let _ = window.emit_to(
                                                "main",
                                                "audio-processing-status",
                                                "cancelled",
                                            );
                                        }
                                    } else {
                                        if let Some(app) = controller.previous_app.lock().take() {
                                            Self::activate_app(&app);
                                        }
                                        if let Err(e) = TextProcessingService::inject_text(text) {
                                            log::error!("Failed to inject original text: {}", e);
                                        }
                                        if let Some(window) = app_handle.get_webview_window("main")
                                        {
                                            let _ = window.emit_to(
                                                "main",
                                                "audio-processing-status",
                                                format!("error: Processing failed: {}", e),
                                            );
                                        }
                                    }
                                }
                            }
                            controller
                                .is_processing
                                .store(false, std::sync::atomic::Ordering::SeqCst);
                        } else {
                            log::error!("No transcription text available");
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit_to(
                                    "main",
                                    "audio-processing-status",
                                    "error: No text transcribed",
                                );
                            }
                            controller
                                .is_processing
                                .store(false, std::sync::atomic::Ordering::SeqCst);
                        }
                    }
                    Err(e) => {
                        log::error!("Transcription error: {}", e);
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.emit_to(
                                "main",
                                "audio-processing-status",
                                format!("error: Transcription failed: {}", e),
                            );
                        }
                        controller
                            .is_processing
                            .store(false, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            });
            controller
                .cancellation_token
                .store(false, std::sync::atomic::Ordering::SeqCst);
            controller
                .is_processing
                .store(false, std::sync::atomic::Ordering::SeqCst);

            // Force a state machine reset
            if let Some(state_machine) = &*state.state_machine.lock() {
                state_machine.send_command(AppCommand::EmitStatus("idle".to_string()));

                state_machine.send_command(AppCommand::PurgeResources);
            }

            log::info!("Audio pipeline processing complete and state reset");
        });
    }

    pub fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            previous_app: parking_lot::Mutex::new(self.previous_app.lock().clone()),
            app_handle: self.app_handle.clone(),
            recording_service: Arc::clone(&self.recording_service),
            transcription_service: Arc::clone(&self.transcription_service),
            cancellation_token: Arc::clone(&self.cancellation_token),
            is_processing: Arc::clone(&self.is_processing),
        }
    }

    pub fn get_recording_service(&self) -> MutexGuard<AudioRecordingService> {
        self.recording_service.lock()
    }

    pub fn get_transcription_service(&self) -> MutexGuard<TextTranscriptionService> {
        self.transcription_service.lock()
    }
}

unsafe impl Send for AudioPipelineController {}
unsafe impl Sync for AudioPipelineController {}
