use tauri::State;

use crate::app_state::AppState;
use crate::models::chat::RespondAskUserRequest;

#[tauri::command]
pub fn respond_ask_user(
    state: State<'_, AppState>,
    request: RespondAskUserRequest,
) -> Result<(), String> {
    let ok = state
        .core
        .chat()
        .ask_store()
        .complete(&request.request_id, request.answer);
    if ok {
        Ok(())
    } else {
        Err("ask request not found or already completed".into())
    }
}
