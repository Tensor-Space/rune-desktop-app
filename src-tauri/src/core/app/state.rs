use crate::{
    audio::{AudioRecorder, AudioTranscriber},
    core::config::Settings,
};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

pub struct AppState {
    pub settings: Arc<RwLock<Settings>>,
    pub transcriber: Arc<Mutex<AudioTranscriber>>,
    pub recorder: Arc<Mutex<AudioRecorder>>,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(settings)),
            transcriber: Arc::new(Mutex::new(AudioTranscriber::new().unwrap())),
            recorder: Arc::new(Mutex::new(AudioRecorder::new())),
        }
    }
}
