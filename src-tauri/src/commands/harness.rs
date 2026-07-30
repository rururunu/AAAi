use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_state::AppState;
use crate::core::checkpoint::{shared_checkpoint_store, Checkpoint};
use crate::core::tools::plan_mode::shared_plan_mode_store;
use crate::core::tools::tool_approval::shared_tool_approval_store;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespondToolApprovalRequest {
    pub request_id: String,
    pub decision: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPlanModeRequest {
    pub session_id: String,
    pub active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPlanModeRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCheckpointsRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewindSessionRequest {
    pub session_id: String,
    pub turn: usize,
    /// `code` | `conversation` | `both`
    pub restore: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewindSessionResponse {
    pub restored_files: usize,
    pub truncated_messages: bool,
}

#[tauri::command]
pub fn respond_tool_approval(request: RespondToolApprovalRequest) -> Result<(), String> {
    let ok = shared_tool_approval_store().complete(&request.request_id, &request.decision);
    if ok {
        Ok(())
    } else {
        Err("tool approval request not found or already completed".into())
    }
}

#[tauri::command]
pub fn set_plan_mode(
    state: State<'_, AppState>,
    request: SetPlanModeRequest,
) -> Result<(), String> {
    shared_plan_mode_store().set_active(&request.session_id, request.active);
    state
        .core
        .chat()
        .emit_plan_mode_changed(&request.session_id, request.active);
    Ok(())
}

#[tauri::command]
pub fn get_plan_mode(request: GetPlanModeRequest) -> Result<bool, String> {
    Ok(shared_plan_mode_store().is_active(&request.session_id))
}

#[tauri::command]
pub fn list_checkpoints(request: ListCheckpointsRequest) -> Result<Vec<Checkpoint>, String> {
    shared_checkpoint_store()
        .list(&request.session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rewind_session(
    state: State<'_, AppState>,
    request: RewindSessionRequest,
) -> Result<RewindSessionResponse, String> {
    let restore = request.restore.as_str();
    if !matches!(restore, "code" | "conversation" | "both") {
        return Err("restore must be code, conversation, or both".into());
    }

    let mut restored_files = 0usize;
    let mut truncated_messages = false;

    if restore == "code" || restore == "both" {
        let session_root = state
            .core
            .chat()
            .conversation()
            .workspace_for_session(&request.session_id)
            .map(std::path::PathBuf::from);
        let root = session_root
            .or_else(|| {
                state
                    .core
                    .workspaces()
                    .current()
                    .map(|workspace| workspace.root)
            })
            .filter(|path| !path.as_os_str().is_empty())
            .ok_or_else(|| "no workspace selected for code rewind".to_string())?;
        restored_files = shared_checkpoint_store()
            .restore_code(&request.session_id, request.turn, &root)
            .map_err(|e| e.to_string())?;
    }

    if restore == "conversation" || restore == "both" {
        let checkpoints = shared_checkpoint_store()
            .list(&request.session_id)
            .map_err(|e| e.to_string())?;
        let checkpoint = checkpoints
            .iter()
            .find(|c| c.turn == request.turn)
            .ok_or_else(|| format!("checkpoint turn {} not found", request.turn))?;
        let Some(user_message_id) = &checkpoint.user_message_id else {
            return Err("checkpoint has no user_message_id for conversation rewind".into());
        };
        state
            .core
            .chat()
            .conversation()
            .truncate_from_message(&request.session_id, user_message_id)
            .map_err(|e| e.to_string())?;
        truncated_messages = true;
        // Drop later checkpoints for this session after rewind turn
        let _ = shared_checkpoint_store().drop_from_turn(&request.session_id, request.turn);
    }

    Ok(RewindSessionResponse {
        restored_files,
        truncated_messages,
    })
}
