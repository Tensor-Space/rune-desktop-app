use serde_json::Value;
use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;

#[derive(Debug)]
pub struct ToolCallResponse {
    pub tool_calls: Vec<ToolCall>,
    pub response_message: String,
}

#[derive(Debug)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug)]
pub struct UsageStats {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost: f64,
}

#[derive(Debug, Clone)]
pub struct ModelTool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Clone)]
pub struct LanguageModelService {
    max_retries: u32,
    initial_retry_delay: Duration,
    timeout_duration: Duration,
    input_tokens: Arc<AtomicU64>,
    output_tokens: Arc<AtomicU64>,
    total_cost: Arc<AtomicU64>,
}

impl LanguageModelService {
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            initial_retry_delay: Duration::from_millis(1000),
            timeout_duration: Duration::from_secs(180),
            input_tokens: Arc::new(AtomicU64::new(0)),
            output_tokens: Arc::new(AtomicU64::new(0)),
            total_cost: Arc::new(AtomicU64::new(0)),
        }
    }

    fn create_retry_strategy(&self) -> impl Iterator<Item = Duration> {
        ExponentialBackoff::from_millis(self.initial_retry_delay.as_millis() as u64)
            .factor(2)
            .max_delay(Duration::from_secs(30))
            .take(self.max_retries as usize)
    }

    pub async fn execute_with_retry<F, Fut, T>(
        &self,
        operation_name: &str,
        mut operation: F,
    ) -> Result<T, Box<dyn Error + Send + Sync>>
    where
        F: FnMut() -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, Box<dyn Error + Send + Sync>>> + Send,
        T: Send + 'static,
    {
        let retry_strategy = self.create_retry_strategy();
        let mut attempt = 0;

        let result = Retry::spawn(retry_strategy, move || {
            attempt += 1;
            let fut = operation();
            async move {
                match timeout(self.timeout_duration, fut).await {
                    Ok(result) => match result {
                        Ok(value) => Ok(value),
                        Err(e) => {
                            if attempt <= self.max_retries {
                                tracing::warn!(
                                    "{} failed (attempt {}/{}): {}. Retrying...",
                                    operation_name,
                                    attempt,
                                    self.max_retries,
                                    e
                                );
                                Err(e)
                            } else {
                                tracing::error!(
                                    "{} failed after {} attempts: {}",
                                    operation_name,
                                    self.max_retries,
                                    e
                                );
                                Err(e)
                            }
                        }
                    },
                    Err(e) => {
                        tracing::error!(
                            "{} timed out after {:?}",
                            operation_name,
                            self.timeout_duration
                        );
                        Err(Box::new(e) as Box<dyn Error + Send + Sync>)
                    }
                }
            }
        })
        .await?;

        Ok(result)
    }

    pub fn configure_retry(&mut self, max_retries: u32, initial_delay_ms: u64) {
        self.max_retries = max_retries;
        self.initial_retry_delay = Duration::from_millis(initial_delay_ms);
    }

    pub fn update_usage_stats(&self, input_tokens: u64, output_tokens: u64, cost: u64) {
        self.input_tokens.fetch_add(input_tokens, Ordering::Relaxed);
        self.output_tokens
            .fetch_add(output_tokens, Ordering::Relaxed);
        self.total_cost.fetch_add(cost, Ordering::Relaxed);
    }

    pub fn get_usage_stats(&self) -> UsageStats {
        UsageStats {
            input_tokens: self.input_tokens.load(Ordering::Relaxed),
            output_tokens: self.output_tokens.load(Ordering::Relaxed),
            cost: self.total_cost.load(Ordering::Relaxed) as f64 / 100_000_000.0,
        }
    }

    pub fn print_usage_stats(&self) {
        let stats = self.get_usage_stats();
        tracing::info!("API Usage Statistics:");
        tracing::info!("Input tokens: {}", stats.input_tokens);
        tracing::info!("Output tokens: {}", stats.output_tokens);
        tracing::info!("Total cost: ${:.4}", stats.cost);
    }
}
