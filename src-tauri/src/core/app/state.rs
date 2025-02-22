use crate::{
    audio::{AudioRecorder, AudioTranscriber},
    core::config::Settings,
};
use parking_lot::{Mutex, RwLock};
use rune_llm::{LLMClient, LLMProvider};
use std::sync::Arc;

pub struct AppState {
    pub settings: Arc<RwLock<Settings>>,
    pub transcriber: Arc<Mutex<AudioTranscriber>>,
    pub recorder: Arc<Mutex<AudioRecorder>>,
    pub llm_client: Arc<Mutex<LLMClient>>,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(settings)),
            transcriber: Arc::new(Mutex::new(AudioTranscriber::new().unwrap())),
            recorder: Arc::new(Mutex::new(AudioRecorder::new())),
            llm_client: Arc::new(Mutex::new(LLMClient::new(
                LLMProvider::OpenAI,
                "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".to_string(),
                None,
                None,
            ))),
        }
    }
}
