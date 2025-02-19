use crate::error::AudioError;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Stream,
};
use rubato::Resampler;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Default)]
pub struct AudioRecorder {
    recording: Arc<Mutex<bool>>,
    stream: Arc<Mutex<Option<Stream>>>,
    buffer: Arc<Mutex<Vec<f32>>>,
    app_handle: Arc<Mutex<Option<AppHandle>>>,
}

impl AudioRecorder {
    pub fn set_app_handle(&self, handle: AppHandle) {
        *self.app_handle.lock().unwrap() = Some(handle);
    }

    pub fn start_recording(
        &self,
        app_handle: &AppHandle,
        output_path: std::path::PathBuf,
    ) -> Result<(), AudioError> {
        self.set_app_handle(app_handle.clone());

        // Ensure we're not already recording
        let mut recording = self
            .recording
            .lock()
            .map_err(|_| AudioError::Recording("Failed to acquire recording lock".to_string()))?;

        if *recording {
            return Err(AudioError::Recording("Already recording".to_string()));
        }

        // Setup audio device and stream
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| AudioError::Device("Failed to get default input device".to_string()))?;

        let input_config = device
            .default_input_config()
            .map_err(|e| AudioError::Device(e.to_string()))?;

        let config = input_config.config();
        let channel_count = config.channels as usize;
        let sample_rate = config.sample_rate.0 as usize;

        // Create WAV writer
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let file = std::fs::File::create(&output_path)
            .map_err(|e| AudioError::Recording(format!("Failed to create output file: {}", e)))?;

        let writer = Arc::new(Mutex::new(Some(
            hound::WavWriter::new(std::io::BufWriter::new(file), spec).map_err(|e| {
                AudioError::Recording(format!("Failed to create WAV writer: {}", e))
            })?,
        )));

        // Setup resampler
        let resample_ratio = 16000.0 / sample_rate as f64;
        let mut resampler = rubato::FastFixedIn::new(
            resample_ratio,
            10.0,
            rubato::PolynomialDegree::Septic,
            1024,
            1,
        )
        .map_err(|e| AudioError::Recording(format!("Failed to create resampler: {}", e)))?;

        let writer_clone = Arc::clone(&writer);
        let recording_clone = Arc::clone(&self.recording);
        let buffer_clone = Arc::clone(&self.buffer);
        let app_handle_clone = Arc::clone(&self.app_handle);

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if *recording_clone.lock().unwrap() {
                        // Process audio in chunks for level monitoring
                        let pcm: Vec<f32> = data.iter().step_by(channel_count).copied().collect();
                        let chunk_size = pcm.len() / 8;
                        let mut levels = vec![0.0; 8];

                        for (i, chunk) in pcm.chunks(chunk_size).enumerate() {
                            if i < 8 {
                                let rms = (chunk.iter().map(|sample| sample * sample).sum::<f32>()
                                    / chunk.len() as f32)
                                    .sqrt();
                                levels[i] = rms;
                            }
                        }

                        // Emit audio levels
                        if let Some(handle) = app_handle_clone.lock().unwrap().as_ref() {
                            handle.emit("audio-levels", levels).unwrap_or_default();
                        }

                        // Process and save audio
                        if !pcm.is_empty() {
                            let mut buffer = buffer_clone.lock().unwrap();
                            buffer.extend_from_slice(&pcm);

                            if buffer.len() >= 1024 {
                                let full_chunks = buffer.len() / 1024;

                                for chunk in 0..full_chunks {
                                    let start = chunk * 1024;
                                    let end = (chunk + 1) * 1024;
                                    let chunk_data = &buffer[start..end];

                                    if let Ok(output) = resampler.process(&[chunk_data], None) {
                                        if let Some(writer) = writer_clone.lock().unwrap().as_mut()
                                        {
                                            for &sample in &output[0] {
                                                let sample = (sample * i16::MAX as f32) as i16;
                                                writer.write_sample(sample).unwrap_or_default();
                                            }
                                        }
                                    }
                                }

                                // Handle remaining samples
                                let remainder = buffer.len() % 1024;
                                if remainder > 0 {
                                    let start = buffer.len() - remainder;
                                    let remainder_data: Vec<f32> = buffer[start..].to_vec();
                                    buffer.clear();
                                    buffer.extend(remainder_data);
                                } else {
                                    buffer.clear();
                                }
                            }
                        }
                    }
                },
                move |err| eprintln!("Audio input error: {}", err),
                None,
            )
            .map_err(|e| AudioError::Recording(format!("Failed to build input stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| AudioError::Recording(format!("Failed to start stream: {}", e)))?;

        *self.stream.lock().unwrap() = Some(stream);
        *recording = true;

        Ok(())
    }

    pub fn stop_recording(&self) -> Result<(), AudioError> {
        let mut recording = self
            .recording
            .lock()
            .map_err(|_| AudioError::Recording("Failed to acquire recording lock".to_string()))?;

        if !*recording {
            return Err(AudioError::Recording("Not currently recording".to_string()));
        }

        *recording = false;
        *self.stream.lock().unwrap() = None;
        self.buffer.lock().unwrap().clear();

        Ok(())
    }
}

// Make AudioRecorder thread-safe
unsafe impl Send for AudioRecorder {}
unsafe impl Sync for AudioRecorder {}
