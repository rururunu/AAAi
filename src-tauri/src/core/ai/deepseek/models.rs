use serde::Deserialize;

use crate::models::chat::ChatModelInfo;

use super::ProviderError;

const MODELS_URL: &str = "https://api.deepseek.com/models";

#[derive(Debug, Deserialize)]
struct ApiModelsResponse {
    data: Vec<ApiModelItem>,
}

#[derive(Debug, Deserialize)]
struct ApiModelItem {
    id: String,
    owned_by: String,
}

pub async fn list_models(api_key: &str) -> Result<Vec<ChatModelInfo>, ProviderError> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(ProviderError::message(
            "DeepSeek API Key is not configured. Please enter it in Settings.",
        ));
    }

    let client = reqwest::Client::new();
    let response = client
        .get(MODELS_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| ProviderError::message(format!("network error: {error}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        return Err(ProviderError::message(format!(
            "DeepSeek API {status}: {text}"
        )));
    }

    let parsed: ApiModelsResponse = response
        .json()
        .await
        .map_err(|error| ProviderError::message(format!("invalid models payload: {error}")))?;

    Ok(parsed
        .data
        .into_iter()
        .map(|item| ChatModelInfo {
            id: item.id,
            owned_by: item.owned_by,
            provider: "deepseek".to_string(),
            display_name: None,
            thinking_variants: None,
        })
        .collect())
}

/// Normalize an OpenAI-compatible base URL to a chat completions endpoint.
///
/// Bare hosts such as `https://www.micuapi.ai` (NewAPI) must become
/// `.../v1/chat/completions`, not `.../chat/completions`.
pub(crate) fn normalize_chat_completions_url(base_url: &str) -> String {
    let mut base = base_url.trim().trim_end_matches('/').to_string();
    if let Some(stripped) = base.strip_suffix("/chat/completions") {
        base = stripped.trim_end_matches('/').to_string();
    }
    if !has_versioned_api_path(&base) {
        base = format!("{base}/v1");
    }
    format!("{base}/chat/completions")
}

fn has_versioned_api_path(base: &str) -> bool {
    let path = url_path(base);
    if path.is_empty() || path == "/" {
        return false;
    }
    path == "/v1" || path.ends_with("/v1") || path.contains("/v1/") || path.contains("/v1beta")
}

fn url_path(base: &str) -> &str {
    let rest = base
        .strip_prefix("https://")
        .or_else(|| base.strip_prefix("http://"))
        .unwrap_or(base);
    match rest.find('/') {
        Some(index) => &rest[index..],
        None => "",
    }
}
