use tauri::AppHandle;

use crate::models::settings::{AppSettings, AppSettingsPatch};
use crate::services::settings_store::{get_settings, set_settings};

#[tauri::command]
pub fn get_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    get_settings(&app)
}

#[tauri::command]
pub fn set_app_settings(app: AppHandle, patch: AppSettingsPatch) -> Result<AppSettings, String> {
    let current = get_settings(&app)?;
    let next = current.merge(patch);
    set_settings(&app, next)
}
