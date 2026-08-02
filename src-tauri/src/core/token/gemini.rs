use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::AppHandle;

use crate::services::gemini_oauth;

use super::{TokenAccuracy, TokenCount};

#[derive(Debug, Clone)]
pub struct GeminiCountError(pub String);

impl std::fmt::Display for GeminiCountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone)]
pub struct GeminiCountClient {
    app: AppHandle,
    cache: Arc<Mutex<HashMap<String, usize>>>,
}

impl GeminiCountClient {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn count(&self, model: &str, text: &str) -> Result<TokenCount, GeminiCountError> {
        let key = cache_key(model, text);
        if let Some(tokens) = self
            .cache
            .lock()
            .ok()
            .and_then(|cache| cache.get(&key).copied())
        {
            return Ok(exact_count(tokens));
        }

        let project = gemini_oauth::ensure_project_id_async(&self.app)
            .await
            .map_err(GeminiCountError)?;
        let access_token = gemini_oauth::ensure_access_token_async(&self.app)
            .await
            .map_err(GeminiCountError)?;
        let model = gemini_oauth::resolve_antigravity_model_id(model);
        let url = format!(
            "https://aiplatform.googleapis.com/v1/projects/{project}/locations/global/publishers/google/models/{model}:countTokens"
        );
        let response = gemini_oauth::antigravity_http_client()
            .map_err(GeminiCountError)?
            .post(url)
            .bearer_auth(access_token)
            .json(&CountTokensRequest {
                contents: vec![CountContent {
                    role: "user",
                    parts: vec![CountPart { text }],
                }],
            })
            .send()
            .await
            .map_err(|error| GeminiCountError(error.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| GeminiCountError(error.to_string()))?;
        if !status.is_success() {
            return Err(GeminiCountError(format!(
                "Gemini countTokens {status}: {body}"
            )));
        }
        let tokens = parse_count_tokens(&body)?;
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(key, tokens);
        }
        Ok(exact_count(tokens))
    }
}

#[derive(Serialize)]
struct CountTokensRequest<'a> {
    contents: Vec<CountContent<'a>>,
}

#[derive(Serialize)]
struct CountContent<'a> {
    role: &'static str,
    parts: Vec<CountPart<'a>>,
}

#[derive(Serialize)]
struct CountPart<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CountTokensResponse {
    total_tokens: usize,
}

fn parse_count_tokens(body: &str) -> Result<usize, GeminiCountError> {
    serde_json::from_str::<CountTokensResponse>(body)
        .map(|response| response.total_tokens)
        .map_err(|error| GeminiCountError(format!("invalid countTokens response: {error}")))
}

fn cache_key(model: &str, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(model.as_bytes());
    hasher.update([0]);
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn exact_count(tokens: usize) -> TokenCount {
    TokenCount {
        tokens,
        accuracy: TokenAccuracy::Exact,
        tokenizer: "google/countTokens".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_official_gemini_count_tokens_response() {
        assert_eq!(parse_count_tokens(r#"{"totalTokens": 17}"#).unwrap(), 17);
    }

    #[test]
    fn cache_key_is_model_aware_and_stable() {
        assert_eq!(
            cache_key("gemini-2.5-pro", "hello"),
            cache_key("gemini-2.5-pro", "hello")
        );
        assert_ne!(
            cache_key("gemini-2.5-pro", "hello"),
            cache_key("gemini-2.5-flash", "hello")
        );
    }
}
