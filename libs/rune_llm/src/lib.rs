use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::timeout;

pub mod rune_api;

#[derive(Debug, Clone)]
pub enum LLMProvider {
    RuneAPI,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutePromptRequest {
    pub prompt: String,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutePromptResponse {
    pub message: String,
    pub tool_calls: Vec<ToolCallResult>,
}

#[async_trait]
pub trait LLMService {
    async fn execute_prompt(
        &self,
        prompt: &str,
        tools: Vec<ToolDefinition>,
    ) -> Result<ExecutePromptResponse>;
}

pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
        }
    }
}

pub struct LLMClientConfig {
    pub timeout: Duration,
    pub retry_config: RetryConfig,
}

impl Default for LLMClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(180),
            retry_config: RetryConfig::default(),
        }
    }
}

pub struct LLMClient {
    service: Box<dyn LLMService + Send + Sync>,
    config: LLMClientConfig,
}

impl LLMClient {
    pub fn new(provider: LLMProvider, config: Option<LLMClientConfig>) -> Self {
        let service: Box<dyn LLMService + Send + Sync> = match provider {
            LLMProvider::RuneAPI => Box::new(rune_api::RuneAPIService::new()),
        };

        Self {
            service,
            config: config.unwrap_or_default(),
        }
    }

    async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
        T: Send + 'static,
    {
        let mut retries = 0;
        let max_retries = self.config.retry_config.max_retries;
        let base_delay = self.config.retry_config.base_delay;

        loop {
            match timeout(self.config.timeout, operation()).await {
                Ok(result) => {
                    match result {
                        Ok(value) => return Ok(value),
                        Err(e) => {
                            if retries >= max_retries {
                                return Err(e.context(format!(
                                    "Operation failed after {} retries",
                                    retries
                                )));
                            }

                            // Log the error
                            log::error!("Attempt {} failed: {}", retries + 1, e);

                            // Exponential backoff
                            let delay = base_delay * 2u32.pow(retries);
                            tokio::time::sleep(delay).await;
                            retries += 1;
                        }
                    }
                }
                Err(_) => {
                    if retries >= max_retries {
                        return Err(anyhow::anyhow!(
                            "Operation timed out after {} retries",
                            retries
                        ));
                    }

                    log::error!("Attempt {} timed out", retries + 1);

                    let delay = base_delay * 2u32.pow(retries);
                    tokio::time::sleep(delay).await;
                    retries += 1;
                }
            }
        }
    }

    pub async fn execute_prompt(
        &self,
        prompt: &str,
        tools: Vec<ToolDefinition>,
    ) -> Result<ExecutePromptResponse> {
        let prompt_string = prompt.to_string();
        let tools_clone = tools;

        self.execute_with_retry(|| async {
            self.service
                .execute_prompt(&prompt_string, tools_clone.clone())
                .await
        })
        .await
    }

    pub fn with_config(mut self, config: LLMClientConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.config.retry_config = retry_config;
        self
    }
}
