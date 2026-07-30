use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::core::runtime::RequestContext;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPlan {
    pub steps: Vec<AgentPlanStep>,
}

impl AgentPlan {
    pub fn new(steps: Vec<AgentPlanStep>) -> Self {
        Self { steps }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPlanStep {
    pub id: String,
    pub action: String,
    pub tool: Option<String>,
    pub description: String,
    pub status: AgentPlanStepStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentPlanStepStatus {
    Planned,
    Running,
    Completed,
    Failed,
}

pub trait Planner: Send + Sync {
    fn initial_plan(&self, input: &str, context: &RequestContext) -> AgentPlan;
    fn plan_tool_call(&self, tool: &str, arguments: &Value, description: &str) -> AgentPlanStep;
}

/// Planner v1 consumes the LLM's structured tool calls as executable plan steps.
/// This keeps planning inside the existing provider turn instead of adding a
/// second model request before every chat response.
pub struct LlmPlanner;

impl Planner for LlmPlanner {
    fn initial_plan(&self, _input: &str, _context: &RequestContext) -> AgentPlan {
        AgentPlan::new(vec![AgentPlanStep {
            id: Uuid::new_v4().to_string(),
            action: "reason".to_string(),
            tool: None,
            description: "Analyze the request and produce a response or structured tool calls."
                .to_string(),
            status: AgentPlanStepStatus::Running,
        }])
    }

    fn plan_tool_call(&self, tool: &str, _arguments: &Value, description: &str) -> AgentPlanStep {
        AgentPlanStep {
            id: Uuid::new_v4().to_string(),
            action: action_for_tool(tool).to_string(),
            tool: Some(tool.to_string()),
            description: description.to_string(),
            status: AgentPlanStepStatus::Running,
        }
    }
}

fn action_for_tool(tool: &str) -> &'static str {
    match tool {
        "run_shell" | "shell" => "run_command",
        "read_file" => "read_file",
        "write_file" | "replace_in_file" | "replace_many_in_file" | "apply_patch" => "modify_file",
        "git" => "inspect_git",
        _ => "call_tool",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_is_serializable() {
        let plan = AgentPlan::new(vec![LlmPlanner.plan_tool_call(
            "run_shell",
            &serde_json::json!({ "command": "cargo check" }),
            "Check compile errors",
        )]);
        let value = serde_json::to_value(plan).unwrap();
        assert_eq!(value["steps"][0]["action"], "run_command");
        assert_eq!(value["steps"][0]["tool"], "run_shell");
    }
}
