use crate::prompts::text_intent_detector_prompt::TextIntentDetectorPrompt;
use rune_llm::LLMClient;

pub struct ActionIntentDetectorService;

impl ActionIntentDetectorService {
    pub async fn detect_intent(llm_client: &LLMClient, text: &str) -> Result<bool, anyhow::Error> {
        let prompt = TextIntentDetectorPrompt::get_prompt(text);
        let tool = TextIntentDetectorPrompt::get_tool();

        let response = llm_client.execute_prompt(&prompt, vec![tool]).await?;

        for tool_call in &response.tool_calls {
            if tool_call.name == "detect_action_intent" {
                if let Some(action_required) = tool_call.arguments.get("action_required") {
                    return Ok(action_required.as_bool().unwrap_or(false));
                }
            }
        }

        if !response.message.is_empty() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&response.message) {
                if let Some(action_required) = value.get("action_required") {
                    return Ok(action_required.as_bool().unwrap_or(false));
                }
            }
        }

        Ok(false)
    }
}
