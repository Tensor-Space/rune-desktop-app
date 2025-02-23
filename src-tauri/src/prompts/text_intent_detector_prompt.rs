pub struct TextIntentDetectorPrompt;

impl TextIntentDetectorPrompt {
    pub fn get_schema() -> &'static str {
        r#"{
            "type": "object",
            "properties": {
                "action_required": {
                    "type": "boolean"
                }
            },
            "required": ["action_required"],
            "additionalProperties": false
        }"#
    }

    pub fn get_prompt(text: &str) -> String {
        format!(
            r#"You are an AI assistant. Your task is to analyze if the following text contains a request for an action.

            Actions include but are not limited to:
            - Generating or creating new text/content
            - Modifying existing text/content
            - Performing calculations
            - Making changes to applications or files
            - Executing commands or operations
            - Requests for the AI to "do" something

            Text to analyze: "{}"

            Determine if this text contains a request for an action.

            Set action_required to:
            - true if the text contains a request for any kind of action
            - false if the text is just a statement, question, or doesn't request any action

            Respond with valid JSON only."#,
            text
        )
    }
}
