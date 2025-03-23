use crate::core::error::AudioError;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream,
};
use crossbeam_channel::{bounded, Sender};
use parking_lot::Mutex;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

const TARGET_SAMPLE_RATE: u32 = 16000;
const LEVEL_UPDATE_INTERVAL: Duration = Duration::from_millis(50);
const DEFAULT_CHUNK_SIZE: usize = 4096;
const MIN_CHUNK_SIZE: usize = 1024;
const PRE_BUFFER_SIZE: usize = 1024;

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
            log::info!(
                "Chunk complete - Size: {}, Total chunks: {}",
                DEFAULT_CHUNK_SIZE,
                self.buffers.len()
            );
        }
    }

    fn finalize(&mut self) {
        if !self.current_chunk.is_empty() && self.current_chunk.len() >= MIN_CHUNK_SIZE {
            self.buffers.push(std::mem::take(&mut self.current_chunk));
            log::info!("Final chunk added - Total chunks: {}", self.buffers.len());
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
    pre_buffer: Arc<Mutex<Vec<f32>>>,
    pre_buffering: Arc<std::sync::atomic::AtomicBool>,
}

impl Default for AudioRecordingService {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecordingService {
    pub fn new() -> Self {
        log::info!("Initializing AudioRecordingService");
        let service = Self {
            state: Arc::new(Mutex::new(RecorderState {
                audio_data: Arc::new(Mutex::new(AudioData::new())),
                current_sample_rate: Arc::new(Mutex::new(0)),
                ..Default::default()
            })),
            last_level_update: Arc::new(Mutex::new(Instant::now())),
            audio_sender: Arc::new(Mutex::new(None)),
            recording_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            pre_buffer: Arc::new(Mutex::new(Vec::with_capacity(PRE_BUFFER_SIZE))),
            pre_buffering: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        service
    }

    // Add this new method to test the audio pipeline
    pub fn audio_check(&self) {
        log::info!("=== Testing Audio System ===");

        // Try to get information about the audio system
        let host = cpal::default_host();
        log::info!("Audio system: {}", host.id().name());

        // List available devices
        match host.devices() {
            Ok(devices) => {
                let mut input_count = 0;

                for device in devices {
                    if let Ok(name) = device.name() {
                        // Check if it's an input device
                        if device.default_input_config().is_ok() {
                            input_count += 1;

                            // Get and log the default config
                            if let Ok(config) = device.default_input_config() {
                                log::info!("Input device: {} (channels: {}, sample rate: {}, format: {:?})",
                                    name, config.channels(), config.sample_rate().0, config.sample_format());
                            } else {
                                log::info!("Input device: {} (no valid config)", name);
                            }
                        }
                    }
                }

                if input_count == 0 {
                    log::info!("No input devices found - audio recording may not work");
                } else {
                    log::info!("Found {} input devices", input_count);
                }
            }
            Err(e) => {
                log::info!("Failed to enumerate audio devices: {}", e);
            }
        }

        // Check default input device
        match host.default_input_device() {
            Some(device) => {
                if let Ok(name) = device.name() {
                    log::info!("Default input device: {}", name);
                } else {
                    log::info!("Default input device: <unnamed>");
                }

                // Test recording for 1 second
                log::info!("Testing 1-second recording...");

                let test_recording = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
                let recording_clone = test_recording.clone();
                let (tx, rx) = bounded::<Vec<f32>>(32);
                let mut received_data = Vec::new();

                // Set up receiver thread
                let receiver_thread = std::thread::spawn(move || {
                    while recording_clone.load(std::sync::atomic::Ordering::SeqCst) {
                        match rx.recv_timeout(Duration::from_millis(100)) {
                            Ok(pcm) => {
                                received_data.extend_from_slice(&pcm);
                            }
                            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                        }
                    }
                    received_data
                });

                // Try to record audio
                if let Ok(config) = device.default_input_config() {
                    let config_out = config.config();
                    let sample_format = config.sample_format();

                    log::info!(
                        "Recording at {} Hz with {} channels",
                        config_out.sample_rate.0,
                        config_out.channels
                    );

                    let stream = match sample_format {
                        cpal::SampleFormat::F32 => {
                            let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                let _ = tx.send(data.to_vec());
                            };

                            device.build_input_stream(
                                &config_out,
                                callback,
                                |err| log::info!("Audio test error: {}", err),
                                None,
                            )
                        }
                        _ => {
                            log::info!("Unsupported sample format for test: {:?}", sample_format);
                            Err(cpal::BuildStreamError::StreamConfigNotSupported)
                        }
                    };

                    match stream {
                        Ok(stream) => {
                            if let Err(e) = stream.play() {
                                log::info!("Failed to start test recording: {}", e);
                            } else {
                                // Record for 1 second
                                std::thread::sleep(Duration::from_secs(1));

                                // Stop the recording
                                test_recording.store(false, std::sync::atomic::Ordering::SeqCst);
                                let _ = stream.pause();

                                // Get the recorded data
                                let recorded_data = receiver_thread.join().unwrap_or_default();

                                if recorded_data.is_empty() {
                                    log::info!("❌ No audio data received during test");
                                } else {
                                    let max_level = recorded_data
                                        .iter()
                                        .fold(0.0f32, |max, &s| max.max(s.abs()));
                                    log::info!(
                                        "✅ Successfully recorded {} samples, max level: {:.6}",
                                        recorded_data.len(),
                                        max_level
                                    );

                                    if max_level < 0.01 {
                                        log::info!(
                                            "⚠️ Audio levels very low - microphone may be muted"
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::info!("Failed to build audio stream for test: {}", e);
                        }
                    }
                } else {
                    log::info!("No valid input configuration for test recording");
                }
            }
            None => {
                log::info!("No default input device found - audio recording may not work");
            }
        }

        log::info!("=== Audio System Test Complete ===");
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        log::info!("Setting app handle");
        let state = self.state.lock();
        state.audio_data.lock().app_handle = Some(handle);
    }

    pub fn set_device_id(&self, device_id: Option<String>) {
        log::info!("Setting device ID: {:?}", device_id);
        self.state.lock().device_id = device_id;
    }

    async fn get_input_device(&self) -> Result<Device, AudioError> {
        log::info!("\n=== Getting Input Device ===");
        let host = cpal::default_host();
        let state = self.state.lock();

        log::info!("\nAvailable audio devices:");
        let mut found_devices = Vec::new();
        if let Ok(devices) = host.devices() {
            for device in devices {
                if let Ok(name) = device.name() {
                    let supported_configs = device.supported_input_configs().map_err(|e| {
                        AudioError::Device(format!("Error getting configs for {}: {}", name, e))
                    })?;

                    log::info!("\nDevice: {}", name);
                    log::info!("Supported configurations:");
                    let mut config_count = 0;
                    for config in supported_configs {
                        config_count += 1;
                        log::info!("  - {:?}", config);
                    }

                    if device.default_input_config().is_ok() {
                        found_devices.push(device);
                        log::info!("  Total configs: {}", config_count);
                        log::info!("  (This is an input device)");
                    }
                }
            }
        }

        if let Some(device_id) = state.device_id.as_ref() {
            log::info!("\nLooking for device: {}", device_id);

            for device in found_devices.iter() {
                if let Ok(name) = device.name() {
                    if name == *device_id {
                        log::info!("Found exact match for device: {}", name);
                        return Ok(device.clone());
                    }
                }
            }

            for device in found_devices.iter() {
                if let Ok(name) = device.name() {
                    if name.contains(device_id) {
                        log::info!("Found partial match for device: {}", name);
                        return Ok(device.clone());
                    }
                }
            }

            if let Some(device) = found_devices.first() {
                log::info!(
                    "Device '{}' not found exactly, using first available input device: {}",
                    device_id,
                    device.name().unwrap_or_default()
                );
                return Ok(device.clone());
            }

            Err(AudioError::Device(format!(
                "No suitable input device found for '{}'",
                device_id
            )))
        } else {
            log::info!("\nNo device specified, using default input device");
            host.default_input_device()
                .ok_or_else(|| AudioError::Device("No default input device available".to_string()))
        }
    }

    pub async fn start_recording(&self, app_handle: &AppHandle) -> Result<(), AudioError> {
        log::info!("=== Starting Recording Process ===");
        log::info!("Audio system: {}", cpal::default_host().id().name());

        {
            let mut state = self.state.lock();
            if let Some(stream) = state.stream.take() {
                log::info!("Cleaning up previous audio stream");
                if let Err(e) = stream.pause() {
                    log::info!("Warning: Failed to pause stream: {}", e);
                }
                drop(stream);
            }

            if let Some(sender) = self.audio_sender.lock().take() {
                log::info!("Cleaning up previous audio channel");
                drop(sender);
            }

            self.recording_active
                .store(false, std::sync::atomic::Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        self.pre_buffering
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.pre_buffer.lock().clear();

        self.set_app_handle(app_handle.clone());
        *self.last_level_update.lock() = Instant::now()
            .checked_sub(LEVEL_UPDATE_INTERVAL * 2)
            .unwrap_or_else(Instant::now);

        let state = self.state.lock();
        {
            let mut audio_data = state.audio_data.lock();
            if audio_data.recording {
                log::info!("Error: Already recording");
                return Err(AudioError::Recording("Already recording".to_string()));
            }
            audio_data.recording = true;
            audio_data.buffers.clear();
            audio_data.current_chunk.clear();
            log::info!("Recording state initialized");
        }
        drop(state);

        let device = self.get_input_device().await?;

        let supported_configs = device
            .supported_input_configs()
            .map_err(|e| AudioError::Device(format!("Error getting supported configs: {}", e)))?;

        let supported_configs_vec: Vec<_> = supported_configs.collect();

        log::info!("\nSupported configurations:");
        for (i, config) in supported_configs_vec.iter().enumerate() {
            log::info!("Config {}: {:?}", i, config);
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
        log::info!(
            "Using {} channels at {} Hz",
            num_channels,
            native_sample_rate
        );

        let chunk_size = DEFAULT_CHUNK_SIZE * num_channels;
        let (tx, rx) = bounded::<Vec<f32>>(32);
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
        let _pre_buffer = Arc::clone(&self.pre_buffer);
        let _pre_buffering = Arc::clone(&self.pre_buffering);

        std::thread::spawn(move || {
            while recording_active.load(std::sync::atomic::Ordering::SeqCst) {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(pcm) => {
                        let mut audio_data = audio_data.lock();
                        if !audio_data.recording {
                            log::info!("Recording stopped");
                            continue;
                        }

                        let mono_samples: Vec<f32> = if pcm.len() % 2 == 0 && num_channels == 2 {
                            pcm.chunks(2)
                                .map(|chunk| (chunk[0] + chunk[1]) * 0.5)
                                .collect()
                        } else if num_channels > 2 {
                            pcm.chunks(num_channels)
                                .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
                                .collect()
                        } else {
                            pcm
                        };

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

                        audio_data.store_samples(&mono_samples);

                        if let Some(handle) = audio_data.app_handle.as_ref() {
                            let now = Instant::now();
                            let mut last_update = last_level_update_arc.lock();
                            if now.duration_since(*last_update) >= LEVEL_UPDATE_INTERVAL
                                || audio_data.buffers.len() < 5
                            {
                                log::info!("Audio levels: {:?}", levels);
                                if let Err(e) = handle.emit("audio-levels", levels) {
                                    log::info!("Failed to emit audio levels: {}", e);
                                }
                                *last_update = now;
                            }
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        if recording_active.load(std::sync::atomic::Ordering::SeqCst) {
                            log::info!("Channel disconnected but recording still active");
                            continue;
                        }
                        break;
                    }
                }
            }

            let mut audio_data = audio_data.lock();
            audio_data.finalize();
            log::info!(
                "Processing thread finished with {} chunks",
                audio_data.buffers.len()
            );
        });

        let sender = tx;
        let recording_active_clone = Arc::clone(&self.recording_active);
        let pre_buffer_clone = Arc::clone(&self.pre_buffer);
        let pre_buffering_clone = Arc::clone(&self.pre_buffering);

        let data_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if !recording_active_clone.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }

            let pcm = data.to_vec();

            if pre_buffering_clone.load(std::sync::atomic::Ordering::SeqCst) {
                let mut buffer = pre_buffer_clone.lock();
                buffer.extend_from_slice(&pcm);

                if buffer.len() >= PRE_BUFFER_SIZE {
                    pre_buffering_clone.store(false, std::sync::atomic::Ordering::SeqCst);
                    log::info!(
                        "Pre-buffering complete, discarded {} initial samples",
                        buffer.len()
                    );
                }
                return;
            }

            let _ = sender.send(pcm);
        };

        let error_callback = move |err| {
            log::info!("Audio input error: {}", err);
        };

        let stream = device
            .build_input_stream(&config.into(), data_callback, error_callback, None)
            .map_err(|e| {
                log::info!("Failed to build input stream: {}", e);
                AudioError::Recording(format!("Failed to build input stream: {}", e))
            })?;

        std::thread::sleep(std::time::Duration::from_millis(10));

        stream.play().map_err(|e| {
            log::info!("Failed to start stream: {}", e);
            AudioError::Recording(format!("Failed to start stream: {}", e))
        })?;

        self.state.lock().stream = Some(stream);
        log::info!("=== Recording Started Successfully ===");

        Ok(())
    }

    pub async fn stop_recording(&self, output_path: PathBuf) -> Result<(), AudioError> {
        log::info!("=== Stopping Recording ===");

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
                log::info!("Stopping audio stream");
                if let Err(e) = stream.pause() {
                    log::info!("Warning: Failed to pause stream: {}", e);
                }
            }
            sample_rate
        };

        {
            if let Some(sender) = self.audio_sender.lock().take() {
                log::info!("Cleaning up audio channel");
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

        log::info!(
            "Stop recording - Buffers: {}, Sample rate: {}",
            buffers.len(),
            native_sample_rate
        );

        if buffers.is_empty() {
            return Err(AudioError::Recording("No audio data recorded".to_string()));
        }

        if !buffers.is_empty() && buffers[0].len() > 0 {
            let max_val = buffers[0].iter().fold(0.0f32, |max, &s| max.max(s.abs()));
            log::info!("First buffer max value: {}", max_val);
        }

        let total_samples = buffers.iter().map(|b| b.len()).sum::<usize>();
        log::info!("Total samples recorded: {}", total_samples);

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: TARGET_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        log::info!("Creating WAV file: {}", output_path.display());
        let mut writer = hound::WavWriter::create(&output_path, spec)
            .map_err(|e| AudioError::Recording(format!("Failed to create WAV file: {}", e)))?;

        let ratio = TARGET_SAMPLE_RATE as f32 / native_sample_rate as f32;
        log::info!("Using simple resampling with ratio: {}", ratio);

        let mut total_written = 0;

        for buffer in buffers.iter() {
            if buffer.is_empty() {
                continue;
            }

            let samples_to_use = buffer;

            let step = 1.0 / ratio;
            let mut idx = 0.0;

            while idx < samples_to_use.len() as f32 {
                let i = idx as usize;
                if i < samples_to_use.len() {
                    let sample = samples_to_use[i];
                    let gain = 0.9;
                    let normalized = sample * gain;
                    let sample_i16 = (normalized * i16::MAX as f32).clamp(-32768.0, 32767.0) as i16;

                    writer.write_sample(sample_i16).map_err(|e| {
                        AudioError::Recording(format!("Failed to write sample: {}", e))
                    })?;
                    total_written += 1;
                }
                idx += step;
            }
        }

        if total_written == 0 {
            log::warn!(
                "No samples were written - generating test tone to verify file writing works"
            );
            for i in 0..16000 {
                let sample = (i as f32 * 0.1).sin() * 0.5;
                let sample_i16 = (sample * i16::MAX as f32) as i16;
                writer.write_sample(sample_i16).map_err(|e| {
                    AudioError::Recording(format!("Failed to write test sample: {}", e))
                })?;
                total_written += 1;
            }
        }

        log::info!("Finalizing WAV file with {} samples", total_written);
        writer
            .finalize()
            .map_err(|e| AudioError::Recording(format!("Failed to finalize WAV file: {}", e)))?;

        if total_written == 0 {
            return Err(AudioError::Recording("No samples written".to_string()));
        }

        log::info!(
            "Recording saved - Samples: {}, Path: {}",
            total_written,
            output_path.display()
        );

        Ok(())
    }

    pub async fn stop_recording_without_save(&self) -> Result<(), AudioError> {
        let log_tag = "=== Stopping Recording (No Save) ===";
        log::info!("{}", log_tag);

        self.recording_active
            .store(false, std::sync::atomic::Ordering::SeqCst);

        {
            let state = self.state.lock();
            let mut audio_data = state.audio_data.lock();
            if !audio_data.recording {
                log::info!("Not currently recording, nothing to clean up");
                return Ok(());
            }
            audio_data.recording = false;
            log::info!("Marked recording as stopped");
        }

        {
            let mut state = self.state.lock();
            if let Some(stream) = state.stream.take() {
                log::info!("Stopping audio stream");
                if let Err(e) = stream.pause() {
                    log::info!("Warning: Failed to pause stream: {}", e);
                }
            }
        }

        {
            if let Some(sender) = self.audio_sender.lock().take() {
                log::info!("Cleaning up audio channel");
                drop(sender);
            }
        }

        log::info!("Recording stopped without saving files");
        Ok(())
    }

    pub fn force_stop(&self) -> Result<(), AudioError> {
        log::info!("Force stopping audio recording");

        self.recording_active
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.pre_buffering
            .store(false, std::sync::atomic::Ordering::SeqCst);

        {
            let state = self.state.lock();
            let mut audio_data = state.audio_data.lock();
            if !audio_data.recording {
                return Ok(());
            }
            audio_data.recording = false;

            audio_data.buffers.clear();
            audio_data.current_chunk.clear();
        }

        {
            let mut state = self.state.lock();
            if let Some(stream) = state.stream.take() {
                let _ = stream.pause();
            }
        }

        {
            let _ = self.audio_sender.lock().take();
        }

        log::info!("Audio recording forcefully stopped");
        Ok(())
    }
}

unsafe impl Send for AudioRecordingService {}
unsafe impl Sync for AudioRecordingService {}
