#![allow(unused_imports)]

pub use crate::core::workspace::WorkspaceManager;

use serde_json::{json, Value};

use crate::runtime::tool::{Tool, ToolContext, ToolError};

pub struct WorkspaceTool;

impl Tool for WorkspaceTool {
    fn name(&self) -> &str {
        "get_workspace"
    }
    fn description(&self) -> &str {
        "Return the active workspace name, root, and project rules file when present (`agent.md` / `AGENTS.md`). Trust the injected workspace path when present; call this for a fresh workspace snapshot."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, _args: Value) -> Result<String, ToolError> {
        let candidates = ["agent.md", "Agent.md", "AGENTS.md", "agents.md"];
        let rules_path = candidates
            .iter()
            .map(|name| ctx.workspace_root.join(name))
            .find(|path| path.is_file());
        let rules = rules_path.as_ref().and_then(|path| {
            std::fs::read_to_string(path)
                .ok()
                .map(|content| content.chars().take(24_000).collect::<String>())
        });
        let workspace = ctx.request_context.workspace.as_ref();
        serde_json::to_string_pretty(&json!({
            "name": workspace.map(|item| item.name.as_str()),
            "root": ctx.workspace_root,
            "projectRulesFile": rules_path.as_ref().and_then(|path| {
                path.file_name().and_then(|name| name.to_str()).map(|s| s.to_string())
            }),
            "projectRules": rules,
        }))
        .map_err(|error| ToolError::new(error.to_string()))
    }
}
