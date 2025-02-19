use anyhow::Result;
use candle::Tensor;
use tokenizers::Tokenizer;

use crate::decoder::token_id;
use crate::model::Model;

// Language detection using the audio encoder's output and the tokenizer
pub fn detect_language(model: &mut Model, tokenizer: &Tokenizer, mel: &Tensor) -> Result<u32> {
    const SOT_TOKEN: &str = "<|startoftranscript|>";
    const LANGUAGE_TOKEN: &str = "<|language|>";

    // Get the tokens for special markers
    let sot_token = token_id(tokenizer, SOT_TOKEN)?;
    let language_token = token_id(tokenizer, LANGUAGE_TOKEN)?;

    // Run the encoder
    let features = model.encoder_forward(mel, true)?;

    // Prepare decoder input with just the SOT token
    let tokens = Tensor::new(&[sot_token], mel.device())?.unsqueeze(0)?;

    // Get decoder output
    let ys = model.decoder_forward(&tokens, &features, true)?;

    // Get logits and find the most likely language token
    let logits = model.decoder_final_linear(&ys)?;
    let logits = logits.squeeze(0)?;
    let logits = logits.squeeze(0)?;

    // Only consider language tokens
    let mut best_lang_token = None;
    let mut best_prob = f32::NEG_INFINITY;

    let logits_v: Vec<f32> = logits.to_vec1()?;
    for (token_id, &logit) in logits_v.iter().enumerate() {
        let token_id = token_id as u32;
        if token_id > language_token && token_id < language_token + 100 {
            if logit > best_prob {
                best_prob = logit;
                best_lang_token = Some(token_id);
            }
        }
    }

    best_lang_token.ok_or_else(|| anyhow::anyhow!("no language token found"))
}
