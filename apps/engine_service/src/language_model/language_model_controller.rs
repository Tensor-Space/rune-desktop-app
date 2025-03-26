use async_openai::types::{ChatCompletionTool, ChatCompletionToolType};
use axum::{http::StatusCode, response::IntoResponse, routing::post, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::openai_service::OpenAIService;
use crate::app_module::AppState;

#[derive(Debug, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Deserialize)]
pub struct ExecutePromptRequest {
    pub prompt: String,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Serialize)]
pub struct ToolCallResult {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Serialize)]
pub struct ExecutePromptResponse {
    pub message: String,
    pub tool_calls: Vec<ToolCallResult>,
}

pub fn language_model_router() -> axum::Router {
    Router::new()
        .route("/execute", post(execute_prompt))
        .with_state(())
}

pub async fn execute_prompt(
    Extension(_ctx): Extension<AppState>,
    Json(request): Json<ExecutePromptRequest>,
) -> impl IntoResponse {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let org_id = std::env::var("OPENAI_ORG_ID").unwrap_or_default();

    if api_key.is_empty() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "OpenAI API key not configured"
            })),
        );
    }

    let openai_service = OpenAIService::new(api_key, org_id);

    let tools: Vec<ChatCompletionTool> = request
        .tools
        .iter()
        .map(|tool| ChatCompletionTool {
            r#type: ChatCompletionToolType::Function,
            function: async_openai::types::FunctionObject {
                name: tool.name.clone(),
                description: Some(tool.description.clone()),
                parameters: Some(tool.parameters.clone()),
                strict: Some(true),
            },
        })
        .collect();

    match openai_service
        .execute_prompt_with_tools(&request.prompt, tools)
        .await
    {
        Ok(result) => {
            let response = ExecutePromptResponse {
                message: result.response_message,
                tool_calls: result
                    .tool_calls
                    .into_iter()
                    .map(|call| ToolCallResult {
                        name: call.name,
                        arguments: call.arguments,
                    })
                    .collect(),
            };

            match serde_json::to_value(response) {
                Ok(json_value) => (StatusCode::OK, Json(json_value)),
                Err(e) => {
                    tracing::error!("Error serializing response: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": format!("Failed to serialize response: {}", e)
                        })),
                    )
                }
            }
        }
        Err(e) => {
            tracing::error!("Error executing prompt: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to execute prompt: {}", e)
                })),
            )
        }
    }
}
