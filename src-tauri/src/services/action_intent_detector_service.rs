use crate::prompts::text_intent_detector_prompt::TextIntentDetectorPrompt;
use rune_llm::LLMClient;

pub struct ActionIntentDetectorService;

impl ActionIntentDetectorService {
    pub async fn detect_intent(
        llm_client: &parking_lot::MutexGuard<'_, LLMClient>,
        text: &str,
    ) -> Result<bool, anyhow::Error> {
        let prompt = TextIntentDetectorPrompt::get_prompt(text);
        let schema = TextIntentDetectorPrompt::get_schema();

        let response = llm_client
            .execute_prompt(&prompt, "intent_detector", Some(schema))
            .await?;

        Ok(response["action_required"].as_bool().unwrap_or(false))
    }
}
