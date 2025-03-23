use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc::{self, unbounded_channel};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::core::error::AudioError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeepgramConfig {
    pub api_key: String,
    pub model: String,
    pub language: String,
    pub punctuate: bool,
    pub smart_format: bool,
    pub interim_results: bool,
}

impl Default for DeepgramConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "nova-2".to_string(),
            language: "en".to_string(),
            punctuate: true,
            smart_format: true,
            interim_results: true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct DeepgramResponse {
    channel: DeepgramChannel,
    #[serde(default)]
    is_final: bool,
    #[serde(default)]
    speech_final: bool,
}

#[derive(Debug, Deserialize)]
struct DeepgramChannel {
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Deserialize)]
struct DeepgramAlternative {
    transcript: String,
}

#[derive(Debug, Clone)]
pub enum TranscriptionMessage {
    Interim(String),
    Final(String),
    Error(String),
    Complete,
}

pub struct AudioTranscriber {
    config: DeepgramConfig,
    ws_sender: Arc<StdMutex<Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>>>>,
    transcript_receiver:
        Arc<StdMutex<Option<tokio::sync::mpsc::UnboundedReceiver<TranscriptionMessage>>>>,
    last_transcript: Arc<StdMutex<String>>,
    is_streaming: Arc<StdMutex<bool>>,
    connection_attempts: Arc<StdMutex<u32>>,
}

impl AudioTranscriber {
    pub fn new(_model_dir: Option<PathBuf>) -> Result<Self, AudioError> {
        Ok(Self {
            config: DeepgramConfig::default(),
            ws_sender: Arc::new(StdMutex::new(None)),
            transcript_receiver: Arc::new(StdMutex::new(None)),
            last_transcript: Arc::new(StdMutex::new(String::new())),
            is_streaming: Arc::new(StdMutex::new(false)),
            connection_attempts: Arc::new(StdMutex::new(0)),
        })
    }

