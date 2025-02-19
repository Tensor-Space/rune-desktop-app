use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use rubato::{FastFixedIn, PolynomialDegree, Resampler};
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tauri_plugin_fs::FsExt;

pub struct AudioRecorder {
    recording: Arc<Mutex<bool>>,
    writer: Arc<Mutex<Option<WavWriter<BufWriter<std::fs::File>>>>>,
    stream: Arc<Mutex<Option<cpal::Stream>>>,
    buffered_pcm: Arc<Mutex<Vec<f32>>>,
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl AudioRecorder {
    pub fn set_app_handle(&self, handle: AppHandle) {
        *self.app_handle.lock().unwrap() = Some(handle);
    }

    pub fn start_recording(
        &self,
        app_handle: &AppHandle,
        output_path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_app_handle(app_handle.clone());

        // Ensure the parent directory exists in temp
        let fs_scope = app_handle.fs_scope();
        fs_scope.allow_directory(&output_path.parent().unwrap(), true)?;

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("Failed to get default input device")?;

        let input_config = device.default_input_config()?;
        let config = input_config.config();

        let channel_count = config.channels as usize;
        let in_sample_rate = config.sample_rate.0 as usize;

        // Create resampler with better configuration
        let resample_ratio = 16000. / in_sample_rate as f64;
        let mut resampler =
            FastFixedIn::new(resample_ratio, 10., PolynomialDegree::Septic, 1024, 1)?;

        // WAV file specifications
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        // Create the file using Tauri's fs plugin
        let file = std::fs::File::create(&output_path)?;
        let writer = WavWriter::new(BufWriter::new(file), spec)?;

        *self.writer.lock().unwrap() = Some(writer);
        *self.recording.lock().unwrap() = true;

        let writer_clone = Arc::clone(&self.writer);
        let recording = Arc::clone(&self.recording);
        let buffered_pcm = Arc::clone(&self.buffered_pcm);
        let app_handle = Arc::clone(&self.app_handle);

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if *recording.lock().unwrap() {
                    let pcm: Vec<f32> = data.iter().step_by(channel_count).copied().collect();
                    let chunk_size = pcm.len() / 10;
                    let mut levels = vec![0.0; 10];

                    for (i, chunk) in pcm.chunks(chunk_size).enumerate() {
                        if i < 10 {
                            let rms = (chunk.iter().map(|sample| sample * sample).sum::<f32>()
                                / chunk.len() as f32)
                                .sqrt();
                            levels[i] = rms;
                        }
                    }

                    if let Some(handle) = app_handle.lock().unwrap().as_ref() {
                        handle.emit("audio-levels", levels).unwrap_or_default();
                    }

                    if !pcm.is_empty() {
                        let mut buffer = buffered_pcm.lock().unwrap();
                        buffer.extend_from_slice(&pcm);

                        if buffer.len() >= 1024 {
                            let full_chunks = buffer.len() / 1024;

                            for chunk in 0..full_chunks {
                                let start = chunk * 1024;
                                let end = (chunk + 1) * 1024;
                                let chunk_data = &buffer[start..end];

                                let output =
                                    resampler.process(&[chunk_data], None).unwrap_or_default();

                                if let Some(writer) = writer_clone.lock().unwrap().as_mut() {
                                    for &sample in &output[0] {
                                        let sample = (sample * i16::MAX as f32) as i16;
                                        writer.write_sample(sample).unwrap_or_default();
                                    }
                                }
                            }

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
            move |err| eprintln!("An error occurred on the input audio stream: {}", err),
            None,
        )?;

        stream.play()?;
        *self.stream.lock().unwrap() = Some(stream);

        Ok(())
    }

    pub fn stop_recording(&self) -> Result<(), Box<dyn std::error::Error>> {
        *self.recording.lock().unwrap() = false;
        *self.stream.lock().unwrap() = None;

        if let Some(writer) = self.writer.lock().unwrap().take() {
            writer.finalize()?;
        }

        Ok(())
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self {
            recording: Arc::new(Mutex::new(false)),
            writer: Arc::new(Mutex::new(None)),
            stream: Arc::new(Mutex::new(None)),
            buffered_pcm: Arc::new(Mutex::new(Vec::new())),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }
}

unsafe impl Send for AudioRecorder {}
unsafe impl Sync for AudioRecorder {}
