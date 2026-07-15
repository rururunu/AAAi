#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub jina_reader_base_url: String,
    pub jina_api_key: Option<String>,
}

impl RuntimeConfig {
    pub fn from_env() -> Self {
        Self {
            jina_reader_base_url: std::env::var("JINA_READER_BASE_URL")
                .unwrap_or_else(|_| "https://r.jina.ai".into())
                .trim_end_matches('/')
                .to_string(),
            jina_api_key: std::env::var("JINA_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty()),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
