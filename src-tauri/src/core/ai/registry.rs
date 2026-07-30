use std::sync::Arc;

use tauri::AppHandle;

use super::antigravity::AntigravityProvider;
use super::deepseek::DeepSeekProvider;
use super::provider::AIProvider;
use crate::models::settings::ReasoningEffort;
use crate::services::gemini_oauth;
use crate::services::settings_store;

/// Resolve the AI provider to use for a request.
/// Priority: Antigravity (Gemini OAuth, when logged in) → custom provider → DeepSeek.
///
/// Gemini models always prefer Antigravity when OAuth is available, even if the same
/// model id also appears under a custom OpenAI-compatible provider. Routing Gemini
/// through that OpenAI path breaks native vision and triggers a fragile multimodal
/// fallback (`Failed to read multimodal response: error decoding response body`).
pub fn resolve_provider(app: AppHandle) -> Arc<dyn AIProvider> {
    let settings = settings_store::get_settings(&app).unwrap_or_default();
    let model = settings.chat_model.trim().to_string();
    resolve_provider_for_model(app, model)
}

/// Resolve a provider bound to a specific model without changing global settings.
pub fn resolve_provider_for_model(app: AppHandle, model: String) -> Arc<dyn AIProvider> {
    let settings = settings_store::get_settings(&app).unwrap_or_default();
    let model = model.trim().to_string();

    if gemini_oauth::is_gemini_model(&model) && settings.gemini_oauth.is_logged_in() {
        return Arc::new(AntigravityProvider::for_model(app, model));
    }

    let resolve_api_key = {
        let app = app.clone();
        let selected_model = model.clone();
        Arc::new(move || {
            let settings = settings_store::get_settings(&app).unwrap_or_default();
            for custom in &settings.custom_providers {
                let custom_ids: Vec<&str> = custom
                    .models
                    .split([',', '\n'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                if custom_ids.contains(&selected_model.as_str()) {
                    return custom.api_key.clone();
                }
            }
            settings.deepseek_api_key
        })
    };

    let resolve_model = {
        let model = model.clone();
        Arc::new(move || model.clone())
    };

    let resolve_effort = {
        let app = app.clone();
        Arc::new(move || {
            settings_store::get_settings(&app)
                .map(|settings| settings.reasoning_effort)
                .unwrap_or(ReasoningEffort::Disabled)
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

    let resolve_base_url = {
        let app = app.clone();
        let selected_model = model.clone();
        Arc::new(move || -> Option<String> {
            let settings = settings_store::get_settings(&app).unwrap_or_default();
            for custom in &settings.custom_providers {
                let custom_ids: Vec<&str> = custom
                    .models
                    .split([',', '\n'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                if custom_ids.contains(&selected_model.as_str()) && !custom.base_url.trim().is_empty() {
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
