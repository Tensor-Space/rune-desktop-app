use std::sync::{Arc, Mutex};

pub mod devices;
pub mod recorder;
pub mod transcriber;

pub use devices::AudioDevice;
pub use recorder::AudioRecorder;
pub use transcriber::AudioTranscriber;

pub struct AudioState {
    pub recorder: Arc<AudioRecorder>,
    pub transcriber: Arc<Mutex<AudioTranscriber>>,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            recorder: Arc::new(AudioRecorder::default()),
            transcriber: Arc::new(Mutex::new(AudioTranscriber::new().unwrap())),
        }
    }
}
