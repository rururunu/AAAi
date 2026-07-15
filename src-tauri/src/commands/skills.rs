use std::path::PathBuf;

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::core::tools::skills::{
    ensure_skills_directory, install_skill_at, list_skill_infos, uninstall_user_skill, SkillInfo,
};

#[tauri::command]
pub fn list_skills() -> Result<Vec<SkillInfo>, String> {
    list_skill_infos().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_skill(path: String, name: Option<String>) -> Result<SkillInfo, String> {
    let source = PathBuf::from(path.trim());
    install_skill_at(&source, name.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn uninstall_skill(name: String) -> Result<(), String> {
    uninstall_user_skill(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_skills_dir() -> Result<String, String> {
    let dir = ensure_skills_directory().map_err(|e| e.to_string())?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_skills_dir(app: AppHandle) -> Result<(), String> {
    let dir = ensure_skills_directory().map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}
