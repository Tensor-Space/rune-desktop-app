use crate::{
    core::app::AppState,
    text::{
        text_generator::TextGenerator, text_injector::TextInjector,
        text_intent_detector::TextIntentDetector, text_transformer::TextTransformer,
    },
};
use std::sync::Arc;

pub struct TextProcessorPipeline;

impl TextProcessorPipeline {
    pub async fn process_text(
        state: &Arc<AppState>,
        app_name: &str,
        text: &str,
    ) -> Result<String, anyhow::Error> {
        let llm_client = state.llm_client.lock();

        let action_required = TextIntentDetector::detect_intent(&llm_client, text).await?;

        let processed_text = if action_required {
            TextGenerator::generate(&llm_client, app_name, text).await?
        } else {
            TextTransformer::transform(&llm_client, app_name, text).await?
        };

        Ok(processed_text)
    }

    pub fn inject_text(text: &str) -> Result<(), anyhow::Error> {
        if let Ok(injector) = TextInjector::new() {
            injector.inject_text(text)?;
        }
        Ok(())
    }
}
