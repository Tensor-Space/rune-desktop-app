use crate::{audio::AudioState, config::Settings};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct AppState {
    pub settings: Arc<RwLock<Settings>>,
    pub audio: Arc<AudioState>,
}

impl AppState {
    pub fn new(settings: Settings, audio: Arc<AudioState>) -> Self {
        Self {
            settings: Arc::new(RwLock::new(settings)),
            audio,
        }
    }
}
