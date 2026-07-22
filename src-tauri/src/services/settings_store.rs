use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};

use crate::models::settings::AppSettings;

const SETTINGS_FILE: &str = "settings.json";

pub struct SettingsState {
    pub settings: Mutex<AppSettings>,
}

impl SettingsState {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            settings: Mutex::new(settings),
        }
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join(SETTINGS_FILE))
        .map_err(|error| error.to_string())
}

pub fn load_settings(app: &AppHandle) -> AppSettings {
    let path = match settings_path(app) {
        Ok(path) => path,
        Err(_) => return AppSettings::default(),
    };

    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => return AppSettings::default(),
    };

    let mut settings: AppSettings = serde_json::from_str(&raw).unwrap_or_default();
    settings.secondary_hotkey =
        crate::services::hotkey::normalize_hotkey(&settings.secondary_hotkey);
    // Migrate to embedded Antigravity OAuth client; old Gemini Desktop tokens won't refresh.
    let defaults = crate::models::settings::GeminiOAuthSettings::default();
    let client_changed = settings.gemini_oauth.client_id.trim() != defaults.client_id;
    if settings.gemini_oauth.client_id.trim().is_empty() || client_changed {
        if client_changed && settings.gemini_oauth.is_logged_in() {
            settings.gemini_oauth.access_token.clear();
            settings.gemini_oauth.refresh_token.clear();
            settings.gemini_oauth.expires_at = 0;
            settings.gemini_oauth.email.clear();
            settings.gemini_oauth.project_id.clear();
        }
        settings.gemini_oauth.client_id = defaults.client_id;
        settings.gemini_oauth.client_secret = defaults.client_secret;
    } else if settings.gemini_oauth.client_secret.trim().is_empty() {
        settings.gemini_oauth.client_secret = defaults.client_secret;
    }
    if settings.custom_accent_color.trim().is_empty()
        || settings.custom_accent_color.eq_ignore_ascii_case("#ffffff")
    {
        // Soft off-white; migrate empty / former pure-white default.
        settings.custom_accent_color = "#e8ecf2".to_string();
    }
    settings
}

pub fn persist_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let raw = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(path, raw).map_err(|error| error.to_string())
}

pub fn get_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let state = app
        .try_state::<SettingsState>()
        .ok_or_else(|| "settings state is unavailable".to_string())?;

    state
        .settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|error| error.to_string())
}

pub fn set_settings(app: &AppHandle, next: AppSettings) -> Result<AppSettings, String> {
    persist_settings(app, &next)?;

    let state = app
        .try_state::<SettingsState>()
        .ok_or_else(|| "settings state is unavailable".to_string())?;

    {
        let mut settings = state.settings.lock().map_err(|error| error.to_string())?;
        *settings = next.clone();
    }

    crate::core::tools::memory::shared_memory_store().configure(&next);
    crate::runtime::search::shared_search_runtime().configure(&next);
    crate::services::hotkey::configure_secondary_hotkey(&next.secondary_hotkey);
    crate::core::tools::tool_approval::shared_tool_approval_store()
        .configure(next.tool_approval_mode);
    crate::core::lsp::shared_lsp_manager().configure(&next);
    crate::core::mcp::shared_mcp_manager().configure(&next);
    // Connecting MCP (npx/uvx cold start) can block for a long time — never hold
    // set_app_settings / the settings UI on that work.
    if let Some(app_state) = app.try_state::<crate::app_state::AppState>() {
        let registry: Arc<_> = app_state.core.tools().registry();
        tauri::async_runtime::spawn_blocking(move || {
            let _ = crate::core::mcp::shared_mcp_manager().register_enabled(registry.as_ref());
        });
    }

    broadcast_settings(app, &next);
    Ok(next)
}

pub fn broadcast_settings(app: &AppHandle, settings: &AppSettings) {
    let _ = app.emit("settings-changed", settings.clone());
}
