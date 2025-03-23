use crate::{
    controllers::audio_pipleine_controller::AudioPipelineController, core::config::Settings,
};
use parking_lot::{Mutex, RwLock};
use rune_llm::{LLMClient, LLMProvider};
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct AppState {
    pub settings: Arc<RwLock<Settings>>,
    pub llm: Arc<Mutex<LLMClient>>,
    pub audio_pipeline: Arc<Mutex<Option<Arc<AudioPipelineController>>>>,
    pub runtime: Runtime,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        // Create a multi-threaded runtime for async operations
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");

        Self {
            settings: Arc::new(RwLock::new(settings)),
            llm: Arc::new(Mutex::new(LLMClient::new(
                LLMProvider::OpenAI,
                "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".to_string(),
                None,
                None,
            ))),
            audio_pipeline: Arc::new(Mutex::new(None)),
            runtime,
        }
    }
}
