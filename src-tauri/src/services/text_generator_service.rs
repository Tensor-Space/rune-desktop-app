use crate::prompts::text_generator_prompt::TextGeneratorPrompt;
use rune_llm::LLMClient;

pub struct TextGeneratorService;

impl TextGeneratorService {
    pub async fn generate(
        llm_client: &LLMClient,
        app_name: &str,
        text: &str,
    ) -> Result<String, anyhow::Error> {
        let prompt = TextGeneratorPrompt::get_prompt(app_name, text);
        let schema = TextGeneratorPrompt::get_schema();

        let response = llm_client
            .execute_prompt(&prompt, "text_generator", Some(schema))
            .await?;

        Ok(response["output"].as_str().unwrap_or(text).to_string())
    }
}
