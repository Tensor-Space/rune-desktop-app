use rune_llm::ToolDefinition;
use serde_json::json;

pub struct TextTransformerPrompt;

impl TextTransformerPrompt {
    pub fn get_tool() -> ToolDefinition {
        ToolDefinition {
            name: "transform_text".to_string(),
            description: "Transforms and formats text based on context".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "output": {
                        "type": "string",
                        "description": "The transformed text"
                    }
                },
                "required": ["output"],
                "additionalProperties": false
            }),
        }
    }

    pub fn get_prompt(app_name: &str, text: &str) -> String {
        format!(
            r#"You are a helpful assistant that processes voice input for {}.
            The following is a voice input recorded in {}:

"{}"

Instructions:
1. Fix any grammar or transcription errors in the text
2. Format the text appropriately for {} usage:
   - If it's an email app, ensure proper email formatting
   - If it's a notes app, structure it as a clear note
   - If it's a messaging app, format as a conversational message
   - If it's a document app, format with proper paragraph structure
3. Maintain the original meaning and intent
4. Clean up the text while keeping it natural for the context
5. If you see any obvious mistakes in choice of words or phrases, proide alternatives that improve clarity and coherence.

Provide only the corrected and contextually formatted text without any explanations or meta-commentary using tool call."#,
            app_name, app_name, text, app_name
        )
    }
}
