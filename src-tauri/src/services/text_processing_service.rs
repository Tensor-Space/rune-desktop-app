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

        // Detect if we need an action (complex generation) or simple transformation
        let action_required = ActionIntentDetectorService::detect_intent(&llm_client, text).await?;

        // Process text based on intent detection
        let processed_text = if action_required {
            TextGeneratorService::generate(&llm_client, app_name, text).await?
        } else {
            TextTransformationService::transform(&llm_client, app_name, text).await?
        };

        Ok(processed_text)
    }

    pub fn inject_text(text: &str) -> Result<(), anyhow::Error> {
        TextInjectorService::inject_text(text)?;
        Ok(())
    }
}
