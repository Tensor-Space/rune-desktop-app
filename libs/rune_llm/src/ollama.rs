use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use crate::LLMService;

pub struct OllamaService {
    client: Client,
}

impl OllamaService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
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
impl LLMService for OllamaService {
    async fn execute_prompt(
        &self,
        prompt: &str,
        _schema_name: &str,
        schema: Option<&str>,
    ) -> Result<Value> {
        let mut full_prompt = prompt.to_string();
        if let Some(schema_str) = schema {
            full_prompt = format!(
                "{}\nPlease provide the response in the following JSON schema:\n{}",
                prompt, schema_str
            );
        }

        let response = self
            .client
            .post("http://localhost:11434/api/generate")
            .json(&json!({
                "model": "llama2",
                "prompt": full_prompt,
                "stream": false
            }))
            .send()
            .await
            .context("Failed to send request to Ollama API")?;

        let result: Value = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        let content = result["response"]
            .as_str()
            .context("Invalid response format from Ollama")?;

        let parsed_content: Value =
            serde_json::from_str(content).context("Failed to parse response as JSON")?;

        if let Some(schema_str) = schema {
            self.validate_json(parsed_content, schema_str).await
        } else {
            Ok(parsed_content)
        }
    }
}
