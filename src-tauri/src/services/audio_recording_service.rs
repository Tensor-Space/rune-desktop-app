use crate::core::error::AudioError;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream,
};
use crossbeam_channel::{bounded, Sender};
use parking_lot::Mutex;
use rubato::{FftFixedIn, Resampler};
use std::{
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

const TARGET_SAMPLE_RATE: u32 = 16000;
const LEVEL_UPDATE_INTERVAL: Duration = Duration::from_millis(50);
const DEFAULT_CHUNK_SIZE: usize = 4096;
const MIN_CHUNK_SIZE: usize = 1024;

fn flush_println(msg: &str) {
    println!("{}", msg);
    std::io::stdout().flush().unwrap_or_default();
}

#[derive(Default, Clone)]
struct AudioData {
    recording: bool,
    app_handle: Option<AppHandle>,
    buffers: Vec<Vec<f32>>,
    current_chunk: Vec<f32>,
}

impl AudioData {
    fn new() -> Self {
        Self {
            recording: false,
            app_handle: None,
            buffers: Vec::new(),
            current_chunk: Vec::with_capacity(DEFAULT_CHUNK_SIZE),
        }
    }

    fn store_samples(&mut self, samples: &[f32]) {
        self.current_chunk.extend_from_slice(samples);

        if self.current_chunk.len() >= DEFAULT_CHUNK_SIZE {
            let full_buffer = std::mem::replace(
                &mut self.current_chunk,
                Vec::with_capacity(DEFAULT_CHUNK_SIZE),
            );
            self.buffers.push(full_buffer);
            flush_println(&format!(
                "Chunk complete - Size: {}, Total chunks: {}",
                DEFAULT_CHUNK_SIZE,
                self.buffers.len()
            ));
        }
    }

    fn finalize(&mut self) {
        if !self.current_chunk.is_empty() && self.current_chunk.len() >= MIN_CHUNK_SIZE {
            self.buffers.push(std::mem::take(&mut self.current_chunk));
            flush_println(&format!(
                "Final chunk added - Total chunks: {}",
                self.buffers.len()
            ));
        }
    }
}

#[derive(Default)]
struct RecorderState {
    stream: Option<Stream>,
    device_id: Option<String>,
    audio_data: Arc<Mutex<AudioData>>,
    current_sample_rate: Arc<Mutex<u32>>,
}

pub struct AudioRecordingService {
    state: Arc<Mutex<RecorderState>>,
    last_level_update: Arc<Mutex<Instant>>,
    audio_sender: Arc<Mutex<Option<Sender<Vec<f32>>>>>,
    recording_active: Arc<std::sync::atomic::AtomicBool>,
}

impl Default for AudioRecordingService {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecordingService {
    pub fn new() -> Self {
        flush_println("Initializing AudioRecordingService");
        Self {
            state: Arc::new(Mutex::new(RecorderState {
                audio_data: Arc::new(Mutex::new(AudioData::new())),
                current_sample_rate: Arc::new(Mutex::new(0)),
                ..Default::default()
            })),
            last_level_update: Arc::new(Mutex::new(Instant::now())),
            audio_sender: Arc::new(Mutex::new(None)),
            recording_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        flush_println("Setting app handle");
        let state = self.state.lock();
        state.audio_data.lock().app_handle = Some(handle);
    }

    pub fn set_device_id(&self, device_id: Option<String>) {
        flush_println(&format!("Setting device ID: {:?}", device_id));
        self.state.lock().device_id = device_id;
    }

    async fn get_input_device(&self) -> Result<Device, AudioError> {
        flush_println("\n=== Getting Input Device ===");
        let host = cpal::default_host();
        let state = self.state.lock();

        flush_println("\nAvailable audio devices:");
        let mut found_devices = Vec::new();
        if let Ok(devices) = host.devices() {
            for device in devices {
                if let Ok(name) = device.name() {
                    let supported_configs = device.supported_input_configs().map_err(|e| {
                        AudioError::Device(format!("Error getting configs for {}: {}", name, e))
                    })?;

                    flush_println(&format!("\nDevice: {}", name));
                    flush_println("Supported configurations:");
                    let mut config_count = 0;
                    for config in supported_configs {
                        config_count += 1;
                        flush_println(&format!("  - {:?}", config));
                    }

                    if device.default_input_config().is_ok() {
                        found_devices.push(device);
                        flush_println(&format!("  Total configs: {}", config_count));
                        flush_println("  (This is an input device)");
                    }
                }
            }
        }

        if let Some(device_id) = state.device_id.as_ref() {
            flush_println(&format!("\nLooking for device: {}", device_id));

            // Try to find an exact match first
            for device in found_devices.iter() {
                if let Ok(name) = device.name() {
                    if name == *device_id {
                        flush_println(&format!("Found exact match for device: {}", name));
                        return Ok(device.clone());
                    }
                }
            }

            // If no exact match, try to find a device that contains the name
            for device in found_devices.iter() {
                if let Ok(name) = device.name() {
                    if name.contains(device_id) {
                        flush_println(&format!("Found partial match for device: {}", name));
                        return Ok(device.clone());
                    }
                }
            }

            // If still no match, return the first available input device
            if let Some(device) = found_devices.first() {
                flush_println(&format!(
                    "Device '{}' not found exactly, using first available input device: {}",
                    device_id,
                    device.name().unwrap_or_default()
                ));
                return Ok(device.clone());
            }

            Err(AudioError::Device(format!(
                "No suitable input device found for '{}'",
                device_id
            )))
        } else {
            flush_println("\nNo device specified, using default input device");
            host.default_input_device()
                .ok_or_else(|| AudioError::Device("No default input device available".to_string()))
        }
    }

    pub async fn start_recording(&self, app_handle: &AppHandle) -> Result<(), AudioError> {
        flush_println("=== Starting Recording Process ===");

        {
            let mut state = self.state.lock();
            if let Some(stream) = state.stream.take() {
                flush_println("Cleaning up previous audio stream");
                if let Err(e) = stream.pause() {
                    flush_println(&format!("Warning: Failed to pause stream: {}", e));
                }
                drop(stream);
            }

            if let Some(sender) = self.audio_sender.lock().take() {
                flush_println("Cleaning up previous audio channel");
                drop(sender);
            }

            self.recording_active
                .store(false, std::sync::atomic::Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        self.set_app_handle(app_handle.clone());

        let state = self.state.lock();
        {
            let mut audio_data = state.audio_data.lock();
            if audio_data.recording {
                flush_println("Error: Already recording");
                return Err(AudioError::Recording("Already recording".to_string()));
            }
            audio_data.recording = true;
            audio_data.buffers.clear();
            audio_data.current_chunk.clear();
            flush_println("Recording state initialized");
        }
        drop(state);

        let device = self.get_input_device().await?;

        let supported_configs = device
            .supported_input_configs()
            .map_err(|e| AudioError::Device(format!("Error getting supported configs: {}", e)))?;

        let supported_configs_vec: Vec<_> = supported_configs.collect();

        flush_println("\nSupported configurations:");
        for (i, config) in supported_configs_vec.iter().enumerate() {
            flush_println(&format!("Config {}: {:?}", i, config));
        }

        let config = supported_configs_vec
            .iter()
            .find(|config| config.sample_format() == cpal::SampleFormat::F32)
            .ok_or_else(|| {
                AudioError::Device("No suitable audio configuration found".to_string())
            })?;

        let native_sample_rate = config.min_sample_rate().0;
        {
            let state = self.state.lock();
            *state.current_sample_rate.lock() = native_sample_rate;
        }

        let config = config.with_sample_rate(cpal::SampleRate(native_sample_rate));

        let num_channels = config.channels() as usize;
        flush_println(&format!(
            "Using {} channels at {} Hz",
            num_channels, native_sample_rate
        ));

        let chunk_size = DEFAULT_CHUNK_SIZE * num_channels;
        let (tx, rx) = bounded::<Vec<f32>>(1024);
        *self.audio_sender.lock() = Some(tx.clone());

        let state = self.state.lock();
        state.audio_data.lock().recording = true;
        state.audio_data.lock().buffers.clear();
        state.audio_data.lock().current_chunk = Vec::with_capacity(chunk_size);
        let audio_data = state.audio_data.clone();
        drop(state);

        let last_level_update_arc = Arc::clone(&self.last_level_update);
        self.recording_active
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let recording_active = Arc::clone(&self.recording_active);

        std::thread::spawn(move || {
            while recording_active.load(std::sync::atomic::Ordering::SeqCst) {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(pcm) => {
                        let mut audio_data = audio_data.lock();
                        if !audio_data.recording {
                            continue;
                        }

                        // Mix stereo to mono if needed
                        let mono_samples: Vec<f32> = if pcm.len() % 2 == 0 && num_channels == 2 {
                            pcm.chunks(2)
                                .map(|chunk| (chunk[0] + chunk[1]) * 0.5)
                                .collect()
                        } else {
                            pcm
                        };

                        // Calculate levels for visualization (8 frequency bands)
                        let levels: Vec<f32> = if !mono_samples.is_empty() {
                            let chunk_size = mono_samples.len() / 8;
                            (0..8)
                                .map(|i| {
                                    let start = i * chunk_size;
                                    let end = start + chunk_size;
                                    let chunk = &mono_samples[start..end];
                                    chunk
                                        .iter()
                                        .map(|&s| s.abs())
                                        .fold(0f32, |max, val| max.max(val))
                                })
                                .collect()
                        } else {
                            vec![0.0; 8]
                        };

                        // Store the audio data
                        audio_data.store_samples(&mono_samples);

                        // Emit audio levels for UI feedback
                        if let Some(handle) = audio_data.app_handle.as_ref() {
                            let now = Instant::now();
                            let mut last_update = last_level_update_arc.lock();
                            if now.duration_since(*last_update) >= LEVEL_UPDATE_INTERVAL {
                                if let Err(e) = handle.emit("audio-levels", levels) {
                                    flush_println(&format!("Failed to emit audio levels: {}", e));
                                }
                                *last_update = now;
                            }
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        if recording_active.load(std::sync::atomic::Ordering::SeqCst) {
                            flush_println("Channel disconnected but recording still active");
                            continue;
                        }
                        break;
                    }
                }
            }

            // Finalize audio and cleanup
            let mut audio_data = audio_data.lock();
            audio_data.finalize();
            flush_println(&format!(
                "Processing thread finished with {} chunks",
                audio_data.buffers.len()
            ));
        });

        let sender = tx;
        let recording_active_clone = Arc::clone(&self.recording_active);

        let data_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if !recording_active_clone.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }

            let pcm = data.to_vec();
            let _ = sender.send(pcm);
        };

        let error_callback = move |err| {
            flush_println(&format!("Audio input error: {}", err));
        };

        let stream = device
            .build_input_stream(&config.into(), data_callback, error_callback, None)
            .map_err(|e| {
                flush_println(&format!("Failed to build input stream: {}", e));
                AudioError::Recording(format!("Failed to build input stream: {}", e))
            })?;

        stream.play().map_err(|e| {
            flush_println(&format!("Failed to start stream: {}", e));
            AudioError::Recording(format!("Failed to start stream: {}", e))
        })?;

        self.state.lock().stream = Some(stream);
        flush_println("=== Recording Started Successfully ===");

        Ok(())
    }

    pub async fn stop_recording(&self, output_path: PathBuf) -> Result<(), AudioError> {
        flush_println("=== Stopping Recording ===");

        self.recording_active
            .store(false, std::sync::atomic::Ordering::SeqCst);

        {
            let state = self.state.lock();
            let mut audio_data = state.audio_data.lock();
            if !audio_data.recording {
                return Err(AudioError::Recording("Not currently recording".to_string()));
            }
            audio_data.recording = false;
        }

        let native_sample_rate = {
            let mut state = self.state.lock();
            let sample_rate = *state.current_sample_rate.lock();
            if let Some(stream) = state.stream.take() {
                flush_println("Stopping audio stream");
                if let Err(e) = stream.pause() {
                    flush_println(&format!("Warning: Failed to pause stream: {}", e));
                }
            }
            sample_rate
        };

        {
            if let Some(sender) = self.audio_sender.lock().take() {
                flush_println("Cleaning up audio channel");
                drop(sender);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(200));

        let state = self.state.lock();
        let mut audio_data = state.audio_data.lock();
        audio_data.finalize();

        let buffers = std::mem::take(&mut audio_data.buffers);
        drop(audio_data);
        drop(state);

        flush_println(&format!(
            "Stop recording - Buffers: {}, Sample rate: {}",
            buffers.len(),
            native_sample_rate
        ));

        if buffers.is_empty() {
            return Err(AudioError::Recording("No audio data recorded".to_string()));
        }

        let total_samples = buffers.iter().map(|b| b.len()).sum::<usize>();
        flush_println(&format!("Total samples recorded: {}", total_samples));

        // Create WAV file with target 16kHz sample rate
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: TARGET_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        flush_println(&format!("Creating WAV file: {}", output_path.display()));
        let mut writer = hound::WavWriter::create(&output_path, spec)
            .map_err(|e| AudioError::Recording(format!("Failed to create WAV file: {}", e)))?;

        // Configure resampler for source rate to 16kHz conversion
        let resampler_chunk_size = 1024;
        let mut resampler = FftFixedIn::<f32>::new(
            native_sample_rate as usize,
            TARGET_SAMPLE_RATE as usize,
            resampler_chunk_size,
            1,
            1,
        )
        .map_err(|e| AudioError::Recording(format!("Failed to create resampler: {}", e)))?;

        flush_println(&format!(
            "Resampler configured - Input rate: {}, Output rate: {}, Chunk size: {}",
            native_sample_rate, TARGET_SAMPLE_RATE, resampler_chunk_size
        ));

        let mut total_written = 0;
        let mut max_amplitude = 0.0f32;

        flush_println("Processing and resampling recorded audio");
        for (i, buffer) in buffers.iter().enumerate() {
            if buffer.is_empty() {
                continue;
            }

            // Track maximum amplitude for normalization
            let buffer_max = buffer.iter().fold(0.0f32, |max, &x| max.max(x.abs()));
            max_amplitude = max_amplitude.max(buffer_max);

            // Process audio in chunks with overlap for smoother resampling
            let mut processed_samples = 0;
            while processed_samples < buffer.len() {
                let end = (processed_samples + resampler_chunk_size).min(buffer.len());
                let chunk = &buffer[processed_samples..end];

                if let Ok(mut output) = resampler.process(&[chunk], None) {
                    if let Some(samples) = output.pop() {
                        if !samples.is_empty() {
                            let gain = if max_amplitude > 1.0 {
                                0.95 / max_amplitude
                            } else {
                                1.0
                            };

                            for sample in samples {
                                let normalized = sample * gain;
                                let sample_i16 =
                                    (normalized * i16::MAX as f32).clamp(-32768.0, 32767.0) as i16;
                                writer.write_sample(sample_i16).map_err(|e| {
                                    AudioError::Recording(format!("Failed to write sample: {}", e))
                                })?;
                                total_written += 1;
                            }
                        }
                    }
                }
                processed_samples += resampler_chunk_size;
            }

            if i % 10 == 0 {
                flush_println(&format!(
                    "Processed buffer {}/{} - Written: {} samples",
                    i + 1,
                    buffers.len(),
                    total_written
                ));
            }
        }

        if total_written == 0 {
            return Err(AudioError::Recording("No samples written".to_string()));
        }

        flush_println("Finalizing WAV file");
        writer
            .finalize()
            .map_err(|e| AudioError::Recording(format!("Failed to finalize WAV file: {}", e)))?;

        flush_println(&format!(
            "Recording saved - Samples: {}, Path: {}",
            total_written,
            output_path.display()
        ));

        Ok(())
    }
}

unsafe impl Send for AudioRecordingService {}
unsafe impl Sync for AudioRecordingService {}
