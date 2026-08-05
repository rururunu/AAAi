use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::chat::limits::estimate_tokens;
use crate::core::runtime::{ChatRequest, ToolCallPayload};

/// A tool call that has started executing: its UI activity has already been
/// created/emitted, so callers only need to run it and report the outcome.
pub struct StartedTool {
    pub call_id: String,
    pub tool_name: String,
    pub activity_id: String,
    pub args: serde_json::Value,
    pub preview_detail: Option<String>,
    pub tool_preview: Option<crate::core::tools::preview::ToolPreview>,
}

/// Result of running one tool call, ready to be folded back into the request
/// as a `Role::Tool` message and inspected by the completion/failure gates.
pub struct ToolOutcome {
    pub call_id: String,
    pub tool_name: String,
    /// Serialized arguments, used to detect repeated identical calls.
    pub arguments: String,
    pub result: String,
    pub success: bool,
    pub user_denied: bool,
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn merge_tool_call(calls: &mut Vec<ToolCallPayload>, incoming: ToolCallPayload) {
    if !incoming.id.is_empty() {
        if let Some(existing) = calls.iter_mut().find(|call| call.id == incoming.id) {
            if !incoming.name.is_empty() {
                existing.name = incoming.name;
            }
            existing.arguments.push_str(&incoming.arguments);
            return;
        }
    }
    calls.push(incoming);
}

pub fn estimate_request_tokens(request: &ChatRequest) -> usize {
    let message_tokens: usize = request
        .messages
        .iter()
        .map(|message| {
            estimate_tokens(&message.content)
                + estimate_tokens(message.reasoning.as_deref().unwrap_or(""))
                + 4
        })
        .sum();
    let tool_tokens: usize = request
        .tools
        .iter()
        .map(|tool| estimate_tokens(&tool.to_string()))
        .sum();
    message_tokens + tool_tokens
}
