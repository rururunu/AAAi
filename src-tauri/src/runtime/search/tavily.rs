use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;

use crate::runtime::search::{SearchProvider, SearchQuery, SearchResult};
use crate::runtime::tool::ToolError;

pub struct TavilyProvider {
    api_key: String,
    client: Client,
}

impl TavilyProvider {
    pub fn new(api_key: impl Into<String>) -> Result<Self, ToolError> {
        let api_key = normalize_api_key(api_key.into());
        if api_key.is_empty() {
            return Err(ToolError::new("Tavily API key is required"));
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(12))
            .user_agent("Anya-Runtime/3")
            .build()
            .map_err(|error| ToolError::new(error.to_string()))?;
        Ok(Self { api_key, client })
    }
}

fn normalize_api_key(raw: String) -> String {
    let mut key = raw.trim().to_string();
    if (key.starts_with('"') && key.ends_with('"'))
        || (key.starts_with('\'') && key.ends_with('\''))
    {
        key = key[1..key.len().saturating_sub(1)].trim().to_string();
    }
    if let Some(rest) = key
        .strip_prefix("Bearer ")
        .or_else(|| key.strip_prefix("bearer "))
    {
        key = rest.trim().to_string();
    }
    key
}

#[derive(Deserialize)]
struct TavilyResponse {
    #[serde(default)]
    results: Vec<TavilyResult>,
}

#[derive(Deserialize)]
struct TavilyResult {
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    published_date: Option<String>,
}

impl SearchProvider for TavilyProvider {
    fn id(&self) -> &'static str {
        "tavily"
    }

    fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, ToolError> {
        let max_results = query.max_results.clamp(1, 20);
        let mut body = json!({
            "api_key": self.api_key,
            "query": query.query,
            "max_results": max_results,
        });
        if let Some(freshness) = &query.freshness {
            body["time_range"] = json!(freshness);
        }
        let response = self
            .client
            .post("https://api.tavily.com/search")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|error| ToolError::new(format!("Tavily request failed: {error}")))?;
        let status = response.status();
        if status.as_u16() == 401 {
            return Err(ToolError::new(
                "Tavily authentication failed (401). Check that the Tavily API key in Settings is valid and starts with tvly-.",
            ));
        }
        if !status.is_success() {
            let detail = response
                .text()
                .unwrap_or_default()
                .chars()
                .take(300)
                .collect::<String>();
            return Err(ToolError::new(format!(
                "Tavily request failed ({status}): {detail}"
            )));
        }
        let payload: TavilyResponse = response
            .json()
            .map_err(|error| ToolError::new(format!("invalid Tavily response: {error}")))?;
        Ok(payload
            .results
            .into_iter()
            .filter(|item| !item.url.is_empty())
            .take(max_results)
            .map(|item| {
                let source = reqwest::Url::parse(&item.url)
                    .ok()
                    .and_then(|url| url.host_str().map(str::to_string))
                    .unwrap_or_default();
                SearchResult {
                    title: item.title,
                    url: item.url,
                    snippet: item.content,
                    source,
                    published_at: item.published_date,
                }
            })
            .collect())
    }
}
