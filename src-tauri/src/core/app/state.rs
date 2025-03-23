use crate::{core::app::settings::Settings, events::types::RecordingCommand};
use parking_lot::{Mutex, RwLock};
use rune_llm::{LLMClient, LLMProvider};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Mutex as AsyncMutex};

pub struct AppState {
    pub settings: Arc<RwLock<Settings>>,
    pub llm_client: Arc<Mutex<LLMClient>>,
    pub recording_tx: Arc<AsyncMutex<Option<Sender<RecordingCommand>>>>,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(settings)),
            llm_client: Arc::new(Mutex::new(LLMClient::new(
                LLMProvider::OpenAI,
                "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".to_string(),
                None,
                None,
            ))),
            recording_tx: Arc::new(AsyncMutex::new(None)),
        }
    }
}
