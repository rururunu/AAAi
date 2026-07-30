use serde::{Deserialize, Serialize};

use crate::core::agent::planner::AgentPlan;

use super::AgentState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
pub enum AgentEvent {
    UserMessage {
        input: String,
    },
    StateChanged {
        from: AgentState,
        to: AgentState,
    },
    ContextCollected {
        has_workspace: bool,
        has_active_file: bool,
        ide: Option<String>,
    },
    PlanCreated {
        plan: AgentPlan,
    },
    ToolCalled {
        call_id: String,
        tool: String,
        description: String,
    },
    ToolResult {
        call_id: String,
        tool: String,
        success: bool,
        result: String,
    },
    FileChanged {
        path: String,
    },
    Error {
        message: String,
    },
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentEventRecord {
    pub run_id: String,
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub event: AgentEvent,
}
