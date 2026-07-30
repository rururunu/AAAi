use tauri::{AppHandle, Emitter, Manager};

use crate::core::runtime::RequestContext;
use crate::services::window::{
    destroy_overlay, hide_overlay, is_overlay_label, minimize_overlay, set_overlay_chat_mode,
    set_overlay_popup_open, show_settings_window,
};

#[tauri::command]
pub async fn open_session_in_overlay(app: AppHandle, session_id: String) -> Result<(), String> {
    tracing::debug!(source = "open_session_in_overlay", "overlay opening start");
    let captured = crate::core::context::store::capture_now();
    let context = app
        .try_state::<crate::app_state::AppState>()
        .map(|state| state.core.chat().environment_context())
        .unwrap_or(captured);
    let all_windows = app.webview_windows();
    let overlay_windows: Vec<_> = all_windows
        .iter()
        .filter(|(label, _)| crate::services::window::is_overlay_label(label))
        .map(|(_, window)| window)
        .collect();

    for window in &overlay_windows {
        let _ = window.emit("context-captured", &context);
        tracing::debug!(
            label = %window.label(),
            source = "open_session_in_overlay",
            "overlay interactive ready"
        );
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

#[tauri::command]
pub fn minimize_overlay_window(app: AppHandle, label: Option<String>) {
    let label = label.unwrap_or_else(|| "overlay".to_string());
    minimize_overlay(&app, &label);
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

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn preview_image_store() -> &'static Mutex<String> {
    static STORE: OnceLock<Mutex<String>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(String::new()))
}

/// Persist preview payload as a local file and return `path:<abs>` for the frontend.
fn cache_preview_payload(app: &AppHandle, path_or_base64: &str) -> Result<String, String> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("cache dir unavailable: {e}"))?
        .join("image-preview");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("failed to create preview cache: {e}"))?;

    let (ext, bytes) = if let Some(rest) = path_or_base64.strip_prefix("data:") {
        let (meta, data) = rest
            .split_once(',')
            .ok_or_else(|| "invalid data URL".to_string())?;
        let mime = meta.split(';').next().unwrap_or("image/png");
        let ext = match mime {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "png",
        };
        use base64::{engine::general_purpose, Engine as _};
        let bytes = general_purpose::STANDARD
            .decode(data.trim())
            .map_err(|e| format!("invalid base64 image: {e}"))?;
        (ext, bytes)
    } else if Path::new(path_or_base64).exists() {
        let ext = if path_or_base64.ends_with(".jpg") || path_or_base64.ends_with(".jpeg") {
            "jpg"
        } else if path_or_base64.ends_with(".gif") {
            "gif"
        } else if path_or_base64.ends_with(".webp") {
            "webp"
        } else {
            "png"
        };
        let bytes =
            std::fs::read(path_or_base64).map_err(|e| format!("Failed to read image file: {e}"))?;
        (ext, bytes)
    } else {
        return Err("unsupported image payload".into());
    };

    let out: PathBuf = cache_dir.join(format!("current.{ext}"));
    std::fs::write(&out, bytes).map_err(|e| format!("failed to write preview cache: {e}"))?;
    Ok(format!("path:{}", out.to_string_lossy()))
}

#[tauri::command]
pub fn get_preview_image() -> String {
    if let Ok(guard) = preview_image_store().lock() {
        guard.clone()
    } else {
        String::new()
    }
}

#[tauri::command]
pub async fn open_image_preview(app: AppHandle, path_or_base64: String) -> Result<(), String> {
    let stored = cache_preview_payload(&app, &path_or_base64)?;
    if let Ok(mut guard) = preview_image_store().lock() {
        *guard = stored;
    }

    crate::services::window::set_overlay_popup_open("overlay", true);

    // Reuse an existing preview window when possible.
    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-preview-") {
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.emit("preview-image-updated", ());
            return Ok(());
        }
    }

    let label = format!("overlay-preview-{}", uuid::Uuid::new_v4());

    let mut x_pos = None;
    let mut y_pos = None;

    if let Some(overlay_win) = app.get_webview_window("overlay") {
        if let (Ok(outer_pos), Ok(outer_size)) =
            (overlay_win.outer_position(), overlay_win.outer_size())
        {
            let scale_factor = overlay_win.scale_factor().unwrap_or(1.0);
            let logical_pos = outer_pos.to_logical::<f64>(scale_factor);
            let logical_size = outer_size.to_logical::<f64>(scale_factor);

            let left_x = logical_pos.x - 740.0;
            if left_x >= 10.0 {
                x_pos = Some(left_x);
            } else {
                x_pos = Some(logical_pos.x + logical_size.width + 20.0);
            }
            y_pos = Some(logical_pos.y);
        }
    }

    let url_str = "/#/image-preview";

    let mut window_builder =
        tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::App(url_str.into()))
            .title("Preview")
            .inner_size(720.0, 520.0)
            .resizable(true)
            .decorations(false);

    if let (Some(x), Some(y)) = (x_pos, y_pos) {
        window_builder = window_builder.position(x, y);
    } else {
        window_builder = window_builder.center();
    }

    let window = window_builder
        .build()
        .map_err(|e| format!("Failed to build window: {e}"))?;
    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}
