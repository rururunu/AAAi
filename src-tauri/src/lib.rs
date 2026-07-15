mod adapters;
mod app_state;
mod commands;
mod core;
mod models;
mod runtime;
mod services;

use std::time::{SystemTime, UNIX_EPOCH};

use rdev::{listen, Event, EventType, Key};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, RunEvent,
};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use app_state::AppState;
use commands::{app, ask, chat, harness, mcp, permission, settings, skills, window, workspace};
use services::settings_store::{load_settings, SettingsState};
use services::window::{
    cleanup_overlay_state, configure_overlay_window, is_overlay_label, mark_blur_guard,
    should_keep_overlay_visible, show_settings_window, toggle_overlay,
};

const DOUBLE_TAP_MS: u64 = 400;

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn is_alt_key(key: Key) -> bool {
    matches!(key, Key::Alt | Key::AltGr)
}

/// 即时获取鼠标物理坐标
fn cursor_pos() -> Option<(i32, i32)> {
    let mut pt = POINT::default();
    unsafe { GetCursorPos(&mut pt).ok().map(|_| (pt.x, pt.y)) }
}

#[derive(Default)]
struct DoubleAltDetector {
    alt_down: bool,
    chorded: bool,
    last_tap_ms: Option<u64>,
    /// Second Alt was pressed within the double-tap window; fire on its keyup
    /// so Alt is no longer held when we simulate Ctrl+Insert / Ctrl+C.
    pending_trigger: bool,
}

impl DoubleAltDetector {
    fn key_press(&mut self, key: Key, now: u64) {
        if is_alt_key(key) {
            if self.alt_down {
                // Key-repeat while holding Alt — ignore.
                return;
            }
            self.alt_down = true;
            self.chorded = false;
            let double_tap = self
                .last_tap_ms
                .is_some_and(|last| now.saturating_sub(last) <= DOUBLE_TAP_MS);
            if double_tap {
                self.last_tap_ms = None;
                self.pending_trigger = true;
            }
            return;
        }

        if self.alt_down {
            self.chorded = true;
            self.last_tap_ms = None;
            self.pending_trigger = false;
        }
    }

    /// Returns true when the second Alt of a double-tap is released.
    fn key_release(&mut self, key: Key, now: u64) -> bool {
        if !is_alt_key(key) || !self.alt_down {
            return false;
        }

        self.alt_down = false;
        if self.chorded {
            self.chorded = false;
            self.last_tap_ms = None;
            self.pending_trigger = false;
            return false;
        }

        if self.pending_trigger {
            self.pending_trigger = false;
            self.last_tap_ms = None;
            return true;
        }

        self.last_tap_ms = Some(now);
        false
    }
}

fn trigger_overlay(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        crate::core::context::provider::force_release_modifiers_for_capture();
        toggle_overlay(&handle, cursor_pos());
    });
}

fn start_hotkey_listener(app: AppHandle) {
    std::thread::spawn(move || {
        let mut double_alt = DoubleAltDetector::default();
        let mut secondary = crate::services::hotkey::SecondaryHotkeyDetector::default();
        let callback = move |event: Event| {
            let chord = crate::services::hotkey::current_secondary_hotkey();
            let triggered = match event.event_type {
                EventType::KeyPress(key) => {
                    double_alt.key_press(key, now_millis());
                    secondary.key_press(key, &chord);
                    false
                }
                EventType::KeyRelease(key) => {
                    let alt = double_alt.key_release(key, now_millis());
                    let chord_hit = secondary.key_release(key, &chord);
                    alt || chord_hit
                }
                _ => false,
            };

            if triggered {
                trigger_overlay(&app);
            }
        };

        if let Err(error) = listen(callback) {
            eprintln!("failed to listen for global shortcuts: {error:?}");
        }
    });
}

#[cfg(test)]
mod double_alt_tests {
    use super::*;

    #[test]
    fn key_repeat_during_long_press_does_not_trigger() {
        let mut detector = DoubleAltDetector::default();
        detector.key_press(Key::Alt, 1_000);
        detector.key_press(Key::Alt, 1_010);
        assert!(!detector.key_release(Key::Alt, 1_020));
    }

