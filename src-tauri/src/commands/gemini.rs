use tauri::AppHandle;

use crate::services::gemini_oauth::{self, GeminiAuthStatus};

#[tauri::command]
pub fn gemini_auth_status(app: AppHandle) -> Result<GeminiAuthStatus, String> {
    gemini_oauth::auth_status(&app)
}

#[tauri::command]
pub fn gemini_oauth_login(app: AppHandle) -> Result<GeminiAuthStatus, String> {
    gemini_oauth::login(&app)
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
