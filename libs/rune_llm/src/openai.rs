use anyhow::{Context, Result};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestUserMessage,
        CreateChatCompletionRequestArgs, ResponseFormat, ResponseFormatJsonSchema,
    },
    Client,
};
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use tokio::time::timeout;

use crate::LLMService;

pub struct OpenAIService {
    client: Client<OpenAIConfig>,
}

impl OpenAIService {
    pub fn new(api_key: String, org_id: String) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_org_id(org_id);
        let client = Client::with_config(config);
        Self { client }
    }

    async fn validate_json(&self, value: Value, schema: &str) -> Result<Value> {
        let schema: Value = serde_json::from_str(schema).context("Failed to parse JSON schema")?;
        let validator =
            jsonschema::validator_for(&schema).context("Failed to compile JSON schema")?;

        let parsed_value: Value =
            serde_json::from_str(&value.to_string()).context("Failed to parse response as JSON")?;

        if validator.is_valid(&parsed_value) {
            Ok(parsed_value)
        } else {
            Err(anyhow::anyhow!("Response does not match JSON schema"))
        }
    }
}

#[async_trait]
impl LLMService for OpenAIService {
    async fn execute_prompt(
        &self,
        prompt: &str,
        schema_name: &str,
        schema: Option<&str>,
    ) -> Result<Value> {
        let mut full_prompt = prompt.to_string();
        if let Some(schema_str) = schema {
            full_prompt = format!(
                "{}\nPlease provide the response in the following JSON schema:\n{}",
                prompt, schema_str
            );
        }

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .response_format(ResponseFormat::JsonSchema {
                json_schema: ResponseFormatJsonSchema {
                    name: schema_name.to_string(),
                    schema: schema.map(|s| serde_json::from_str(s).unwrap()),
                    description: None,
                    strict: Some(true),
                },
            })
            .messages([ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage {
                    content: full_prompt.into(),
                    name: None,
                },
            )])
            .temperature(0.1)
            .build()
            .context("Failed to build chat completion request")?;

        let response = timeout(Duration::from_secs(180), self.client.chat().create(request))
            .await
            .unwrap()
            .unwrap();
        // .context("Request timed out")?
        // .context("OpenAI API request failed")?;

        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .context("No content in response")?;

        let parsed_content: Value =
            serde_json::from_str(&content).context("Failed to parse response as JSON")?;

        if let Some(schema_str) = schema {
            self.validate_json(parsed_content, schema_str).await
        } else {
            Ok(parsed_content)
        }
    }
}
