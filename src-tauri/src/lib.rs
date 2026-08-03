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
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent,
};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use app_state::AppState;
use commands::{
    app, ask, chat, diff, gemini, harness, mcp, permission, settings, skills, token_usage, window,
    workspace,
};
use services::overlay_native::clear_minimize_pending;
use services::settings_store::{
    apply_runtime_settings, load_settings, register_enabled_mcp_tools, SettingsState,
};
use services::window::{
    cleanup_overlay_state, configure_overlay_window, handle_overlay_focused, is_overlay_label,
    mark_blur_guard, should_keep_overlay_visible, show_settings_window, show_workbench_window,
    toggle_overlay,
};

const DOUBLE_TAP_MS: u64 = 400;

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn cursor_pos() -> Option<(i32, i32)> {
    let mut pt = POINT::default();
    unsafe { GetCursorPos(&mut pt).ok().map(|_| (pt.x, pt.y)) }
}

#[derive(Default)]
struct DoubleModifierDetector {
    modifier: Option<crate::services::hotkey::PrimaryHotkey>,
    modifier_down: bool,
    chorded: bool,
    last_tap_ms: Option<u64>,
    /// Second Alt was pressed within the double-tap window; fire on its keyup
    /// so Alt is no longer held when we simulate Ctrl+Insert / Ctrl+C.
    pending_trigger: bool,
}

impl DoubleModifierDetector {
    fn sync_modifier(&mut self, modifier: crate::services::hotkey::PrimaryHotkey) {
        if self.modifier != Some(modifier) {
            *self = Self {
                modifier: Some(modifier),
                ..Self::default()
            };
        }
    }

