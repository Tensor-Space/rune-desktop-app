use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use crate::LLMService;

pub struct AnthropicService {
    client: Client,
    api_key: String,
}

impl AnthropicService {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
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
impl LLMService for AnthropicService {
    async fn execute_prompt(&self, prompt: &str, schema: Option<&str>) -> Result<Value> {
        let mut full_prompt = prompt.to_string();
        if let Some(schema_str) = schema {
            full_prompt = format!(
                "{}\nPlease provide the response in the following JSON schema:\n{}",
                prompt, schema_str
            );
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": "claude-3-opus-20240229",
                "max_tokens": 1024,
                "messages": [{
                    "role": "user",
                    "content": full_prompt
                }]
            }))
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;

        let result: Value = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        let content = result["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|msg| msg["text"].as_str())
            .context("Invalid response format from Anthropic")?;

        let parsed_content: Value =
            serde_json::from_str(content).context("Failed to parse response as JSON")?;

        if let Some(schema_str) = schema {
            self.validate_json(parsed_content, schema_str).await
        } else {
            Ok(parsed_content)
        }
    }
}
