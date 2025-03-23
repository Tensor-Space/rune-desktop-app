use crate::{
    audio::{manager::AudioManager, AudioRecorder, WhisperTranscriber},
    core::app::AppState,
    events::{
        emitter::EventEmitter,
        types::{RecordingCommand, ServiceEvent},
    },
    text::text_processor_pipeline::TextProcessorPipeline,
};
use parking_lot::Mutex;
use std::{process::Command, sync::Arc};
use tauri::{path::BaseDirectory, AppHandle, Manager};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex as AsyncMutex,
};

pub struct RecordingService {
    state: Arc<AppState>,
    app_handle: AppHandle,
    recorder: Arc<Mutex<AudioRecorder>>,
    transcriber: Arc<Mutex<WhisperTranscriber>>,
    previous_app: Mutex<Option<String>>,
    command_rx: AsyncMutex<Option<Receiver<RecordingCommand>>>,
    event_emitter: EventEmitter,
    internal_event_tx: Sender<ServiceEvent>,
    internal_event_rx: AsyncMutex<Option<Receiver<ServiceEvent>>>,
}

impl RecordingService {
    pub fn new(state: Arc<AppState>, app_handle: AppHandle) -> Self {
        let (internal_tx, internal_rx) = mpsc::channel(100);

        let recorder = Arc::new(Mutex::new(AudioRecorder::new()));

        // Initialize the WhisperTranscriber with the model path
        let model_dir = match app_handle
            .path()
            .resolve("models/whisper-base", BaseDirectory::Resource)
        {
            Ok(dir) => dir,
            Err(e) => {
                log::error!("Failed to resolve models directory: {}", e);
                panic!("Cannot resolve model directory: {}", e);
            }
        };

        let transcriber = WhisperTranscriber::new(Some(model_dir.clone()))
            .map(|t| Arc::new(Mutex::new(t)))
            .unwrap_or_else(|e| {
                log::error!("Failed to create transcriber: {}", e);
                panic!("Cannot initialize transcriber: {}", e);
            });

        // Initialize the model after creation
        tauri::async_runtime::block_on(async {
            if let Err(e) = transcriber.lock().initialize().await {
                log::error!("Failed to initialize Whisper model: {}", e);
                log::warn!(
                    "Make sure the model files are available at: {:?}",
                    model_dir
                );
                log::warn!("Download from: https://huggingface.co/openai/whisper-small");
            }
        });

        Self {
            state,
            app_handle: app_handle.clone(),
            recorder,
            transcriber,
            previous_app: Mutex::new(None),
            command_rx: AsyncMutex::new(None),
            event_emitter: EventEmitter::new(app_handle),
            internal_event_tx: internal_tx,
            internal_event_rx: AsyncMutex::new(Some(internal_rx)),
        }
    }

    pub fn create_channels() -> (Sender<RecordingCommand>, Receiver<RecordingCommand>) {
        mpsc::channel(100)
    }

    pub async fn set_command_receiver(&self, rx: Receiver<RecordingCommand>) {
        *self.command_rx.lock().await = Some(rx);
    }

    pub async fn run(&self) {
        let event_processor = self.process_internal_events();

        let command_processor = self.process_commands();

        tokio::select! {
            _ = event_processor => log::info!("Event processor completed"),
            _ = command_processor => log::info!("Command processor completed"),
        }
    }

    async fn process_commands(&self) {
        let mut rx = match self.command_rx.lock().await.take() {
            Some(rx) => rx,
            None => return,
        };

        while let Some(command) = rx.recv().await {
            match command {
                RecordingCommand::Start => {
                    self.handle_start_recording().await;
                }
                RecordingCommand::Stop => {
                    self.handle_stop_recording().await;
                }
                RecordingCommand::SetDevice(device_id) => {
                    self.handle_set_device(device_id);
                }
                RecordingCommand::ProcessText { app_name, text } => {
                    self.handle_process_text(app_name, text).await;
                }
            }
        }
    }

