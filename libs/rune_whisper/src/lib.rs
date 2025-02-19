pub mod decoder;
pub mod model;
pub mod multilingual;
pub mod pcm_decode;

use anyhow::Result;
use candle::Device;
use candle::Tensor;
use candle_transformers::models::whisper::{self as m, Config};
use hf_hub::{api::sync::Api, Repo, RepoType};
use model::Model;
use model::WhichModel;
use std::path::PathBuf;
use tokenizers::Tokenizer;

pub struct WhisperConfig {
    pub model: WhichModel,
    pub device: Device,
    pub language: Option<String>,
    pub timestamps: bool,
    pub quantized: bool,
    pub seed: u64,
    pub model_id: Option<String>,
    pub revision: Option<String>,
    pub verbose: bool,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            model: WhichModel::Base,
            device: Device::Cpu,
            language: Some("en".to_string()),
            timestamps: false,
            quantized: false,
            seed: 299792458,
            model_id: None,
            revision: None,
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

        // Get model and revision info
        let (model_id, revision) = match (&config.model_id, &config.revision) {
            (Some(model_id), Some(revision)) => (model_id.clone(), revision.clone()),
            (Some(model_id), None) => (model_id.clone(), "main".to_string()),
            (None, Some(revision)) => {
                let (default_model, _) = if config.quantized {
                    ("lmz/candle-whisper", "main")
                } else {
                    config.model.model_and_revision()
                };
                (default_model.to_string(), revision.clone())
            }
            (None, None) => {
                let (default_model, default_revision) = if config.quantized {
                    ("lmz/candle-whisper", "main")
                } else {
                    config.model.model_and_revision()
                };
                (default_model.to_string(), default_revision.to_string())
            }
        };

        // Load model files
        let api = Api::new()?;
        let repo = api.repo(Repo::with_revision(model_id, RepoType::Model, revision));

        let (config_filename, tokenizer_filename, weights_filename) = if config.quantized {
            let ext = match config.model {
                WhichModel::TinyEn => "tiny-en",
                WhichModel::Tiny => "tiny",
                _ => unimplemented!("no quantized support for {:?}", config.model),
            };
            (
                repo.get(&format!("config-{ext}.json"))?,
                repo.get(&format!("tokenizer-{ext}.json"))?,
                repo.get(&format!("model-{ext}-q80.gguf"))?,
            )
        } else {
            (
                repo.get("config.json")?,
                repo.get("tokenizer.json")?,
                repo.get("model.safetensors")?,
            )
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
        let model = if config.quantized {
            let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(
                &weights_filename,
                device,
            )?;
            Model::Quantized(m::quantized_model::Whisper::load(&vb, model_config)?)
        } else {
            let vb = unsafe {
                candle_nn::VarBuilder::from_mmaped_safetensors(
                    &[weights_filename],
                    m::DTYPE,
                    device,
                )?
            };
            Model::Normal(m::model::Whisper::load(&vb, model_config)?)
        };

        Ok(Self {
            config,
            model: Some(model),
            tokenizer,
            mel_filters,
            device: device.clone(),
            language_token: None, // Will be set during first transcription if needed
        })
    }

    pub fn transcribe(&mut self, audio_path: PathBuf) -> Result<Vec<String>> {
        // Process audio
        let mut model = self
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

        // Detect language if needed (only on first run)
        if self.language_token.is_none() {
            self.language_token = match (self.config.model.is_multilingual(), &self.config.language)
            {
                (true, None) => Some(multilingual::detect_language(
                    &mut model,
                    &self.tokenizer,
                    &mel,
                )?),
                (false, None) => None,
                (true, Some(language)) => {
                    match decoder::token_id(&self.tokenizer, &format!("<|{language}|>")) {
                        Ok(token_id) => Some(token_id),
                        Err(_) => anyhow::bail!("language {language} is not supported"),
                    }
                }
                (false, Some(_)) => {
                    anyhow::bail!("a language cannot be set for non-multilingual models")
                }
            };
        }

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
