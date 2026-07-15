use tauri::{AppHandle, State};

use crate::app_state::AppState;
use crate::core::ai::deepseek;
use crate::core::chat::SendPreferences;
use crate::models::chat::{
    ChatCancelRequest, ChatHistoryRequest, ChatHistoryResponse, ChatModelInfo, ChatSendRequest,
    ChatSendResponse, ListChatSessionsResponse,
};
use crate::services::settings_store::get_settings;

#[tauri::command]
pub async fn chat(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ChatSendRequest,
) -> Result<ChatSendResponse, String> {
    let settings = get_settings(&app)?;
    let preferences = SendPreferences::from(&settings);
    // `reqwest::blocking::Client` owns a Tokio runtime. Creating/dropping it on a
    // tokio worker panics — keep configure off the async path.
    let settings_for_cfg = settings.clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::core::tools::memory::shared_memory_store().configure(&settings_for_cfg);
        crate::runtime::search::shared_search_runtime().configure(&settings_for_cfg);
    })
    .await
    .map_err(|error| format!("configure runtimes failed: {error}"))?;

    let result = state
        .core
        .chat()
        .send(request.session_id, request.message, preferences)
        .await
        .map_err(|error| error.to_string())?;

    Ok(ChatSendResponse {
        session_id: result.session_id,
        user_message_id: result.user_message_id,
        assistant_message_id: result.assistant_message_id,
    })
}

#[tauri::command]
pub fn chat_cancel(state: State<'_, AppState>, request: ChatCancelRequest) -> Result<(), String> {
    state
        .core
        .chat()
        .cancel(&request.message_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn chat_history(
    state: State<'_, AppState>,
    request: ChatHistoryRequest,
) -> Result<ChatHistoryResponse, String> {
    let session_id = request
        .session_id
        .unwrap_or_else(|| crate::core::runtime::DEFAULT_SESSION_ID.to_string());

    let messages = state
        .core
        .chat()
        .history(&session_id)
        .map_err(|error| error.to_string())?;

    Ok(ChatHistoryResponse {
        session_id,
        messages,
    })
}

#[tauri::command]
pub fn list_chat_sessions(state: State<'_, AppState>) -> Result<ListChatSessionsResponse, String> {
    let sessions = state.core.chat().list_sessions();
    Ok(ListChatSessionsResponse { sessions })
}

#[tauri::command]
pub async fn list_chat_models(app: AppHandle) -> Result<Vec<ChatModelInfo>, String> {
    let settings = get_settings(&app)?;
    deepseek::list_models(&settings.deepseek_api_key)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_chat_session(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    state.core.chat().conversation().delete_session(&session_id);
    Ok(())
}

#[tauri::command]
pub fn clear_all_chat_sessions(state: State<'_, AppState>) -> Result<(), String> {
    state.core.chat().conversation().clear_all_sessions();
    Ok(())
}
