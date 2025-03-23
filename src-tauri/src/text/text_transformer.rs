pub struct TextTransformer;

impl TextTransformer {
    pub async fn transform(text: &str) -> Result<String, anyhow::Error> {
        Ok(text.to_string())
    }
}
