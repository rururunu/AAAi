use std::process::Command;

use serde_json::{json, Value};

use crate::runtime::terminal::prepare_command;
use crate::runtime::tool::{Tool, ToolContext, ToolError};

pub struct GitTool;

impl Tool for GitTool {
    fn name(&self) -> &str {
        "git"
    }
    fn description(&self) -> &str {
        "Inspect or commit the active Git repository. Supports current_branch, status, diff, log, and commit actions."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["current_branch", "status", "diff", "log", "commit"] },
                "message": { "type": "string", "description": "Commit message, required for commit" },
                "staged": { "type": "boolean", "default": false },
                "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 }
            },
            "required": ["action"]
        })
    }
    fn read_only(&self) -> bool {
        false
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let action = args["action"].as_str().unwrap_or_default();
        let mut command = Command::new("git");
        command.current_dir(&ctx.workspace_root);
        prepare_command(&mut command);
        match action {
            "current_branch" => {
                command.args(["branch", "--show-current"]);
            }
            "status" => {
                command.args(["status", "--short", "--branch"]);
            }
            "diff" => {
                command.arg("diff");
                if args["staged"].as_bool().unwrap_or(false) {
                    command.arg("--staged");
                }
            }
            "log" => {
                let limit = args["limit"]
                    .as_u64()
                    .unwrap_or(20)
                    .clamp(1, 100)
                    .to_string();
                command.args(["log", "--oneline", "--decorate", "-n", &limit]);
            }
            "commit" => {
                let message = args["message"].as_str().unwrap_or_default().trim();
                if message.is_empty() {
                    return Err(ToolError::new("commit message is required"));
                }
                command.args(["commit", "-m", message]);
            }
            _ => return Err(ToolError::new("unsupported git action")),
        }
        let output = command
            .output()
            .map_err(|error| ToolError::new(error.to_string()))?;
        let mut text = String::from_utf8_lossy(&output.stdout).to_string();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        if !output.status.success() {
            return Err(ToolError::new(format!(
                "git {action} failed: {}",
                text.trim()
            )));
        }
        Ok(text)
    }
}