    pub fn set_config(&mut self, config: DeepgramConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> DeepgramConfig {
        self.config.clone()
    }

    pub async fn start_streaming(&self) -> Result<(), AudioError> {
        if self.config.api_key.is_empty() {
            return Err(AudioError::Transcription(
                "Deepgram API key is not set".to_string(),
            ));
        }

        // First, make sure we're not already streaming
        {
            let mut is_streaming = self.is_streaming.lock().unwrap();
            if *is_streaming {
                log::info!("Already streaming, closing previous connection...");
                // If we're already streaming, close the previous connection first
                if let Some(sender) = self.ws_sender.lock().unwrap().as_ref() {
                    let _ = sender.send(Vec::new()); // Send empty vector to signal close
                }
                // Small delay to ensure the previous connection closes
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            *is_streaming = true;
            *self.last_transcript.lock().unwrap() = String::new();
        }

        // Increment connection attempts counter
        {
            let mut attempts = self.connection_attempts.lock().unwrap();
            *attempts += 1;
            if *attempts > 5 {
                // Reset counter and delay to prevent rapid reconnection attempts
                *attempts = 0;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }

        // Create new channels
        let (ws_tx, mut ws_rx) = unbounded_channel::<Vec<u8>>();
        let (transcript_tx, transcript_rx) = unbounded_channel::<TranscriptionMessage>();

        *self.ws_sender.lock().unwrap() = Some(ws_tx);
        *self.transcript_receiver.lock().unwrap() = Some(transcript_rx);

        let url_str = format!(
               "wss://api.deepgram.com/v1/listen?model={}&language={}&punctuate={}&smart_format={}&interim_results={}",
               self.config.model,
               self.config.language,
               self.config.punctuate,
               self.config.smart_format,
               self.config.interim_results
           );

        log::info!("Connecting to Deepgram at URL: {}", url_str);

        let last_transcript = self.last_transcript.clone();
        let is_streaming = self.is_streaming.clone();
        let api_key = self.config.api_key.clone();
        let connection_attempts = self.connection_attempts.clone();

        tokio::spawn(async move {
            // IMPORTANT: Use string instead of Url object to avoid type issues
            log::info!("Attempting to connect to Deepgram WebSocket");

            // Create the request properly
            let request = tungstenite::http::Request::builder()
                .uri(url_str)
                .header("Authorization", format!("Token {}", api_key))
                .body(())
                .unwrap_or_else(|e| {
                    log::error!("Failed to build request: {}", e);
                    panic!("Failed to build WebSocket request");
                });

            // Connect using the request
            let ws_stream_result = connect_async(request).await;

            match ws_stream_result {
                Ok((ws_stream, response)) => {
                    log::info!("Successfully connected to Deepgram WebSocket");
                    log::debug!("Connection response status: {:?}", response.status());

                    // Reset connection attempts on successful connection
                    *connection_attempts.lock().unwrap() = 0;

                    let (mut ws_write, mut ws_read) = ws_stream.split();

                    let send_task = tokio::spawn(async move {
                        while let Some(audio_chunk) = ws_rx.recv().await {
                            if audio_chunk.is_empty() {
                                log::info!("Received close signal, closing WebSocket");
                                let close_msg = Message::Text(r#"{"type":"CloseStream"}"#.into());
                                if let Err(e) = ws_write.send(close_msg).await {
                                    log::error!("Failed to send close message: {}", e);
                                }
                                break;
                            } else {
                                let bytes = Bytes::from(audio_chunk);
                                let binary_msg = Message::Binary(bytes);
                                match ws_write.send(binary_msg.clone()).await {
                                    Ok(_) => {} // Successfully sent
                                    Err(e) => {
                                        log::error!("Failed to send audio chunk: {}", e);
                                        // Don't break immediately, retry a few times
                                        let retry_count = 3;
                                        let mut success = false;
                                        for i in 0..retry_count {
                                            log::warn!(
                                                "Retrying send (attempt {}/{})",
                                                i + 1,
                                                retry_count
                                            );
                                            tokio::time::sleep(Duration::from_millis(50)).await;
                                            if ws_write.send(binary_msg.clone()).await.is_ok() {
                                                success = true;
                                                break;
                                            }
                                        }
                                        if !success {
                                            log::error!("Failed to send after {} retries, closing connection", retry_count);
                                            break; // Give up after retries
                                        }
                                    }
                                }
                            }
                        }

                        // Close the connection properly
                        log::info!("Closing WebSocket connection");
                        if let Err(e) = ws_write.close().await {
                            log::warn!("Error closing WebSocket: {}", e);
                        }
                    });

                    let receive_task = tokio::spawn(async move {
                        // Add a ping timer to keep connection alive
                        let mut ping_interval = tokio::time::interval(Duration::from_secs(30));

                        loop {
                            tokio::select! {
                                _ = ping_interval.tick() => {
                                    // Send a heartbeat ping if supported
                                    if let Some(msg) = ws_read.next().await {
                                        match msg {
                                            Ok(Message::Pong(_)) => {
                                                // Successfully received pong response
                                                log::debug!("Received pong from Deepgram");
                                            },
                                            Err(e) => {
                                                log::warn!("Failed to receive pong: {}", e);
                                            },
                                            _ => {}
                                        }
                                    }
                                },
                                msg = ws_read.next() => {
                                    match msg {
                                        Some(Ok(Message::Text(text))) => {
                                            log::debug!("Received text message: {}", text);
                                            match serde_json::from_str::<DeepgramResponse>(&text) {
                                                Ok(response) => {
                                                    if !response.channel.alternatives.is_empty() {
                                                        let transcript = &response.channel.alternatives[0].transcript;

                                                        if !transcript.trim().is_empty() {
                                                            *last_transcript.lock().unwrap() = transcript.clone();

                                                            if response.is_final || response.speech_final {
                                                                log::info!("Final transcript: {}", transcript);
                                                                let _ = transcript_tx.send(
                                                                    TranscriptionMessage::Final(transcript.clone()),
                                                                );
                                                            } else {
                                                                log::debug!("Interim transcript: {}", transcript);
                                                                let _ = transcript_tx.send(
                                                                    TranscriptionMessage::Interim(transcript.clone()),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log::warn!("Failed to parse Deepgram response: {}", e);
                                                    // Still continue, don't break on parsing errors
                                                }
                                            }
                                        }
                                        Some(Ok(Message::Binary(data))) => {
                                            log::debug!("Received binary message of {} bytes", data.len());
                                        }
                                        Some(Ok(Message::Close(frame))) => {
                                            log::info!("Deepgram closed the connection: {:?}", frame);
                                            break;
                                        }
                                        Some(Err(e)) => {
                                            log::error!("WebSocket error: {}", e);
                                            let _ = transcript_tx.send(TranscriptionMessage::Error(
                                                format!("WebSocket error: {}", e),
                                            ));
                                            break;
                                        }
                                        None => {
                                            log::info!("WebSocket stream ended");
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }

                        *is_streaming.lock().unwrap() = false;
                        let _ = transcript_tx.send(TranscriptionMessage::Complete);
                    });

                    tokio::select! {
                        _ = send_task => log::info!("Send task completed"),
                        _ = receive_task => log::info!("Receive task completed"),
                    }
                }
                Err(e) => {
                    log::error!("Failed to connect to Deepgram: {}", e);
                    let _ = transcript_tx.send(TranscriptionMessage::Error(format!(
                        "Failed to connect to Deepgram: {}",
                        e
                    )));
                    *is_streaming.lock().unwrap() = false;
                }
            }
        });

        // Wait a moment to make sure the connection attempt is started
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    pub async fn create_audio_channel(&self) -> Result<mpsc::Sender<Vec<u8>>, AudioError> {
        if !*self.is_streaming.lock().unwrap() {
            return Err(AudioError::Transcription("Not streaming".to_string()));
        }

        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);
        let ws_sender = self.ws_sender.clone();

        // Create a dedicated task that transfers data from this channel to the WebSocket
        // Fix for Send trait issue
        tokio::spawn(async move {
            while let Some(audio_chunk) = rx.recv().await {
                let sender_option = {
                    // Scope the lock to drop it quickly
                    let guard = ws_sender.lock().unwrap();
                    guard.clone()
                };

                if let Some(sender) = sender_option {
                    if let Err(e) = sender.send(audio_chunk) {
                        log::error!("Failed to forward audio chunk to WebSocket: {}", e);
                        break;
                    }
                } else {
                    log::warn!("WebSocket sender no longer available");
                    break;
                }

                // Small delay to prevent overwhelming the socket
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            log::info!("Audio forwarding task completed");
        });

        Ok(tx)
    }

    pub fn send_audio_chunk(&self, chunk: Vec<u8>) -> Result<(), AudioError> {
        let mut retry_count = 0;
        const MAX_RETRIES: usize = 3;

        // First check if streaming is active
        if !*self.is_streaming.lock().unwrap() {
            log::warn!("Cannot send audio chunk - transcription service not streaming");
            return Err(AudioError::Transcription("Not streaming".to_string()));
        }

        while retry_count < MAX_RETRIES {
            // Scope the lock to drop it quickly
            let sender_opt = {
                let guard = self.ws_sender.lock().unwrap();
                guard.clone()
            };

            if let Some(sender) = sender_opt {
                match sender.send(chunk.clone()) {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        log::warn!(
                            "Failed to send chunk, retry {}/{}: {}",
                            retry_count + 1,
                            MAX_RETRIES,
                            e
                        );

                        if retry_count == MAX_RETRIES - 1 {
                            // Mark as not streaming on final retry failure
                            *self.is_streaming.lock().unwrap() = false;
                            return Err(AudioError::Transcription(format!(
                                "Failed to send audio chunk after {} retries: {}",
                                MAX_RETRIES, e
                            )));
                        }

                        retry_count += 1;
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
            } else {
                log::error!("No WebSocket sender available");
                *self.is_streaming.lock().unwrap() = false;
                return Err(AudioError::Transcription(
                    "WebSocket sender not available".to_string(),
                ));
            }
        }

        Err(AudioError::Transcription(
            "Maximum retries exceeded".to_string(),
        ))
    }

    pub fn try_receive_message(&self) -> Option<TranscriptionMessage> {
        if let Some(receiver) = self.transcript_receiver.lock().unwrap().as_mut() {
            receiver.try_recv().ok()
        } else {
            None
        }
    }

    pub async fn check_connection(&self) -> bool {
        if !*self.is_streaming.lock().unwrap() {
            return false;
        }

        // Try to send a ping message
        // Scope the lock to drop it quickly
        let sender_opt = {
            let guard = self.ws_sender.lock().unwrap();
            guard.clone()
        };

        if let Some(sender) = sender_opt {
            // We'll use a very small chunk as a "ping"
            let ping_data = vec![0u8; 2];
            match sender.send(ping_data) {
                Ok(_) => true,
                Err(e) => {
                    log::warn!("WebSocket connection appears to be broken: {}", e);
                    *self.is_streaming.lock().unwrap() = false;
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn end_streaming(&self) -> Result<(), AudioError> {
        log::info!("Ending transcription streaming");

        // First check if we're actually streaming
        if !*self.is_streaming.lock().unwrap() {
            log::warn!("Not currently streaming, nothing to end");
            return Ok(());
        }

        // Scope the lock to drop it quickly
        let sender_opt = {
            let guard = self.ws_sender.lock().unwrap();
            guard.clone()
        };

        if let Some(sender) = sender_opt {
            match sender.send(Vec::new()) {
                Ok(_) => {
                    log::info!("Successfully sent end streaming signal");
                    // Give a moment for the close message to be processed
                    std::thread::sleep(Duration::from_millis(100));
                    *self.is_streaming.lock().unwrap() = false;
                    Ok(())
                }
                Err(e) => {
                    log::error!("Failed to send end streaming signal: {}", e);
                    // Even if we fail to send the signal, mark as not streaming
                    *self.is_streaming.lock().unwrap() = false;
                    Err(AudioError::Transcription(format!(
                        "Failed to send end streaming signal: {}",
                        e
                    )))
                }
            }
        } else {
            log::warn!("No active streaming channel to end");
            *self.is_streaming.lock().unwrap() = false;
            Ok(())
        }
    }

    pub fn get_last_transcript(&self) -> String {
        self.last_transcript.lock().unwrap().clone()
    }

    pub async fn transcribe_file(&self, audio_path: PathBuf) -> Result<Vec<String>, AudioError> {
        if self.config.api_key.is_empty() {
            return Err(AudioError::Transcription(
                "Deepgram API key is not set".to_string(),
            ));
        }

        log::info!("Transcribing file: {}", audio_path.display());

        let mut file = File::open(&audio_path)
            .await
            .map_err(|e| AudioError::Transcription(format!("Failed to open audio file: {}", e)))?;

        let mut audio_data = Vec::new();
        file.read_to_end(&mut audio_data)
            .await
            .map_err(|e| AudioError::Transcription(format!("Failed to read audio file: {}", e)))?;

        log::info!("Read {} bytes from audio file", audio_data.len());

        if audio_data.is_empty() {
            return Err(AudioError::Transcription("Audio file is empty".to_string()));
        }

        let client = reqwest::Client::new();

        let url = format!(
            "https://api.deepgram.com/v1/listen?model={}&language={}&punctuate={}&smart_format={}",
            self.config.model,
            self.config.language,
            self.config.punctuate,
            self.config.smart_format
        );

        log::info!("Sending transcription request to Deepgram API: {}", url);

        let response = client
            .post(&url)
            .header("Authorization", format!("Token {}", self.config.api_key))
            .header("Content-Type", "audio/wav")
            .body(audio_data)
            .send()
            .await
            .map_err(|e| AudioError::Transcription(format!("Failed to send request: {}", e)))?;

        let response_status = response.status();
        log::info!("Received response with status: {}", response_status);

        if !response_status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            log::error!("Deepgram API error ({}): {}", response_status, error_text);

            return Err(AudioError::Transcription(format!(
                "Deepgram API error ({}): {}",
                response_status, error_text
            )));
        }

        let response_json = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| AudioError::Transcription(format!("Failed to parse response: {}", e)))?;

        if let Some(results) = response_json.get("results") {
            if let Some(channels) = results.get("channels") {
                if let Some(channel) = channels.get(0) {
                    if let Some(alternatives) = channel.get("alternatives") {
                        if let Some(alternative) = alternatives.get(0) {
                            if let Some(transcript) = alternative.get("transcript") {
                                if let Some(text) = transcript.as_str() {
                                    let trimmed = text.trim();
                                    if !trimmed.is_empty() {
                                        log::info!("Successfully transcribed: {}", trimmed);
                                        return Ok(vec![trimmed.to_string()]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        log::error!("Failed to extract transcript from response");
        Err(AudioError::Transcription(
            "Failed to extract transcript from response".to_string(),
        ))
    }

    pub async fn transcribe(&mut self, audio_path: PathBuf) -> Result<Vec<String>, AudioError> {
        self.transcribe_file(audio_path).await
    }
}
