use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;

use crate::runtime::search::{SearchProvider, SearchQuery, SearchResult};
use crate::runtime::tool::ToolError;

pub struct SerperProvider {
    api_key: String,
    client: Client,
}

impl SerperProvider {
    pub fn new(api_key: impl Into<String>) -> Result<Self, ToolError> {
        let api_key = api_key.into().trim().to_string();
        if api_key.is_empty() {
            return Err(ToolError::new("Serper API key is required"));
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(12))
            .user_agent("AltAltAi-Runtime/3")
            .build()
            .map_err(|error| ToolError::new(error.to_string()))?;
        Ok(Self { api_key, client })
    }
}

#[derive(Deserialize)]
struct SerperResponse {
    #[serde(default)]
    organic: Vec<SerperOrganic>,
}

#[derive(Deserialize)]
struct SerperOrganic {
    #[serde(default)]
    title: String,
    #[serde(default)]
    link: String,
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    date: Option<String>,
}

fn freshness_to_tbs(freshness: &str) -> Option<&'static str> {
    match freshness {
        "day" => Some("qdr:d"),
        "week" => Some("qdr:w"),
        "month" => Some("qdr:m"),
        "year" => Some("qdr:y"),
        _ => None,
    }
}

impl SearchProvider for SerperProvider {
    fn id(&self) -> &'static str {
        "serper"
    }

    fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, ToolError> {
        let max_results = query.max_results.clamp(1, 20);
        let mut body = json!({
            "q": query.query,
            "num": max_results,
        });
        if let Some(language) = &query.language {
            body["hl"] = json!(language);
        }
        if let Some(freshness) = query.freshness.as_deref().and_then(freshness_to_tbs) {
            body["tbs"] = json!(freshness);
        }
        let response = self
            .client
            .post("https://google.serper.dev/search")
            .header("X-API-KEY", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| ToolError::new(format!("Serper request failed: {error}")))?;
        let payload: SerperResponse = response
            .json()
            .map_err(|error| ToolError::new(format!("invalid Serper response: {error}")))?;
        Ok(payload
            .organic
            .into_iter()
            .filter(|item| !item.link.is_empty())
            .take(max_results)
            .map(|item| {
                let source = reqwest::Url::parse(&item.link)
                    .ok()
                    .and_then(|url| url.host_str().map(str::to_string))
                    .unwrap_or_default();
                SearchResult {
                    title: item.title,
                    url: item.link,
                    snippet: item.snippet,
                    source,
                    published_at: item.date,
                }
            })
            .collect())
    }
}
