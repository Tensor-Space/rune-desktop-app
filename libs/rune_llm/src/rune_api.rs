use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::{ExecutePromptRequest, ExecutePromptResponse, LLMService, ToolDefinition};

pub struct RuneAPIService {
    client: Client,
}

impl RuneAPIService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl LLMService for RuneAPIService {
    async fn execute_prompt(
        &self,
        prompt: &str,
        tools: Vec<ToolDefinition>,
    ) -> Result<ExecutePromptResponse> {
        let request = ExecutePromptRequest {
            prompt: prompt.to_string(),
            tools,
        };

        let response = self
            .client
            .post("https://api.runeapp.ai/engine/v1/language-model/execute")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Rune API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "API returned error status: {}, body: {}",
                status,
                error_text
            ));
        }

        let result: Value = response
            .json()
            .await
            .context("Failed to parse Rune API response")?;

        let response: ExecutePromptResponse = serde_json::from_value(result)
            .context("Failed to parse response into ExecutePromptResponse")?;

        Ok(response)
    }
}
