pub mod decoder;
pub mod model;
pub mod multilingual;
pub mod pcm_decode;

use anyhow::Result;
use candle::Device;
use candle::Tensor;
use candle_transformers::models::whisper::{self as m, Config};
use model::Model;
use std::path::PathBuf;
use tokenizers::Tokenizer;

pub struct WhisperConfig {
    pub device: Device,
    pub timestamps: bool,
    pub seed: u64,
    pub model_dir: Option<PathBuf>,
    pub verbose: bool,
}

impl WhisperConfig {
    pub fn new(model_dir: Option<PathBuf>) -> Self {
        Self {
            device: Device::Cpu,
            timestamps: false,
            seed: 299792458,
            model_dir,
            verbose: false,
        }
    }
}

pub struct Whisper {
    config: WhisperConfig,
    model: Option<Model>,
    tokenizer: Tokenizer,
    mel_filters: Vec<f32>,
    device: Device,
    language_token: Option<u32>,
}

impl Whisper {
    pub fn new(config: WhisperConfig) -> Result<Self> {
        let device = &config.device.clone();

        // Get paths to model files
        let (config_filename, tokenizer_filename, weights_filename) =
            if let Some(model_dir) = &config.model_dir {
                (
                    model_dir.join("config.json"),
                    model_dir.join("tokenizer.json"),
                    model_dir.join("model.safetensors"),
                )
            } else {
                anyhow::bail!("model_dir must be specified")
            };

        // Load configuration and tokenizer
        let model_config: Config =
            serde_json::from_str(&std::fs::read_to_string(config_filename)?)?;
        let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(anyhow::Error::msg)?;

        // Load mel filters
        let mel_bytes = match model_config.num_mel_bins {
            80 => include_bytes!("melfilters.bytes").as_slice(),
            128 => include_bytes!("melfilters128.bytes").as_slice(),
            nmel => anyhow::bail!("unexpected num_mel_bins {nmel}"),
        };
        let mut mel_filters = vec![0f32; mel_bytes.len() / 4];
        <byteorder::LittleEndian as byteorder::ByteOrder>::read_f32_into(
            mel_bytes,
            &mut mel_filters,
        );

        // Create model
        let vb = unsafe {
            candle_nn::VarBuilder::from_mmaped_safetensors(&[weights_filename], m::DTYPE, device)?
        };
        let model = Model::Normal(m::model::Whisper::load(&vb, model_config)?);

        // Get English language token
        let language_token = decoder::token_id(&tokenizer, "<|en|>")
            .map_err(|_| anyhow::anyhow!("Failed to get English language token"))?;

        Ok(Self {
            config,
            model: Some(model),
            tokenizer,
            mel_filters,
            device: device.clone(),
            language_token: Some(language_token),
        })
    }

    pub fn transcribe(&mut self, audio_path: PathBuf) -> Result<Vec<String>> {
        // Process audio
        let model = self
            .model
            .take()
            .ok_or_else(|| anyhow::anyhow!("Model not available"))?;

        let (pcm_data, sample_rate) = pcm_decode::pcm_decode(audio_path)?;
        if sample_rate != m::SAMPLE_RATE as u32 {
            anyhow::bail!("input file must have a {} sampling rate", m::SAMPLE_RATE)
        }

        let mel = m::audio::pcm_to_mel(model.config(), &pcm_data, &self.mel_filters);
        let mel_len = mel.len();
        let mel = Tensor::from_vec(
            mel,
            (
                1,
                model.config().num_mel_bins,
                mel_len / model.config().num_mel_bins,
            ),
            &self.device,
        )?;

        // Create decoder and run transcription
        let mut dc = decoder::Decoder::new(
            model,
            self.tokenizer.clone(),
            self.config.seed,
            &self.device,
            self.language_token,
            self.config.timestamps,
            self.config.verbose,
        )?;

        let segments = dc.run(&mel)?;

        // Get model back from decoder
        self.model = Some(dc.take_model());

        Ok(segments.into_iter().map(|s| s.dr.text).collect())
    }
}
