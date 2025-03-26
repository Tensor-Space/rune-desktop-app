pub struct TextGeneratorPrompt;

impl TextGeneratorPrompt {
    pub fn get_schema() -> &'static str {
        r#"{
            "type": "object",
            "properties": {
                "output": {
                    "type": "string"
                },
                "action_required": {
                    "type": "boolean"
                }
            },
            "required": ["output", "action_required"],
            "additionalProperties": false
        }"#
    }

    pub fn get_prompt(app_name: &str, text: &str) -> String {
        format!(
            r#"You are a helpful assistant that generates content based on voice input.
            The following is a voice input recorded in {}:

"{}"

Instructions:
1. Interpret the input as an instruction and generate appropriate content
2. Even if the instruction is indirect or implicit, create relevant content
3. Format the output appropriately for {}:
   - For email app: Generate professional email content
   - For notes app: Create structured, detailed notes
   - For messaging app: Generate conversational messages
   - For document app: Create well-formatted document content
4. Maintain consistent tone and style suitable for {}
5. Always provide meaningful, contextual content

Generate the content without any explanations or meta-commentary.
Ensure the output is complete and ready for use in {}."#,
            app_name, text, app_name, app_name, app_name
        )
    }
}
