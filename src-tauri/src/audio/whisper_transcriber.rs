use crate::core::error::AudioError;
use parking_lot::Mutex;
use rune_whisper_local::{Whisper, WhisperConfig};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub enum TranscriptionMessage {
    Interim(String),
    Final(String),
    Error(String),
    Complete,
}

pub struct WhisperTranscriber {
    model_dir: Option<PathBuf>,
    transcript_sender: Arc<Mutex<Option<UnboundedSender<TranscriptionMessage>>>>,
    transcript_receiver: Arc<Mutex<Option<UnboundedReceiver<TranscriptionMessage>>>>,
    last_transcript: Arc<Mutex<String>>,
    is_processing: Arc<Mutex<bool>>,
    whisper: Arc<Mutex<Option<Whisper>>>,
}

impl WhisperTranscriber {
    pub fn new(model_dir: Option<PathBuf>) -> Result<Self, AudioError> {
        let (transcript_tx, transcript_rx) = unbounded_channel::<TranscriptionMessage>();

        Ok(Self {
            model_dir,
            transcript_sender: Arc::new(Mutex::new(Some(transcript_tx))),
            transcript_receiver: Arc::new(Mutex::new(Some(transcript_rx))),
            last_transcript: Arc::new(Mutex::new(String::new())),
            is_processing: Arc::new(Mutex::new(false)),
            whisper: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn initialize(&self) -> Result<(), AudioError> {
        let model_dir = self.model_dir.clone().ok_or_else(|| {
            AudioError::Transcription("Whisper model directory not specified".to_string())
        })?;

        log::info!("Initializing Whisper model from: {:?}", model_dir);

        // Initialize the model in a background thread to avoid blocking
        let whisper_arc = self.whisper.clone();
        let model_dir_clone = model_dir.clone();

        thread::spawn(move || {
            let config = WhisperConfig::new(Some(model_dir_clone.clone()));

            match Whisper::new(config) {
                Ok(whisper) => {
                    log::info!(
                        "Whisper model loaded successfully from {:?}",
                        model_dir_clone
                    );
                    *whisper_arc.lock() = Some(whisper);
                }
                Err(e) => {
                    log::error!("Failed to load Whisper model: {}", e);
                    log::warn!("Make sure model files (model.safetensors, config.json, tokenizer.json) exist at: {:?}", model_dir_clone);
                }
            }
        });

        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.whisper.lock().is_some()
    }

    pub async fn transcribe_file(&self, audio_path: PathBuf) -> Result<Vec<String>, AudioError> {
        // First set the processing flag
        {
            *self.is_processing.lock() = true;
            *self.last_transcript.lock() = String::new();
        }

        // Create new channels
        let (tx, rx) = unbounded_channel::<TranscriptionMessage>();
        *self.transcript_sender.lock() = Some(tx.clone());
        *self.transcript_receiver.lock() = Some(rx);

        log::info!("Starting transcription of file: {:?}", audio_path);

        // Wait for model to initialize if needed
        let max_wait_seconds = 10;
        for i in 0..max_wait_seconds {
            if self.is_initialized() {
                break;
            }

            log::info!(
                "Waiting for Whisper model to initialize ({}/{})",
                i + 1,
                max_wait_seconds
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            if i == max_wait_seconds - 1 && !self.is_initialized() {
                *self.is_processing.lock() = false;
                return Err(AudioError::Transcription(
                    "Timed out waiting for Whisper model to initialize".to_string(),
                ));
            }
        }

        // Clone what we need for the thread
        let whisper_clone = self.whisper.clone();
        let is_processing_clone = self.is_processing.clone();
        let last_transcript_clone = self.last_transcript.clone();
        let tx_clone = tx.clone();
        let audio_path_clone = audio_path.clone();

        // Process in background thread
        thread::spawn(move || {
            let result = {
                let mut whisper_guard = whisper_clone.lock();
                if let Some(whisper) = whisper_guard.as_mut() {
                    log::info!("Processing audio with Whisper...");
                    whisper.transcribe(audio_path_clone.clone())
                } else {
                    Err(anyhow::anyhow!("Whisper model not initialized"))
                }
            };

            match result {
                Ok(transcriptions) => {
                    if !transcriptions.is_empty() {
                        // Get the full transcription text
                        let full_text = transcriptions.join(" ");
                        log::info!("Transcription successful: {}", full_text);

                        // Update last transcript
                        *last_transcript_clone.lock() = full_text.clone();

                        // Send the final result
                        let _ = tx_clone.send(TranscriptionMessage::Final(full_text));
                    } else {
                        log::warn!("Transcription returned no results");
                        // Send empty result if no transcription
                        let _ = tx_clone.send(TranscriptionMessage::Final(String::new()));
                    }
                }
                Err(e) => {
                    let error_msg = format!("Transcription failed: {}", e);
                    log::error!("{}", error_msg);
                    let _ = tx_clone.send(TranscriptionMessage::Error(error_msg));
                }
            }

            // Mark processing as complete
            *is_processing_clone.lock() = false;
            let _ = tx_clone.send(TranscriptionMessage::Complete);
        });

        // Return empty vec - we'll get results asynchronously
        Ok(Vec::new())
    }

    pub fn try_receive_message(&self) -> Option<TranscriptionMessage> {
        if let Some(receiver) = self.transcript_receiver.lock().as_mut() {
            receiver.try_recv().ok()
        } else {
            None
        }
    }

    pub fn get_last_transcript(&self) -> String {
        self.last_transcript.lock().clone()
    }

    pub fn is_processing(&self) -> bool {
        *self.is_processing.lock()
    }
}
