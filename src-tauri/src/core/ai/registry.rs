use std::sync::Arc;

use tauri::AppHandle;

use super::deepseek::DeepSeekProvider;
use super::provider::AIProvider;
use crate::services::settings_store;

/// Resolve the AI provider to use for a request.
/// If the currently selected model belongs to the custom provider,
/// a DeepSeekProvider pointed at the custom base URL is returned.
/// Otherwise the standard DeepSeek provider is returned.
pub fn resolve_provider(app: AppHandle) -> Arc<dyn AIProvider> {
    let resolve_api_key = {
        let app = app.clone();
        Arc::new(move || {
            let settings = settings_store::get_settings(&app).unwrap_or_default();
            let model = settings.chat_model.trim().to_string();
            for custom in &settings.custom_providers {
                let custom_ids: Vec<&str> = custom
                    .models
                    .split([',', '\n'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                if custom_ids.contains(&model.as_str()) {
                    return custom.api_key.clone();
                }
            }
            settings.deepseek_api_key
        })
    };

    let resolve_model = {
        let app = app.clone();
        Arc::new(move || {
            settings_store::get_settings(&app)
                .map(|settings| settings.chat_model)
                .unwrap_or_else(|_| default_chat_model())
        })
    };

    let resolve_effort = {
        let app = app.clone();
        Arc::new(move || {
            settings_store::get_settings(&app)
                .map(|settings| settings.reasoning_effort)
                .unwrap_or_default()
        })
    };

    let resolve_pass_tool_reasoning = {
        let app = app.clone();
        Arc::new(move || {
            settings_store::get_settings(&app)
                .map(|settings| settings.pass_tool_reasoning)
                .unwrap_or(true)
        })
    };

    // Resolver for a custom base URL (None = use default DeepSeek endpoint).
    let resolve_base_url = {
        let app = app.clone();
        Arc::new(move || -> Option<String> {
            let settings = settings_store::get_settings(&app).unwrap_or_default();
            let model = settings.chat_model.trim().to_string();
            for custom in &settings.custom_providers {
                let custom_ids: Vec<&str> = custom
                    .models
                    .split([',', '\n'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                if custom_ids.contains(&model.as_str()) && !custom.base_url.trim().is_empty() {
                    return Some(custom.base_url.trim().to_string());
                }
            }
            None
        })
    };

    Arc::new(DeepSeekProvider::new(
        app.clone(),
        resolve_api_key,
        resolve_model,
        resolve_effort,
        resolve_pass_tool_reasoning,
        Some(resolve_base_url),
    ))
}

fn default_chat_model() -> String {
    "deepseek-chat".to_string()
}
