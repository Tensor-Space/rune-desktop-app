use crate::language_model::language_model_service::{LanguageModelService, ToolCallResponse};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: i32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(default)]
    text: String,
    #[serde(default)]
    r#type: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

pub struct AnthropicService {
    client: reqwest::Client,
    api_key: String,
    language_model: LanguageModelService,
}

impl AnthropicService {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            language_model: LanguageModelService::new(),
        }
    }

    pub async fn execute_prompt_with_tools(
        &self,
        prompt: &str,
        tools: Vec<Value>,
    ) -> Result<ToolCallResponse, Box<dyn Error + Send + Sync>> {
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let prompt = prompt.to_string();
        let tools = tools.clone();

        self.language_model
            .execute_with_retry("Anthropic chat completion", move || {
                let client = client.clone();
                let api_key = api_key.clone();
                let prompt = prompt.clone();
                let tools = tools.clone();

                async move {
                    let messages = vec![AnthropicMessage {
                        role: "user".to_string(),
                        content: prompt,
                    }];

                    let request = AnthropicRequest {
                        model: "claude-3-5-sonnet-latest".to_string(),
                        max_tokens: 1024,
                        messages,
                        temperature: Some(0.1),
                        tools,
                    };

                    let mut headers = HeaderMap::new();
                    headers.insert(
                        "x-api-key",
                        HeaderValue::from_str(&api_key).expect("Valid API key"),
                    );
                    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
                    headers.insert("content-type", HeaderValue::from_static("application/json"));

                    let response = client
                        .post("https://api.anthropic.com/v1/messages")
                        .headers(headers)
                        .json(&request)
                        .send()
                        .await?
                        .error_for_status()?
                        .json::<AnthropicResponse>()
                        .await?;

                    let response_message = response
                        .content
                        .iter()
                        .filter(|block| block.r#type == "text")
                        .map(|block| block.text.clone())
                        .collect::<Vec<String>>()
                        .join("");

                    let tool_calls = Vec::new();

                    Ok(ToolCallResponse {
                        tool_calls,
                        response_message,
                    })
                }
            })
            .await
    }
}
