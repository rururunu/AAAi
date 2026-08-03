use tauri::AppHandle;

use crate::services::gemini_oauth::{self, GeminiAuthStatus};

#[tauri::command]
pub fn gemini_auth_status(app: AppHandle) -> Result<GeminiAuthStatus, String> {
    gemini_oauth::auth_status(&app)
}

/// Runs on a blocking pool thread so the UI stays responsive while waiting for
/// the browser OAuth callback (or cancel/timeout).
#[tauri::command]
pub async fn gemini_oauth_login(app: AppHandle) -> Result<GeminiAuthStatus, String> {
    tauri::async_runtime::spawn_blocking(move || gemini_oauth::login(&app))
        .await
        .map_err(|error| format!("Gemini login task failed: {error}"))?
}

#[tauri::command]
pub fn gemini_oauth_cancel_login() -> Result<(), String> {
    gemini_oauth::cancel_login();
    Ok(())
}

#[tauri::command]
pub fn gemini_oauth_logout(app: AppHandle) -> Result<GeminiAuthStatus, String> {
    gemini_oauth::logout(&app)
}

#[tauri::command]
pub fn gemini_import_client_secrets(
    app: AppHandle,
    path: String,
) -> Result<GeminiAuthStatus, String> {
    gemini_oauth::import_client_secrets(&app, path.trim())
}