    #[test]
    fn two_complete_taps_trigger_on_second_release() {
        let mut detector = DoubleAltDetector::default();
        detector.key_press(Key::Alt, 1_000);
        assert!(!detector.key_release(Key::Alt, 1_050));
        detector.key_press(Key::Alt, 1_250);
        assert!(detector.key_release(Key::Alt, 1_280));
        // Next press must not immediately fire without a new arming tap.
        detector.key_press(Key::Alt, 1_300);
        assert!(!detector.key_release(Key::Alt, 1_320));
    }

    #[test]
    fn alt_chord_does_not_count_as_a_tap() {
        let mut detector = DoubleAltDetector::default();
        detector.key_press(Key::Alt, 1_000);
        detector.key_press(Key::KeyC, 1_010);
        assert!(!detector.key_release(Key::Alt, 1_020));
        detector.key_press(Key::Alt, 1_100);
        assert!(!detector.key_release(Key::Alt, 1_120));
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &quit])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .expect("missing application icon");

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("AltAltAi")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => show_settings_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let settings = load_settings(app.handle());
            crate::core::tools::memory::shared_memory_store().configure(&settings);
            crate::runtime::search::shared_search_runtime().configure(&settings);
            crate::services::hotkey::configure_secondary_hotkey(&settings.secondary_hotkey);
            crate::core::tools::tool_approval::shared_tool_approval_store()
                .configure(settings.tool_approval_mode);
            crate::core::lsp::shared_lsp_manager().configure(&settings);
            crate::core::mcp::shared_mcp_manager().configure(&settings);
            app.manage(SettingsState::new(settings.clone()));
            app.manage(AppState::new(app.handle().clone()));
            // Never block app startup on MCP cold-start / missing npx.
            if let Some(state) = app.try_state::<AppState>() {
                let registry = state.core.tools().registry();
                tauri::async_runtime::spawn_blocking(move || {
                    let _ = crate::core::mcp::shared_mcp_manager().register_enabled(registry.as_ref());
                });
            }
            setup_tray(app)?;
            if let Some(window) = app.get_webview_window("overlay") {
                configure_overlay_window(&window);
            }
            start_hotkey_listener(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label().to_string();

            // 窗口销毁时清理状态
            if let tauri::WindowEvent::Destroyed = event {
                if is_overlay_label(&label) {
                    cleanup_overlay_state(&label);
                }
                return;
            }

            let tauri::WindowEvent::Focused(focused) = event else {
                return;
            };

            // 某个 overlay 获得焦点时，设置 blur guard，防止因窗口间焦点切换
            // 导致另一个 overlay 被错误地隐藏（如点击另一个 overlay 的关闭按钮）
            if *focused && is_overlay_label(&label) {
                mark_blur_guard();
                return;
            }

            if *focused || !window.is_visible().unwrap_or(false) {
                return;
            }

            if is_overlay_label(&label) {
                if should_keep_overlay_visible(&label) {
                    return;
                }
                let _ = window.hide();
                let _ = window.emit_to(&label, "overlay-hidden", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            window::open_settings,
            window::open_session_in_overlay,
            window::hide_overlay_window,
            window::close_overlay_window,
            window::exit_app,
            window::set_overlay_chat_mode_command,
            window::set_overlay_popup_open_command,
            window::take_overlay_context,
            settings::get_app_settings,
            settings::set_app_settings,
            skills::list_skills,
            skills::install_skill,
            skills::uninstall_skill,
            skills::get_skills_dir,
            skills::open_skills_dir,
            mcp::get_mcp_runtime_support,
            app::get_app_info,
            chat::chat,
            chat::chat_cancel,
            chat::chat_history,
            chat::list_chat_sessions,
            chat::list_chat_models,
            chat::delete_chat_session,
            chat::clear_all_chat_sessions,
            workspace::list_workspaces,
            workspace::get_current_workspace,
            workspace::list_workspace_files,
            workspace::create_workspace,
            workspace::switch_workspace,
            workspace::clear_current_workspace,
            workspace::delete_workspace,
            ask::respond_ask_user,
            permission::respond_path_permission,
            harness::respond_tool_approval,
            harness::set_plan_mode,
            harness::get_plan_mode,
            harness::list_checkpoints,
            harness::rewind_session,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
