use anyhow::Result;
use rune_whisper::{Whisper, WhisperConfig};
use std::path::PathBuf;

pub struct AudioTranscriber {
    whisper: Whisper, // Store the Whisper instance
}

impl AudioTranscriber {
    pub fn new() -> Result<Self> {
        let config = WhisperConfig::default();
        let whisper = Whisper::new(config)?;
        Ok(Self { whisper })
    }

    pub fn transcribe(&mut self, audio_path: PathBuf) -> Result<Vec<String>> {
        self.whisper.transcribe(audio_path)
    }
}
