use crate::runtime::tool::ToolError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub query: String,
    pub max_results: usize,
    pub language: Option<String>,
    pub freshness: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: String,
    pub published_at: Option<String>,
}

pub trait SearchProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, ToolError>;
}
