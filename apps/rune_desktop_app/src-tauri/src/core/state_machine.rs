use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppStateType {
    Idle,
    Recording,
    Transcribing,
    Processing,
    Error,
}

pub struct StateMachine {
    current_state: Mutex<AppStateType>,
    app_handle: AppHandle,
    command_sender: Sender<AppCommand>,
    command_receiver: Arc<Mutex<Receiver<AppCommand>>>,
}

#[derive(Debug)]
pub enum AppCommand {
    StartRecording,
    StopRecording,
    Cancel,
    PurgeResources,
    EmitStatus(String),
}

impl StateMachine {
    pub fn new(app_handle: AppHandle) -> Arc<Self> {
        let (tx, rx) = unbounded();

        let machine = Arc::new(Self {
            current_state: Mutex::new(AppStateType::Idle),
            app_handle,
            command_sender: tx,
            command_receiver: Arc::new(Mutex::new(rx)),
        });

        Self::start_command_processor(Arc::clone(&machine));

        machine
    }

    fn start_command_processor(machine: Arc<StateMachine>) {
        std::thread::spawn(move || {
            let receiver = machine.command_receiver.lock().clone();

            log::info!("Command processor started");

            while let Ok(command) = receiver.recv() {
                log::info!("Processing command: {:?}", command);

                match command {
                    AppCommand::StartRecording => {
                        let current = *machine.current_state.lock();
                        if current == AppStateType::Idle {
                            let machine_clone = Arc::clone(&machine);

                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Builder::new_current_thread()
                                    .enable_all()
                                    .build()
                                    .unwrap();

                                rt.block_on(async {
                                    if let Some(state) = machine_clone
                                        .app_handle
                                        .try_state::<Arc<crate::core::app::AppState>>()
                                    {
                                        if let Some(pipeline) = &*state.audio_pipeline.lock() {
                                            let _ = pipeline.start().await;
                                        }
                                    }
                                });

                                {
                                    let mut state = machine_clone.current_state.lock();
                                    *state = AppStateType::Recording;
                                }
                                machine_clone.emit_status("recording");
                            });
                        }
                    }
                    AppCommand::StopRecording => {
                        let current = *machine.current_state.lock();
                        if current == AppStateType::Recording {
                            let machine_clone = Arc::clone(&machine);

                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Builder::new_current_thread()
                                    .enable_all()
                                    .build()
                                    .unwrap();

                                rt.block_on(async {
                                    if let Some(state) = machine_clone
                                        .app_handle
                                        .try_state::<Arc<crate::core::app::AppState>>()
                                    {
                                        if let Some(pipeline) = &*state.audio_pipeline.lock() {
                                            pipeline.stop().await;
                                        }
                                    }
                                });

                                {
                                    let mut state = machine_clone.current_state.lock();
                                    *state = AppStateType::Transcribing;
                                }
                                machine_clone.emit_status("transcribing");

                                std::thread::sleep(std::time::Duration::from_secs(5));

                                if *machine_clone.current_state.lock() != AppStateType::Idle {
                                    log::info!("Resetting state machine from lingering state");
                                    *machine_clone.current_state.lock() = AppStateType::Idle;
                                    machine_clone.emit_status("idle");
                                    machine_clone.send_command(AppCommand::PurgeResources);
                                }
                            });
                        }
                    }
                    AppCommand::Cancel => {
                        machine.perform_cancellation();
                        machine.send_command(AppCommand::PurgeResources);
                        *machine.current_state.lock() = AppStateType::Idle;
                        machine.emit_status("cancelled");
                    }
                    AppCommand::PurgeResources => {
                        machine.purge_resources();
                    }
                    AppCommand::EmitStatus(status) => {
                        machine.emit_status(&status);
                    }
                }
            }

            log::warn!("Command processor exited");
        });
    }

    fn perform_cancellation(&self) {
        log::info!("Performing cancellation");

        let app_handle = self.app_handle.clone();
        std::thread::spawn(move || {
            log::info!("Cancellation thread started");

            if let Some(state) = app_handle.try_state::<Arc<crate::core::app::AppState>>() {
                let pipeline_exists = {
                    let lock = state.audio_pipeline.lock();
                    lock.is_some()
                };

                if pipeline_exists {
                    let pipeline: Arc<
                        crate::controllers::audio_pipleine_controller::AudioPipelineController,
                    > = {
                        let lock = state.audio_pipeline.lock();
                        match &*lock {
                            Some(pipeline) => Arc::clone(pipeline),
                            None => return,
                        }
                    };

                    let recording_service = pipeline.get_recording_service();
                    let _ = recording_service.force_stop();

                    if let Some(app_name) = pipeline.previous_app.lock().take() {
                        crate::controllers::audio_pipleine_controller::AudioPipelineController::activate_app(&app_name);
                    }

                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.emit_to("main", "audio-processing-status", "cancelled");
                        let _ = window.hide();
                    }
                }
            }

            log::info!("Cancellation completed");
        });
    }

    fn purge_resources(&self) {
        log::info!("Purging resources");

        let app_handle = self.app_handle.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                if let Some(state) = app_handle.try_state::<Arc<crate::core::app::AppState>>() {
                    let new_pipeline = Arc::new(
                        crate::controllers::audio_pipleine_controller::AudioPipelineController::new(
                            Arc::clone(&state),
                            app_handle.clone(),
                        ),
                    );

                    *state.audio_pipeline.lock() = Some(new_pipeline);
                }
            });

            log::info!("Resources purged");
        });
    }

    fn emit_status(&self, status: &str) {
        if let Some(window) = self.app_handle.get_webview_window("main") {
            let _ = window.emit_to("main", "audio-processing-status", status);
        }
    }

    pub fn send_command(&self, command: AppCommand) {
        let _ = self.command_sender.send(command);
    }

    pub fn get_state(&self) -> AppStateType {
        *self.current_state.lock()
    }
}

unsafe impl Send for StateMachine {}
unsafe impl Sync for StateMachine {}