    fn key_press(&mut self, key: Key, now: u64, modifier: crate::services::hotkey::PrimaryHotkey) {
        self.sync_modifier(modifier);
        if modifier.matches(key) {
            if self.modifier_down {
                // Key-repeat while holding Alt — ignore.
                return;
            }
            self.modifier_down = true;
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

        if self.modifier_down {
            self.chorded = true;
            self.last_tap_ms = None;
            self.pending_trigger = false;
        }
    }

    /// Returns true when the second Alt of a double-tap is released.
    fn key_release(
        &mut self,
        key: Key,
        now: u64,
        modifier: crate::services::hotkey::PrimaryHotkey,
    ) -> bool {
        self.sync_modifier(modifier);
        if !modifier.matches(key) || !self.modifier_down {
            return false;
        }

        self.modifier_down = false;
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
        let mut primary_detector = DoubleModifierDetector::default();
        let mut secondary = crate::services::hotkey::SecondaryHotkeyDetector::default();
        let callback = move |event: Event| {
            let primary = crate::services::hotkey::current_primary_hotkey();
            let chord = crate::services::hotkey::current_secondary_hotkey();
            let triggered = match event.event_type {
                EventType::KeyPress(key) => {
                    primary_detector.key_press(key, now_millis(), primary);
                    secondary.key_press(key, &chord);
                    false
                }
                EventType::KeyRelease(key) => {
                    let primary_hit = primary_detector.key_release(key, now_millis(), primary);
                    let chord_hit = secondary.key_release(key, &chord);
                    primary_hit || chord_hit
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

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let workbench = MenuItem::with_id(app, "workbench", "Open Workbench", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&workbench, &settings, &quit])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .expect("missing application icon");

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(app.package_info().name.clone())
        .on_menu_event(|app, event| match event.id.as_ref() {
            "workbench" => show_workbench_window(app),
            "settings" => show_settings_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_workbench_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                show_workbench_window(&handle);
            });
        }))
        .setup(|app| {
            let config_dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            crate::core::chat::telemetry::init_logging(&config_dir);
            let settings = load_settings(app.handle());
            apply_runtime_settings(&settings);
            crate::services::pin_badge::start(app.handle().clone());
            app.manage(SettingsState::new(settings.clone()));
            app.manage(AppState::new(app.handle().clone()));
            crate::core::context::providers::local_api::start_server(app.handle().clone());
            register_enabled_mcp_tools(app.handle());
            setup_tray(app)?;
            if let Some(window) = app.get_webview_window("overlay") {
                configure_overlay_window(&window);
            }
            start_hotkey_listener(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label().to_string();

            if label == "workbench" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                    return;
                }
            }

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
                handle_overlay_focused(window.app_handle(), &label);
                return;
            }

            if !*focused && is_overlay_label(&label) {
                clear_minimize_pending();
            }

            if *focused
                || !window.is_visible().unwrap_or(false)
                || window.is_minimized().unwrap_or(false)
            {
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
            window::open_session_in_workbench,
            window::show_interaction_notification,
            window::set_window_session_view,
            window::hide_overlay_window,
            window::minimize_overlay_window,
            window::close_overlay_window,
            window::exit_app,
            window::set_overlay_chat_mode_command,
            window::set_overlay_popup_open_command,
            window::take_overlay_context,
            window::open_image_preview,
            window::get_preview_image,
            settings::get_app_settings,
            settings::set_app_settings,
            gemini::gemini_auth_status,
            gemini::gemini_oauth_login,
            gemini::gemini_oauth_logout,
            gemini::gemini_import_client_secrets,
            skills::list_skills,
            skills::install_skill,
            skills::uninstall_skill,
            skills::get_skills_dir,
            skills::open_skills_dir,
            mcp::get_mcp_runtime_support,
            app::get_app_info,
            diff::build_code_diff,
            chat::chat,
            chat::chat_cancel,
            chat::agent_debug_snapshot,
            chat::chat_history,
            chat::list_chat_sessions,
            chat::list_chat_models,
            chat::get_context_usage,
            chat::get_environment_context,
            chat::delete_chat_session,
            chat::clear_all_chat_sessions,
            token_usage::get_token_usage_report,
            workspace::list_workspaces,
            workspace::get_current_workspace,
            workspace::list_workspace_files,
            workspace::create_workspace,
            workspace::switch_workspace,
            workspace::clear_current_workspace,
            workspace::delete_workspace,
            workspace::open_workspace_folder,
            workspace::set_workspace_pinned,
            workspace::reorder_workspaces,
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

pub fn configure_prestart_webview() {
    services::settings_store::configure_prestart_webview();
}

#[cfg(test)]
mod double_modifier_tests {
    use super::*;

    #[test]
    fn key_repeat_during_long_press_does_not_trigger() {
        let mut detector = DoubleModifierDetector::default();
        let modifier = crate::services::hotkey::PrimaryHotkey::Alt;
        detector.key_press(Key::Alt, 1_000, modifier);
        detector.key_press(Key::Alt, 1_010, modifier);
        assert!(!detector.key_release(Key::Alt, 1_020, modifier));
    }

    #[test]
    fn two_complete_taps_trigger_on_second_release() {
        let mut detector = DoubleModifierDetector::default();
        let modifier = crate::services::hotkey::PrimaryHotkey::Alt;
        detector.key_press(Key::Alt, 1_000, modifier);
        assert!(!detector.key_release(Key::Alt, 1_050, modifier));
        detector.key_press(Key::Alt, 1_250, modifier);
        assert!(detector.key_release(Key::Alt, 1_280, modifier));
        detector.key_press(Key::Alt, 1_300, modifier);
        assert!(!detector.key_release(Key::Alt, 1_320, modifier));
    }

    #[test]
    fn alt_chord_does_not_count_as_a_tap() {
        let mut detector = DoubleModifierDetector::default();
        let modifier = crate::services::hotkey::PrimaryHotkey::Alt;
        detector.key_press(Key::Alt, 1_000, modifier);
        detector.key_press(Key::KeyC, 1_010, modifier);
        assert!(!detector.key_release(Key::Alt, 1_020, modifier));
        detector.key_press(Key::Alt, 1_100, modifier);
        assert!(!detector.key_release(Key::Alt, 1_120, modifier));
    }

    #[test]
    fn configured_ctrl_double_tap_triggers() {
        let mut detector = DoubleModifierDetector::default();
        let modifier = crate::services::hotkey::PrimaryHotkey::Ctrl;
        detector.key_press(Key::ControlLeft, 1_000, modifier);
        assert!(!detector.key_release(Key::ControlLeft, 1_050, modifier));
        detector.key_press(Key::ControlRight, 1_200, modifier);
        assert!(detector.key_release(Key::ControlRight, 1_240, modifier));
    }
}
