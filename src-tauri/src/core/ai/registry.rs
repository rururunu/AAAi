use std::sync::Arc;

use tauri::AppHandle;

use super::deepseek::DeepSeekProvider;
use super::provider::AIProvider;
use crate::services::settings_store;

/// 根据设置解析当前 AI Provider，后续可扩展为多 Provider 切换。
pub fn resolve_provider(app: AppHandle) -> Arc<dyn AIProvider> {
    let resolve_api_key = {
        let app = app.clone();
        Arc::new(move || {
            settings_store::get_settings(&app)
                .map(|settings| settings.deepseek_api_key)
                .unwrap_or_default()
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

    Arc::new(DeepSeekProvider::new(
        resolve_api_key,
        resolve_model,
        resolve_effort,
        resolve_pass_tool_reasoning,
    ))
}

fn default_chat_model() -> String {
    "deepseek-chat".to_string()
}
