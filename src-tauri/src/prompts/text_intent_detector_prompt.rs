use rune_llm::ToolDefinition;
use serde_json::json;

pub struct TextIntentDetectorPrompt;

impl TextIntentDetectorPrompt {
    pub fn get_tool() -> ToolDefinition {
        ToolDefinition {
            name: "detect_action_intent".to_string(),
            description:
                "Detects whether the input text contains a request for action or text generation"
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action_required": {
                        "type": "boolean",
                        "description": "Whether the text contains a request for action or text generation"
                    }
                },
                "required": ["action_required"],
                "additionalProperties": false
            }),
        }
    }

    pub fn get_prompt(text: &str) -> String {
        format!(
            r#"You are an AI assistant. Your task is to analyze if the following text contains a request for an action or text generation.

            Actions include but are not limited to:
            - Generating or creating new text/content
            - Modifying existing text/content
            - Performing calculations
            - Making changes to applications or files
            - Executing commands or operations
            - Requests for the AI to "do" something
            - Asking for some information not available in the context to ai

            Text to analyze: "{}"

            Determine if this text contains a request for an action.

            Set action_required to:
            - true if the text contains a request for any kind of action
            - false if the text is just a statement, question, or doesn't request any action

            NOTE: Use tool "detect_action_intent" for sending the response"#,
            text
        )
    }
}
