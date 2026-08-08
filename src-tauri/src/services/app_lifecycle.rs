//! Process-exit coordination for tray apps and Windows updater installs.
//!
//! The workbench deliberately intercepts close/exit so Anya stays in the tray.
//! Windows MSI updates call `cleanup_before_exit` (via the updater plugin) which
//! then deadlocks against those intercepts — download finishes, install never runs.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager};

use crate::services::window::{hide_overlay, is_overlay_label};

static ALLOW_EXIT: AtomicBool = AtomicBool::new(false);

pub fn allow_exit() -> bool {
    ALLOW_EXIT.load(Ordering::SeqCst)
}

pub fn mark_allow_exit() {
    ALLOW_EXIT.store(true, Ordering::SeqCst);
}

pub fn clear_allow_exit() {
    ALLOW_EXIT.store(false, Ordering::SeqCst);
}

/// Hide always-on-top overlays and allow the process to actually exit.
pub fn prepare_for_update(app: &AppHandle) {
    mark_allow_exit();

    let labels: Vec<String> = app
        .webview_windows()
        .into_keys()
        .filter(|label| is_overlay_label(label))
        .collect();

    for label in labels {
        hide_overlay(app, &label);
    }

    if let Some(workbench) = app.get_webview_window("workbench") {
        let _ = workbench.set_always_on_top(false);
    }
}
