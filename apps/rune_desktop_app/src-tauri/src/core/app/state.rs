use crate::core::state_machine::{AppCommand, StateMachine};
use crate::{
    controllers::audio_pipleine_controller::AudioPipelineController, core::config::Settings,
};
use parking_lot::{Mutex, RwLock};
use rune_llm::{LLMClient, LLMProvider};
use std::sync::Arc;
use tauri::AppHandle;
use tokio::runtime::Runtime;

pub struct AppState {
    pub settings: Arc<RwLock<Settings>>,
    pub llm: Arc<Mutex<Option<LLMClient>>>,
    pub audio_pipeline: Arc<Mutex<Option<Arc<AudioPipelineController>>>>,
    pub runtime: Runtime,
    pub state_machine: Arc<Mutex<Option<Arc<StateMachine>>>>,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        // Create a multi-threaded runtime for async operations
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");

        Self {
            settings: Arc::new(RwLock::new(settings)),
            llm: Arc::new(Mutex::new(None)), // Initialize as None
            audio_pipeline: Arc::new(Mutex::new(None)),
            runtime,
            state_machine: Arc::new(Mutex::new(None)),
        }
    }

    pub fn init_state_machine(&self, app_handle: AppHandle) {
        let machine = StateMachine::new(app_handle);
        *self.state_machine.lock() = Some(machine);
    }

    pub fn init_llm_client(&self, openai_api_key: Option<String>) {
        let openai_key = openai_api_key.unwrap_or_else(|| "".to_string());

        let llm_client = LLMClient::new(LLMProvider::OpenAI, openai_key, None, None);

        *self.llm.lock() = Some(llm_client);
    }

    pub fn cancel_current_operation(&self) {
        if let Some(machine) = &*self.state_machine.lock() {
            machine.send_command(AppCommand::Cancel);
        }
    }
}

impl AppState {
    pub fn execute_async<F, T>(&self, future: F) -> tokio::task::JoinHandle<T>
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::spawn(future)
    }

    pub fn execute_blocking<F, T>(&self, func: F) -> std::thread::JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        std::thread::spawn(func)
    }
}