    async fn process_internal_events(&self) {
        let mut rx = match self.internal_event_rx.lock().await.take() {
            Some(rx) => rx,
            None => return,
        };

        while let Some(event) = rx.recv().await {
            match event {
                ServiceEvent::RecordingStarted => {
                    self.event_emitter.emit_recording_started();
                }
                ServiceEvent::RecordingStopped => {
                    self.event_emitter.emit_recording_stopped();
                }
                ServiceEvent::TranscriptionReceived { text, is_final } => {
                    self.event_emitter.emit_transcription(&text, is_final);
                }
                ServiceEvent::AudioLevelsUpdated { levels } => {
                    self.event_emitter.emit_audio_levels(levels);
                }
                ServiceEvent::TextProcessed { text } => {
                    self.event_emitter.emit_text_processed(&text);
                }
                ServiceEvent::Error { message } => {
                    self.event_emitter.emit_error("recording_error", &message);
                }
                ServiceEvent::ProcessingCompleted => {
                    self.event_emitter.emit_processing_completed();
                }
            }
        }
    }

    async fn handle_start_recording(&self) {
        if let Some(app_name) = Self::get_frontmost_app_name() {
            *self.previous_app.lock() = Some(app_name);
        }

        if let Some(window) = self.app_handle.get_webview_window("main") {
            if let Err(e) = window.show() {
                log::error!("Failed to show window: {}", e);
            }
        }

        let settings = self.state.settings.read().clone();
        let device_id = settings.audio.default_device.clone();
        {
            let recorder = self.recorder.lock();
            recorder.set_device_id(device_id);
            recorder.set_app_handle(self.app_handle.clone());
        }

        let recorder = self.recorder.lock();
        match recorder.start_recording(&self.app_handle).await {
            Ok(_) => {
                let _ = self
                    .internal_event_tx
                    .send(ServiceEvent::RecordingStarted)
                    .await;
            }
            Err(e) => {
                let _ = self
                    .internal_event_tx
                    .send(ServiceEvent::Error {
                        message: format!("Failed to start recording: {}", e),
                    })
                    .await;
            }
        }
    }

