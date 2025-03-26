use crate::prompts::text_transformer_prompt::TextTransformerPrompt;
use rune_llm::LLMClient;

pub struct TextTransformationService;

impl TextTransformationService {
    pub async fn transform(
        llm_client: &LLMClient,
        app_name: &str,
        text: &str,
    ) -> Result<String, anyhow::Error> {
        let prompt = TextTransformerPrompt::get_prompt(app_name, text);
        let tool = TextTransformerPrompt::get_tool();

        let response = llm_client.execute_prompt(&prompt, vec![tool]).await?;

        for tool_call in &response.tool_calls {
            if tool_call.name == "transform_text" {
                if let Some(output) = tool_call.arguments.get("output") {
                    return Ok(output.as_str().unwrap_or(text).to_string());
                }
            }
        }

        if !response.message.is_empty() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&response.message) {
                if let Some(output) = value.get("output") {
                    return Ok(output.as_str().unwrap_or(text).to_string());
                }
            }
        }

        if !response.message.is_empty() {
            return Ok(response.message);
        }

        Ok(text.to_string())
    }
}
