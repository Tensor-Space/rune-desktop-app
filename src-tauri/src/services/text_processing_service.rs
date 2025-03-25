use crate::core::app::AppState;
use crate::services::{
    text_generator_service::TextGeneratorService, text_injector_service::TextInjectorService,
    text_transformation_service::TextTransformationService,
};
use std::sync::Arc;

use super::action_intent_detector_service::ActionIntentDetectorService;

pub struct TextProcessingService;

impl TextProcessingService {
    pub async fn process_text(
        state: &Arc<AppState>,
        app_name: &str,
        text: &str,
    ) -> Result<String, anyhow::Error> {
        let llm_client = state.llm.lock();

        let action_required = match &*llm_client {
            Some(client) => ActionIntentDetectorService::detect_intent(client, text).await?,
            None => return Err(anyhow::anyhow!("LLM client not initialized")),
        };

        let processed_text = if action_required {
            match &*llm_client {
                Some(client) => TextGeneratorService::generate(client, app_name, text).await?,
                None => return Err(anyhow::anyhow!("LLM client not initialized")),
            }
        } else {
            match &*llm_client {
                Some(client) => {
                    TextTransformationService::transform(client, app_name, text).await?
                }
                None => return Err(anyhow::anyhow!("LLM client not initialized")),
            }
        };

        Ok(processed_text)
    }

    pub fn inject_text(text: &str) -> Result<(), anyhow::Error> {
        TextInjectorService::inject_text(text)?;
        Ok(())
    }
}
