use tauri::AppHandle;

use crate::models::app_info::AppInfo;

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> Result<AppInfo, String> {
    Ok(AppInfo {
        name: app.package_info().name.clone(),
        version: app.package_info().version.to_string(),
        identifier: app.config().identifier.clone(),
    })
}
