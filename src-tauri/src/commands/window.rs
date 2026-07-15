use tauri::{AppHandle, Emitter, Manager};

use crate::core::runtime::RequestContext;
use crate::services::window::{
    destroy_overlay, hide_overlay, is_overlay_label, set_overlay_chat_mode, set_overlay_popup_open,
    show_settings_window,
};

#[tauri::command]
pub async fn open_session_in_overlay(app: AppHandle, session_id: String) -> Result<(), String> {
    let all_windows = app.webview_windows();
    let overlay_windows: Vec<_> = all_windows
        .iter()
        .filter(|(label, _)| crate::services::window::is_overlay_label(label))
        .map(|(_, window)| window)
        .collect();

    for window in &overlay_windows {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("overlay-shown", ());
    }

    // 延迟 150 毫秒，等待 Webview 激活并且前端 listener 完成挂载/苏醒
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let _ = app.emit("open-session", session_id);
    Ok(())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) {
    show_settings_window(&app);
}

#[tauri::command]
pub fn hide_overlay_window(app: AppHandle, label: Option<String>) {
    let label = label.unwrap_or_else(|| "overlay".to_string());
    hide_overlay(&app, &label);
}

/// 前端调用：关闭并销毁窗口（适用于聊天窗口的关闭按钮）
#[tauri::command]
pub fn close_overlay_window(app: AppHandle, label: String) {
    if label == "overlay" {
        // 基础窗口只隐藏不销毁
        hide_overlay(&app, &label);
    } else if is_overlay_label(&label) {
        destroy_overlay(&app, &label);
    }
}

#[tauri::command]
pub fn exit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn set_overlay_chat_mode_command(label: String, enabled: bool) {
    set_overlay_chat_mode(&label, enabled);
}

#[tauri::command]
pub fn set_overlay_popup_open_command(label: String, open: bool) {
    set_overlay_popup_open(&label, open);
}

#[tauri::command]
pub fn take_overlay_context(label: String) -> Option<RequestContext> {
    crate::services::window::take_overlay_context(&label)
}
