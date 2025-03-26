use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestUserMessage, ChatCompletionTool,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use serde_json::Value;
use std::error::Error;
use std::fmt;

// Custom error type for OpenAI service
#[derive(Debug)]
struct OpenAIError(String);

impl fmt::Display for OpenAIError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for OpenAIError {}

use super::language_model_service::{LanguageModelService, ToolCall, ToolCallResponse};

pub struct OpenAIService {
    client: Client<OpenAIConfig>,
    language_model: LanguageModelService,
}

impl OpenAIService {
    pub fn new(api_key: String, org_id: String) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_org_id(org_id);
        let client = Client::with_config(config);

        Self {
            client,
            language_model: LanguageModelService::new(),
        }
    }

    fn calculate_cost(&self, input_tokens: u64, output_tokens: u64) -> u64 {
        const INPUT_COST_PER_TOKEN: u64 = 15;
        const OUTPUT_COST_PER_TOKEN: u64 = 60;

        let input_cost = input_tokens * INPUT_COST_PER_TOKEN;
        let output_cost = output_tokens * OUTPUT_COST_PER_TOKEN;

        input_cost + output_cost
    }

    pub async fn execute_prompt_with_tools(
        &self,
        prompt: &str,
        tools: Vec<ChatCompletionTool>,
    ) -> Result<ToolCallResponse, Box<dyn Error + Send + Sync>> {
        self.language_model
            .execute_with_retry("Chat completion", || async {
                let mut binding = CreateChatCompletionRequestArgs::default();
                let mut request = binding
                    .model("gpt-4o")
                    .messages(vec![ChatCompletionRequestMessage::User(
                        ChatCompletionRequestUserMessage {
                            content: prompt.to_string().into(),
                            name: None,
                        },
                    )])
                    .temperature(0.1);

                if !tools.is_empty() {
                    request = request.tools(tools.clone()).tool_choice("auto".to_string());
                }

                let request = request.build()?;
                let response = self.client.chat().create(request).await?;

                if let Some(usage) = response.usage {
                    let cost = self
                        .calculate_cost(usage.prompt_tokens as u64, usage.completion_tokens as u64);
                    self.language_model.update_usage_stats(
                        usage.prompt_tokens as u64,
                        usage.completion_tokens as u64,
                        cost,
                    );
                }

                let message = response
                    .choices
                    .first()
                    .ok_or_else(|| OpenAIError("No response from OpenAI".to_string()))?
                    .message
                    .clone();

                let response_message = message.content.unwrap_or_default();

                let tool_calls = message
                    .tool_calls
                    .unwrap_or_default()
                    .into_iter()
                    .map(|tool_call| {
                        let function = tool_call.function;
                        ToolCall {
                            name: function.name,
                            arguments: serde_json::from_str(&function.arguments)
                                .unwrap_or(Value::Null),
                        }
                    })
                    .collect();

                Ok(ToolCallResponse {
                    tool_calls,
                    response_message,
                })
            })
            .await
    }

    pub async fn generate_embeddings(
        &self,
        text: &str,
    ) -> Result<Vec<f32>, Box<dyn Error + Send + Sync>> {
        self.language_model
            .execute_with_retry("Generate embeddings", || async {
                let request = async_openai::types::CreateEmbeddingRequestArgs::default()
                    .model("text-embedding-3-small")
                    .input(text)
                    .build()?;

                let response = self.client.embeddings().create(request).await?;

                let embeddings = response
                    .data
                    .first()
                    .ok_or_else(|| OpenAIError("No embeddings generated".to_string()))?
                    .embedding
                    .clone();

                Ok(embeddings)
            })
            .await
    }
}
