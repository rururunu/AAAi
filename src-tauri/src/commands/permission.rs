use tauri::State;

use crate::app_state::AppState;
use crate::models::chat::RespondPathPermissionRequest;

#[tauri::command]
pub fn respond_path_permission(
    state: State<'_, AppState>,
    request: RespondPathPermissionRequest,
) -> Result<(), String> {
    let ok = state
        .core
        .chat()
        .path_permission_store()
        .complete(&request.request_id, &request.decision);
    if ok {
        Ok(())
    } else {
        Err("path permission request not found or already completed".into())
    }
}
