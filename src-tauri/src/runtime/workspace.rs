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
        "Return the active workspace name, root, and project rules file when present. Trust the injected workspace path when present; call this for a fresh workspace snapshot."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, _args: Value) -> Result<String, ToolError> {
        let rules_path = ctx.workspace_root.join("AGENTS.md");
        let rules = std::fs::read_to_string(rules_path)
            .ok()
            .map(|content| content.chars().take(24_000).collect::<String>());
        let workspace = ctx.request_context.workspace.as_ref();
        serde_json::to_string_pretty(&json!({
            "name": workspace.map(|item| item.name.as_str()),
            "root": ctx.workspace_root,
            "projectRules": rules,
        }))
        .map_err(|error| ToolError::new(error.to_string()))
    }
}
