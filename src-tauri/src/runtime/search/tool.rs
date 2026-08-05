use std::sync::Arc;

use chrono::Local;
use serde_json::{json, Value};

use crate::runtime::search::{SearchQuery, SearchRuntime};
use crate::runtime::tool::{Tool, ToolContext, ToolError};

pub struct SearchTool {
    runtime: Arc<SearchRuntime>,
}

impl SearchTool {
    pub fn new(runtime: Arc<SearchRuntime>) -> Self {
        Self { runtime }
    }
}

fn today_local() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

impl Tool for SearchTool {
    fn name(&self) -> &str {
        "web_search"
    }
    fn description(&self) -> &str {
        "Search the web and return structured result metadata. Prefer including today's date in time-sensitive queries. Use before browser_read when the user asks for current, recent, or externally verifiable information. Skip when repository evidence or stable knowledge already answers the question."
    }
    fn parameters_schema(&self) -> Value {
        let today = today_local();
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": format!(
                        "Search query. Today is {today} (local). For current events, news, prices, scores, or anything time-sensitive, include this date or year in the query."
                    )
                },
                "max_results": { "type": "integer", "minimum": 1, "maximum": 20, "default": 8 },
                "language": { "type": "string" },
                "freshness": { "type": "string", "enum": ["day", "week", "month", "year"] }
            },
            "required": ["query"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn available(&self) -> bool {
        self.runtime.is_available()
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let provider = self.runtime.provider().ok_or_else(|| {
            ToolError::new(
                "web search is not available; enable it in Settings and configure the selected provider API key",
            )
        })?;
        let today = today_local();
        let query = SearchQuery {
            query: args["query"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .to_string(),
            max_results: args["max_results"].as_u64().unwrap_or(8) as usize,
            language: args["language"].as_str().map(str::to_string),
            freshness: args["freshness"].as_str().map(str::to_string),
        };
        if query.query.is_empty() {
            return Err(ToolError::new("search query is required"));
        }
        serde_json::to_string_pretty(&json!({
            "provider": provider.id(),
            "asOf": today,
            "results": provider.search(&query)?,
        }))
        .map_err(|error| ToolError::new(error.to_string()))
    }

    fn schema(&self) -> Value {
        let today = today_local();
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": format!(
                    "Search the web and return structured result metadata. Today's date is {today} (local timezone). For current, recent, or time-sensitive information, include this date in the query (and prefer freshness=day/week when appropriate). Use before browser_read when snippets are insufficient."
                ),
                "parameters": self.parameters_schema(),
            }
        })
    }
}