    async fn handle_stop_recording(&self) {
        let audio_manager = AudioManager::new();
        let temp_path = audio_manager
            .get_recordings_path(&self.app_handle)
            .join("rune_recording.wav");

        if let Err(e) = self.recorder.lock().stop_recording(temp_path.clone()).await {
            log::error!("Failed to stop recording: {}", e);
            let _ = self
                .internal_event_tx
                .send(ServiceEvent::Error {
                    message: format!("Failed to stop recording: {}", e),
                })
                .await;
            return;
        }

        let _ = self
            .internal_event_tx
            .send(ServiceEvent::RecordingStopped)
            .await;
        self.event_emitter.emit_processing_started();

        // Get the previous app name
        let app_name = {
            let guard = self.previous_app.lock();
            guard.clone().unwrap_or_default()
        };

        // Start transcription process
        log::info!("Starting transcription of recording: {:?}", temp_path);

        let transcription_result = {
            let transcriber = self.transcriber.lock();
            transcriber.transcribe_file(temp_path.clone()).await
        };

        if let Err(e) = transcription_result {
            log::error!("Failed to start transcription: {}", e);
            let _ = self
                .internal_event_tx
                .send(ServiceEvent::Error {
                    message: format!("Failed to start transcription: {}", e),
                })
                .await;
            return;
        }

        // Set up a listener for transcription completion
        let transcriber_clone = self.transcriber.clone();
        let app_handle_clone = self.app_handle.clone();
        let internal_event_tx_clone = self.internal_event_tx.clone();
        let app_name_clone = app_name.to_string();
        let state_clone = self.state.clone();

        // Create a separate task to check for transcription completion
        std::thread::spawn(move || {
            // Run the async code in a new runtime
            tauri::async_runtime::block_on(async {
                let mut completed = false;
                let mut last_status_log = std::time::Instant::now();

                while !completed {
                    // Periodically log status updates
                    let now = std::time::Instant::now();
                    if now.duration_since(last_status_log).as_secs() >= 5 {
                        log::info!("Still processing transcription...");
                        last_status_log = now;
                    }

                    // Check for new messages
                    let message = {
                        let transcriber_guard = transcriber_clone.lock();
                        transcriber_guard.try_receive_message()
                    };

                    if let Some(message) = message {
                        match message {
                            crate::audio::whisper_transcriber::TranscriptionMessage::Final(
                                text,
                            ) => {
                                log::info!("Received final transcription: {}", text);

                                // Send the transcription result
                                let _ = internal_event_tx_clone
                                    .send(ServiceEvent::TranscriptionReceived {
                                        text: text.clone(),
                                        is_final: true,
                                    })
                                    .await;

                                // Process the text
                                match TextProcessorPipeline::process_text(
                                    &state_clone,
                                    &app_name_clone,
                                    &text,
                                )
                                .await
                                {
                                    Ok(processed_text) => {
                                        // Activate previous app
                                        Self::activate_app(&app_name_clone);

                                        // Inject processed text
                                        if let Err(e) =
                                            TextProcessorPipeline::inject_text(&processed_text)
                                        {
                                            log::error!("Failed to inject text: {}", e);
                                            let _ = internal_event_tx_clone
                                                .send(ServiceEvent::Error {
                                                    message: format!(
                                                        "Failed to inject text: {}",
                                                        e
                                                    ),
                                                })
                                                .await;
                                        } else {
                                            let _ = internal_event_tx_clone
                                                .send(ServiceEvent::TextProcessed {
                                                    text: processed_text,
                                                })
                                                .await;
                                        }

                                        // Hide the window
                                        if let Some(window) =
                                            app_handle_clone.get_webview_window("main")
                                        {
                                            if let Err(e) = window.hide() {
                                                log::error!("Failed to hide window: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Text processing error: {}", e);

                                        // If processing fails, inject the original text
                                        if let Some(app) = app_name_clone.clone().into() {
                                            Self::activate_app(&app);
                                        }

                                        if let Err(inject_err) =
                                            TextProcessorPipeline::inject_text(&text)
                                        {
                                            log::error!(
                                                "Failed to inject original text: {}",
                                                inject_err
                                            );
                                        }

                                        let _ = internal_event_tx_clone
                                            .send(ServiceEvent::Error {
                                                message: format!("Processing failed: {}", e),
                                            })
                                            .await;
                                    }
                                };
                                completed = true;
                            }
                            crate::audio::whisper_transcriber::TranscriptionMessage::Error(err) => {
                                log::error!("Transcription error: {}", err);
                                let _ = internal_event_tx_clone
                                    .send(ServiceEvent::Error { message: err })
                                    .await;
                                completed = true;
                            }
                            crate::audio::whisper_transcriber::TranscriptionMessage::Complete => {
                                log::info!("Transcription complete");
                                completed = true;
                            }
                            _ => {
                                // Handle interim updates if needed
                            }
                        }
                    }

                    // Short delay to prevent high CPU usage
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    // Break the loop if processing completed
                    if !transcriber_clone.lock().is_processing() && completed {
                        break;
                    }
                }

                // Final processing complete event
                let _ = internal_event_tx_clone
                    .send(ServiceEvent::ProcessingCompleted)
                    .await;
            });
        });
    }

    fn handle_set_device(&self, device_id: String) {
        let recorder = self.recorder.lock();
        recorder.set_device_id(Some(device_id));
        log::info!("Device set successfully");
    }

    async fn handle_process_text(&self, app_name: String, text: String) {
        self.process_and_inject_text(&app_name, &text).await;
    }

    async fn process_and_inject_text(&self, app_name: &str, text: &str) {
        match TextProcessorPipeline::process_text(&self.state, app_name, text).await {
            Ok(processed_text) => {
                if let Some(app) = self.previous_app.lock().take() {
                    Self::activate_app(&app);
                }

                if let Err(e) = TextProcessorPipeline::inject_text(&processed_text) {
                    log::error!("Failed to inject text: {}", e);
                    let _ = self
                        .internal_event_tx
                        .send(ServiceEvent::Error {
                            message: format!("Failed to inject text: {}", e),
                        })
                        .await;
                } else {
                    let _ = self
                        .internal_event_tx
                        .send(ServiceEvent::TextProcessed {
                            text: processed_text,
                        })
                        .await;
                    self.event_emitter.emit_processing_completed();
                }

                if let Some(window) = self.app_handle.get_webview_window("main") {
                    if let Err(e) = window.hide() {
                        log::error!("Failed to hide window: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Text processing error: {}", e);

                if let Some(app) = self.previous_app.lock().take() {
                    Self::activate_app(&app);
                }

                if let Err(inject_err) = TextProcessorPipeline::inject_text(text) {
                    log::error!("Failed to inject original text: {}", inject_err);
                }

                let _ = self
                    .internal_event_tx
                    .send(ServiceEvent::Error {
                        message: format!("Processing failed: {}", e),
                    })
                    .await;
            }
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
}
