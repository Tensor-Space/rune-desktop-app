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
        let schema = TextTransformerPrompt::get_schema();

        let response = llm_client
            .execute_prompt(&prompt, "text_transformer", Some(schema))
            .await?;

        Ok(response["output"].as_str().unwrap_or(text).to_string())
    }
}
