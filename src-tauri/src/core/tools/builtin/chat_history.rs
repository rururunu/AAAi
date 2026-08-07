//! Chat session history builtin tools.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;

pub(super) struct ListChatsTool;

impl Tool for ListChatsTool {
    fn name(&self) -> &str {
        "list_chats"
    }
    fn description(&self) -> &str {
        "List chat session ids. Use before read_chat when you need to pick a session; for content lookup prefer search_past_chats."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, _args: Value) -> Result<String, ToolError> {
        let sessions = ctx.conversation.inner();
        let guard = sessions.lock().map_err(|_| ToolError::new("lock"))?;
        Ok(guard.keys().cloned().collect::<Vec<_>>().join("\n"))
    }
}

pub(super) struct ReadChatTool {
    pub conversation: Arc<ConversationManager>,
}

impl Tool for ReadChatTool {
    fn name(&self) -> &str {
        "read_chat"
    }
    fn description(&self) -> &str {
        "Read messages from a chat session by id. Use for conversation facts from a known session; prefer search_past_chats to locate which session first. For durable cross-chat preferences use search_memory instead."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "session_id": { "type": "string" } },
            "required": ["session_id"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let session_id = args["session_id"].as_str().unwrap_or("default");
        let messages = self.conversation.messages(session_id);
        Ok(messages
            .iter()
            .map(|m| format!("{:?}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

pub(super) struct SearchPastChatsTool {
    pub conversation: Arc<ConversationManager>,
}

impl Tool for SearchPastChatsTool {
    fn name(&self) -> &str {
        "search_past_chats"
    }
    fn description(&self) -> &str {
        "Search text across past chat sessions for conversation facts. Prefer this over guessing session ids; use read_chat to pull full context from a hit. For durable user preferences across chats use search_memory."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "query": { "type": "string" } },
            "required": ["query"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let query = args["query"].as_str().unwrap_or("").to_lowercase();
        let sessions = self.conversation.inner();
        let guard = sessions.lock().map_err(|_| ToolError::new("lock"))?;
        let mut hits = Vec::new();
        for (session_id, messages) in guard.iter() {
            for message in messages {
                if message.content.to_lowercase().contains(&query) {
                    hits.push(format!(
                        "{session_id} {:?}: {}",
                        message.role, message.content
                    ));
                }
            }
        }
        Ok(hits.join("\n"))
    }
}

pub(super) struct ListFailureCandidatesTool {
    pub conversation: Arc<ConversationManager>,
}

impl Tool for ListFailureCandidatesTool {
    fn name(&self) -> &str {
        "list_failure_candidates"
    }
    fn description(&self) -> &str {
        "Mine recent journal tool failures into reviewable rule/Skill candidates. Writes a Markdown report under .anya/candidates/; does not auto-install anything."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "min_count": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 2,
                    "description": "Minimum recurrence count to include a fingerprint"
                }
            }
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let min_count = args["min_count"].as_u64().unwrap_or(2);
        let pool = self.conversation.db_pool();
        let report = match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| {
                handle.block_on(crate::core::chat::trajectory::mine_failure_candidates(
                    &pool, min_count,
                ))
            }),
            Err(_) => tauri::async_runtime::block_on(
                crate::core::chat::trajectory::mine_failure_candidates(&pool, min_count),
            ),
        }
        .map_err(ToolError::new)?;
        let path =
            crate::core::chat::trajectory::write_candidate_report(&ctx.workspace_root, &report)
                .map_err(ToolError::new)?;
        Ok(format!(
            "candidates={} report={}",
            report.candidates.len(),
            path.display()
        ))
    }
}
