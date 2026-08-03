use std::sync::Arc;

use tauri::AppHandle;

use super::antigravity::AntigravityProvider;
use super::deepseek::DeepSeekProvider;
use super::provider::AIProvider;
use crate::models::settings::{AppSettings, CustomProviderConfig, ReasoningEffort};
use crate::services::gemini_oauth;
use crate::services::settings_store;

/// Resolve the provider selected for the primary chat model.
pub fn resolve_provider(app: AppHandle) -> Arc<dyn AIProvider> {
    let settings = settings_store::get_settings(&app).unwrap_or_default();
    let model = settings.chat_model.trim().to_string();
    let provider = settings.chat_model_provider.trim().to_string();
    resolve_provider_for_selection(app, model, provider)
}

/// Resolve a provider by model only for callers that do not own a provider selection.
pub fn resolve_provider_for_model(app: AppHandle, model: String) -> Arc<dyn AIProvider> {
    resolve_provider_for_selection(app, model, String::new())
}

fn provider_has_model(provider: &CustomProviderConfig, model: &str) -> bool {
    provider
        .models
        .split([',', '\n'])
        .map(str::trim)
        .any(|id| !id.is_empty() && id == model)
}

fn custom_provider_for_selection<'a>(
    settings: &'a AppSettings,
    model: &str,
    provider_hint: &str,
) -> Option<&'a CustomProviderConfig> {
    if provider_hint.is_empty() {
        return settings
            .custom_providers
            .iter()
            .find(|provider| provider_has_model(provider, model));
    }

    settings
        .custom_providers
        .iter()
        .find(|provider| provider.id == provider_hint && provider_has_model(provider, model))
}

/// Resolve the provider by an explicit model + provider-hint selection.
/// Used for per-conversation model overrides; empty hint resolves by model match.
pub(crate) fn resolve_provider_for_selection(
    app: AppHandle,
    model: String,
    provider_hint: String,
) -> Arc<dyn AIProvider> {
    let settings = settings_store::get_settings(&app).unwrap_or_default();
    let model = model.trim().to_string();
    let provider_hint = provider_hint.trim().to_string();

    if (provider_hint.is_empty() || provider_hint == "gemini")
        && gemini_oauth::is_gemini_model(&model)
        && settings.gemini_oauth.is_logged_in()
    {
        return Arc::new(AntigravityProvider::for_model(app, model));
    }
    let resolve_api_key = {
        let app = app.clone();
        let selected_model = model.clone();
        let selected_provider = provider_hint.clone();
        Arc::new(move || {
            let settings = settings_store::get_settings(&app).unwrap_or_default();
            custom_provider_for_selection(&settings, &selected_model, &selected_provider)
                .map(|custom| custom.api_key.clone())
                .unwrap_or(settings.deepseek_api_key)
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
        let selected_provider = provider_hint;
        Arc::new(move || -> Option<String> {
            let settings = settings_store::get_settings(&app).unwrap_or_default();
            custom_provider_for_selection(&settings, &selected_model, &selected_provider).and_then(
                |custom| {
                    let base_url = custom.base_url.trim();
                    (!base_url.is_empty()).then(|| base_url.to_string())
                },
            )
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

#[cfg(test)]
mod tests {
    use super::custom_provider_for_selection;
    use crate::models::settings::{AppSettings, CustomProviderConfig};

    fn provider(id: &str, api_key: &str) -> CustomProviderConfig {
        CustomProviderConfig {
            id: id.into(),
            name: id.into(),
            base_url: format!("https://{id}.example/v1"),
            api_key: api_key.into(),
            models: "shared-model".into(),
        }
    }

    #[test]
    fn provider_hint_disambiguates_duplicate_chat_model_ids() {
        let settings = AppSettings {
            custom_providers: vec![provider("first", "key-1"), provider("second", "key-2")],
            ..Default::default()
        };

        let selected = custom_provider_for_selection(&settings, "shared-model", "second")
            .expect("second provider should match");
        assert_eq!(selected.api_key, "key-2");
        assert_eq!(selected.base_url, "https://second.example/v1");
    }

    #[test]
    fn empty_provider_hint_keeps_legacy_first_model_match() {
        let settings = AppSettings {
            custom_providers: vec![provider("first", "key-1"), provider("second", "key-2")],
            ..Default::default()
        };

        let selected = custom_provider_for_selection(&settings, "shared-model", "")
            .expect("legacy model lookup should match");
        assert_eq!(selected.id, "first");
    }
}
