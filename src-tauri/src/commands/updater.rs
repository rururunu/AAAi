use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

use crate::services::app_lifecycle::{clear_allow_exit, mark_allow_exit, prepare_for_update};

const PROGRESS_EVENT: &str = "updater://progress";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
enum UpdateProgressEvent {
    Started { content_length: Option<u64> },
    Progress { chunk_length: usize },
    Finished,
}

/// Download and install the latest update.
///
/// Uses a custom `on_before_exit` that does **not** call `cleanup_before_exit`.
/// The plugin default hangs forever against our tray `prevent_exit` /
/// `prevent_close` handlers (download reaches 100%, MSI never launches).
#[tauri::command]
pub async fn download_and_install_update(app: AppHandle) -> Result<(), String> {
    prepare_for_update(&app);

    let result = install_update_inner(app).await;
    if result.is_err() {
        clear_allow_exit();
    }
    result
}

async fn install_update_inner(app: AppHandle) -> Result<(), String> {
    let updater = app
        .updater_builder()
        .on_before_exit(|| {
            // Intentionally skip `app.cleanup_before_exit()` — it deadlocks with
            // tray-style ExitRequested/CloseRequested interception on Windows.
            mark_allow_exit();
        })
        .build()
        .map_err(|error| error.to_string())?;

    let update = updater
        .check()
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No update available".to_string())?;

    let progress_app = app.clone();
    let mut started = false;

    update
        .download_and_install(
            move |chunk_length, content_length| {
                if !started {
                    started = true;
                    let _ = progress_app.emit(
                        PROGRESS_EVENT,
                        UpdateProgressEvent::Started { content_length },
                    );
                }
                let _ = progress_app.emit(
                    PROGRESS_EVENT,
                    UpdateProgressEvent::Progress { chunk_length },
                );
            },
            {
                let progress_app = app.clone();
                move || {
                    let _ = progress_app.emit(PROGRESS_EVENT, UpdateProgressEvent::Finished);
                }
            },
        )
        .await
        .map_err(|error| error.to_string())
}
