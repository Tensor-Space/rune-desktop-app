use crate::core::error::AudioError;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream,
};
use crossbeam_channel::{bounded, Sender};
use parking_lot::Mutex;
use rubato::{FftFixedIn, Resampler};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

const CHUNK_DURATION: Duration = Duration::from_secs(10);
const TARGET_SAMPLE_RATE: u32 = 16000;
const LEVEL_UPDATE_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Default, Clone)]
struct AudioData {
    recording: bool,
    app_handle: Option<AppHandle>,
    buffers: Vec<Vec<f32>>,
}

#[derive(Default)]
struct RecorderState {
    stream: Option<Stream>,
    device_id: Option<String>,
    audio_data: Arc<Mutex<AudioData>>,
    source_sample_rate: u32,
}

pub struct AudioRecorder {
    state: Arc<Mutex<RecorderState>>,
    last_level_update: Arc<Mutex<Instant>>,
    audio_sender: Arc<Mutex<Option<Sender<Vec<f32>>>>>,
    recording_active: Arc<std::sync::atomic::AtomicBool>,
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RecorderState {
                audio_data: Arc::new(Mutex::new(AudioData::default())),
                source_sample_rate: 44100,
                ..Default::default()
            })),
            last_level_update: Arc::new(Mutex::new(Instant::now())),
            audio_sender: Arc::new(Mutex::new(None)),
            recording_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        let state = self.state.lock();
        state.audio_data.lock().app_handle = Some(handle);
    }

    pub fn set_device_id(&self, device_id: Option<String>) {
        self.state.lock().device_id = device_id;
    }

    async fn get_input_device(&self) -> Result<Device, AudioError> {
        let host = cpal::default_host();
        let state = self.state.lock();

        if let Some(device_id) = state.device_id.as_ref() {
            let devices = host
                .devices()
                .map_err(|e| AudioError::Device(format!("Failed to get devices: {}", e)))?;

            for device in devices {
                if let Ok(name) = device.name() {
                    if name == *device_id {
                        return Ok(device);
                    }
                }
            }

            Err(AudioError::Device(format!(
                "Device '{}' not found",
                device_id
            )))
        } else {
            host.default_input_device()
                .ok_or_else(|| AudioError::Device("No default input device available".to_string()))
        }
    }

    pub async fn start_recording(&self, app_handle: &AppHandle) -> Result<(), AudioError> {
        self.set_app_handle(app_handle.clone());

        {
            let state = self.state.lock();
            let mut audio_data = state.audio_data.lock();
            if audio_data.recording {
                return Err(AudioError::Recording("Already recording".to_string()));
            }
            audio_data.recording = true;
            audio_data.buffers.clear();
        }

        let device = self.get_input_device().await?;
        println!("Using input device: {}", device.name().unwrap_or_default());

        let input_config = device
            .default_input_config()
            .map_err(|e| AudioError::Device(e.to_string()))?;

        println!("Input format: {:?}", input_config.sample_format());
        println!("Channels: {}", input_config.channels());
        println!("Sample rate: {}", input_config.sample_rate().0);

        let config = input_config.config();
        let channel_count = config.channels as usize;
        let source_sample_rate = config.sample_rate.0;

        self.state.lock().source_sample_rate = source_sample_rate;

        println!(
            "Input config - Channels: {}, Sample Rate: {}",
            channel_count, source_sample_rate
        );

        let (tx, rx) = bounded::<Vec<f32>>(1024);
        *self.audio_sender.lock() = Some(tx.clone());

        let audio_data = self.state.lock().audio_data.clone();
        let last_level_update_arc = Arc::clone(&self.last_level_update);
        let chunk_samples = (source_sample_rate as f32 * CHUNK_DURATION.as_secs_f32()) as usize;

        self.recording_active
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let recording_active = Arc::clone(&self.recording_active);

        std::thread::spawn(move || {
            let mut current_chunk = Vec::with_capacity(chunk_samples);
            let mut total_samples_processed = 0;

            while recording_active.load(std::sync::atomic::Ordering::SeqCst) {
                match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(pcm) => {
                        let mut audio_data = audio_data.lock();
                        if !audio_data.recording {
                            println!(
                                "Recording stopped. Total samples processed: {}",
                                total_samples_processed
                            );
                            break;
                        }

                        let non_zero_samples = pcm.iter().filter(|&&x| x != 0.0).count();
                        let max_amplitude = pcm.iter().fold(0.0f32, |max, &x| max.max(x.abs()));
                        println!(
                            "Received audio chunk - Size: {}, Non-zero samples: {}, Max amplitude: {}",
                            pcm.len(),
                            non_zero_samples,
                            max_amplitude
                        );

                        // Skip silent chunks
                        if max_amplitude < 1e-6 {
                            println!("Skipping silent chunk");
                            continue;
                        }

                        total_samples_processed += pcm.len();

                        if let Some(handle) = audio_data.app_handle.as_ref() {
                            let now = Instant::now();
                            let mut last_update = last_level_update_arc.lock();
                            if now.duration_since(*last_update) >= LEVEL_UPDATE_INTERVAL {
                                let rms = (pcm.iter().map(|&s| s * s).sum::<f32>()
                                    / pcm.len() as f32)
                                    .sqrt();
                                if let Err(e) = handle.emit("audio-levels", vec![rms]) {
                                    eprintln!("Failed to emit audio levels: {}", e);
                                }
                                *last_update = now;
                            }
                        }

                        current_chunk.extend_from_slice(&pcm);

                        if current_chunk.len() >= chunk_samples {
                            println!(
                                "Chunk complete - Size: {}, Non-zero samples: {}",
                                current_chunk.len(),
                                current_chunk.iter().filter(|&&x| x != 0.0).count()
                            );
                            audio_data.buffers.push(std::mem::replace(
                                &mut current_chunk,
                                Vec::with_capacity(chunk_samples),
                            ));
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        println!("Channel disconnected, stopping recording thread");
                        break;
                    }
                }
            }

            // Push any remaining samples
            if !current_chunk.is_empty() {
                let mut audio_data = audio_data.lock();
                audio_data.buffers.push(current_chunk);
            }

            println!("Processing thread finished");
        });

        let sender = self.audio_sender.lock().as_ref().unwrap().clone();
        let recording_active_clone = Arc::clone(&self.recording_active);
        let data_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if !recording_active_clone.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }

            // Average samples across channels
            let pcm: Vec<f32> = data
                .chunks(channel_count)
                .map(|chunk| chunk.iter().sum::<f32>() / channel_count as f32)
                .collect();

            // Only send if we have non-zero data
            let max_amplitude = pcm.iter().fold(0.0f32, |max, &x| max.max(x.abs()));
            if max_amplitude < 1e-6 {
                return;
            }

            match sender.send(pcm) {
                Ok(_) => {
                    println!(
                        "Audio data sent - Size: {}, Max amplitude: {}",
                        data.len(),
                        max_amplitude
                    );
                }
                Err(e) => {
                    if recording_active_clone.load(std::sync::atomic::Ordering::SeqCst) {
                        eprintln!("Failed to send audio data: {}", e);
                    }
                }
            }
        };

        let error_callback = move |err| eprintln!("Audio input error: {}", err);

        let stream = device
            .build_input_stream(&config, data_callback, error_callback, None)
            .map_err(|e| AudioError::Recording(format!("Failed to build input stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| AudioError::Recording(format!("Failed to start stream: {}", e)))?;

        self.state.lock().stream = Some(stream);
        println!("Recording started successfully");

        Ok(())
    }

    pub async fn stop_recording(&self, output_path: std::path::PathBuf) -> Result<(), AudioError> {
        // First, stop the recording flag to prevent new data from being processed
        {
            let state = self.state.lock();
            let mut audio_data = state.audio_data.lock();
            if !audio_data.recording {
                return Err(AudioError::Recording("Not currently recording".to_string()));
            }
            audio_data.recording = false;
        }

        // Stop the stream first to prevent new callbacks
        {
            let mut state = self.state.lock();
            if let Some(stream) = state.stream.take() {
                drop(stream); // This will stop the audio callbacks
            }
        }

        // Now set the recording_active flag to false
        self.recording_active
            .store(false, std::sync::atomic::Ordering::SeqCst);

        // Wait a moment for any in-flight callbacks to complete
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Now it's safe to drop the sender
        drop(self.audio_sender.lock().take());

        // Wait for the processing thread to finish
        std::thread::sleep(std::time::Duration::from_secs(1));

        let state = self.state.lock();
        let mut audio_data = state.audio_data.lock();

        let source_sample_rate = state.source_sample_rate;
        let buffers = std::mem::take(&mut audio_data.buffers);

        println!(
            "Stop recording - Number of buffers: {}, Source sample rate: {}",
            buffers.len(),
            source_sample_rate
        );

        // Check if we have any audio data
        if buffers.is_empty() {
            return Err(AudioError::Recording("No audio data recorded".to_string()));
        }

        let total_input_samples: usize = buffers.iter().map(|b| b.len()).sum();
        println!("Total input samples: {}", total_input_samples);

        drop(audio_data);
        drop(state);

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: TARGET_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(&output_path, spec)
            .map_err(|e| AudioError::Recording(format!("Failed to create WAV writer: {}", e)))?;

        println!("Created WAV writer with spec: {:?}", spec);

        // Calculate optimal chunk size for resampling
        let resampler_chunk_size = ((source_sample_rate as f32 * 0.1) as usize).next_power_of_two();
        println!("Using resampler chunk size: {}", resampler_chunk_size);

        let mut resampler = FftFixedIn::<f32>::new(
            source_sample_rate as usize,
            TARGET_SAMPLE_RATE as usize,
            resampler_chunk_size,
            1, // Increased overlap for better quality
            1,
        )
        .map_err(|e| AudioError::Recording(format!("Failed to create resampler: {}", e)))?;

        let mut total_samples_written = 0;
        let mut max_amplitude = 0.0f32;

        for (i, buffer) in buffers.iter().enumerate() {
            if buffer.is_empty() {
                println!("Warning: Empty buffer at index {}", i);
                continue;
            }

            // Check for silent buffer
            let buffer_max = buffer.iter().fold(0.0f32, |max, &x| max.max(x.abs()));
            max_amplitude = max_amplitude.max(buffer_max);

            if buffer_max < 1e-6 {
                println!(
                    "Warning: Silent buffer at index {} (max amplitude: {})",
                    i, buffer_max
                );
                continue;
            }

            // Process buffer in smaller chunks to avoid resampling issues
            let chunk_size = resampler_chunk_size;
            for chunk in buffer.chunks(chunk_size) {
                let chunk_vec = chunk.to_vec();
                match resampler.process(&[&chunk_vec], None) {
                    Ok(mut output) => {
                        if let Some(samples) = output.pop() {
                            if samples.is_empty() {
                                println!(
                                    "Warning: Resampler produced empty output for buffer {}",
                                    i
                                );
                                continue;
                            }

                            let resampled_max =
                                samples.iter().fold(0.0f32, |max, &x| max.max(x.abs()));
                            println!(
                            "Buffer {}: Input len={}, Output len={}, Input max={}, Output max={}",
                            i,
                            buffer.len(),
                            samples.len(),
                            buffer_max,
                            resampled_max
                        );

                            // Apply normalization if audio is too quiet
                            let gain = if max_amplitude < 0.1 {
                                0.5 / max_amplitude
                            } else {
                                1.0
                            };

                            for sample in samples {
                                let normalized = sample * gain;
                                let sample_i16 = (normalized * i16::MAX as f32)
                                    .clamp(i16::MIN as f32, i16::MAX as f32)
                                    as i16;
                                writer.write_sample(sample_i16).map_err(|e| {
                                    AudioError::Recording(format!("Failed to write sample: {}", e))
                                })?;
                                total_samples_written += 1;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Resampling error on chunk of buffer {}: {}", i + 1, e);
                        continue;
                    }
                }
            }
        }

        if total_samples_written == 0 {
            return Err(AudioError::Recording(
                "No samples were written to the output file".to_string(),
            ));
        }

        writer
            .finalize()
            .map_err(|e| AudioError::Recording(format!("Failed to finalize WAV file: {}", e)))?;

        println!(
            "Recording saved successfully - Total samples written: {}, File path: {}",
            total_samples_written,
            output_path.display()
        );

        let mut state = self.state.lock();
        state.stream = None;

        Ok(())
    }
}

unsafe impl Send for AudioRecorder {}
unsafe impl Sync for AudioRecorder {}
