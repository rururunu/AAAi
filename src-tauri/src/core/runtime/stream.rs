use serde::{Deserialize, Serialize};

/// 流式工具调用 — 预留 Claude / Gemini tool use。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallPayload {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// 统一流式事件 — 所有 Provider 必须映射到此枚举。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
pub enum StreamEvent {
    Start,
    Delta(String),
    Reasoning(String),
    /// Ephemeral UI status (e.g. analyzing_images). Empty kind clears.
    Status { kind: String },
    /// Persist updated user message content (e.g. image analysis tags).
    UserContentPatch {
        message_id: String,
        content: String,
    },
    ToolCall(ToolCallPayload),
    /// 一轮流式结束；若 `tool_calls` 非空则 Agent 应继续执行工具。
    TurnComplete {
        content: String,
        reasoning: Option<String>,
        tool_calls: Vec<ToolCallPayload>,
        finish_reason: Option<String>,
    },
    Finish,
    Error(String),
}
