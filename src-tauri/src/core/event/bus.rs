use crate::core::runtime::ChatMessage;

use crate::core::tools::context::TaskItem;

/// EventBus 事件 — Vue 只订阅这些，不感知 Provider。
#[derive(Debug, Clone)]
pub enum BusEvent {
    AgentEvent {
        event: crate::core::agent::AgentEventRecord,
    },
    AgentDebugEvent {
        event: crate::core::agent::AgentDebugEvent,
    },
    TokenUsage {
        model: String,
        usage: crate::core::token::TokenUsage,
    },
    SubagentStarted {
        subagent_id: String,
        parent_subagent_id: Option<String>,
        description: String,
        read_only: bool,
        depth: u32,
        timestamp_ms: u64,
    },
    SubagentProgress {
        subagent_id: String,
        kind: String,
        content: String,
        timestamp_ms: u64,
    },
    SubagentFinished {
        subagent_id: String,
        success: bool,
        summary: String,
        timestamp_ms: u64,
    },
    ChatStarted {
        session_id: String,
        user_message: ChatMessage,
        assistant_message: ChatMessage,
    },
    ChatDelta {
        session_id: String,
        message_id: String,
        delta: String,
    },
    ChatReasoning {
        session_id: String,
        message_id: String,
        content: String,
    },
    ChatStatus {
        session_id: String,
        message_id: String,
        kind: String,
    },
    ChatUserContent {
        session_id: String,
        message_id: String,
        content: String,
    },
    ChatFinished {
        session_id: String,
        message_id: String,
        content: String,
        reasoning: Option<String>,
        finish_reason: Option<String>,
    },
    ChatError {
        session_id: String,
        message_id: String,
        message: String,
    },
    ChatContextNotice {
        session_id: String,
        kind: String,
        message: String,
        usage_ratio: f32,
        folded_messages: Option<usize>,
    },
    AskUser {
        session_id: String,
        request_id: String,
        questions: Vec<crate::core::tools::context::AskQuestion>,
    },
    PathPermissionRequest {
        session_id: String,
        request_id: String,
        path: String,
        operation: String,
        tool_name: String,
    },
    ToolApprovalRequest {
        session_id: String,
        request_id: String,
        tool_name: String,
        title: String,
        arguments: serde_json::Value,
        preview: Option<crate::core::tools::preview::ToolPreview>,
    },
    PlanModeChanged {
        session_id: String,
        active: bool,
    },
    ToolStarted {
        session_id: String,
        subagent_id: Option<String>,
        parent_activity_id: Option<String>,
        message_id: String,
        activity_id: String,
        tool_name: String,
        title: String,
        kind: String,
        detail: Option<String>,
        arguments: serde_json::Value,
        preview: Option<crate::core::tools::preview::ToolPreview>,
    },
    ToolFinished {
        session_id: String,
        subagent_id: Option<String>,
        parent_activity_id: Option<String>,
        message_id: String,
        activity_id: String,
        tool_name: String,
        title: String,
        kind: String,
        detail: Option<String>,
        arguments: serde_json::Value,
        preview: Option<crate::core::tools::preview::ToolPreview>,
        result: String,
        success: bool,
    },
    TaskListUpdated {
        session_id: String,
        tasks: Vec<TaskItem>,
    },
    /// Frontend should handle navigation / UI slash commands.
    SlashCommand {
        session_id: String,
        command: String,
        args: String,
    },
}

/// 事件总线抽象 — Core 不依赖 Tauri，由 adapter 实现。
pub trait EventBus: Send + Sync {
    fn emit(&self, event: BusEvent);
}
