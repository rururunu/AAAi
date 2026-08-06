use std::process::Command;

use serde_json::{json, Value};

use crate::runtime::terminal::prepare_command;
use crate::runtime::tool::{Tool, ToolContext, ToolError};

/// Read-only git inspection: current_branch, status, diff, log.
pub struct GitTool;

impl Tool for GitTool {
    fn name(&self) -> &str {
        "git"
    }
    fn description(&self) -> &str {
        "Inspect the active Git repository (read-only): current_branch, status, diff, and log. Prefer this over shell git for those actions. Use git_commit for commits; never invent commit success without a tool result."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["current_branch", "status", "diff", "log"] },
                "staged": { "type": "boolean", "default": false },
                "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 }
            },
            "required": ["action"]
        })
    }
    fn read_only(&self) -> bool {
        true
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
            _ => return Err(ToolError::new("unsupported git action")),
        }
        run_git(command, action)
    }
}

/// Write-capable git operation: commit.
pub struct GitCommitTool;

impl Tool for GitCommitTool {
    fn name(&self) -> &str {
        "git_commit"
    }
    fn description(&self) -> &str {
        "Commit staged changes in the active repository with a message. Prefer this over shell git commit. Only call when the user asked to commit; do not invent permission for publishing or force-push."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": { "type": "string", "description": "Commit message, required" }
            },
            "required": ["message"]
        })
    }
    fn read_only(&self) -> bool {
        false
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let message = args["message"].as_str().unwrap_or_default().trim();
        if message.is_empty() {
            return Err(ToolError::new("commit message is required"));
        }
        let mut command = Command::new("git");
        command.current_dir(&ctx.workspace_root);
        prepare_command(&mut command);
        command.args(["commit", "-m", message]);
        run_git(command, "commit")
    }
}

fn run_git(mut command: Command, action: &str) -> Result<String, ToolError> {
    let output = command
        .output()
        .map_err(|error| ToolError::new(error.to_string()))?;
    let mut text = crate::runtime::encoding::decode_process_bytes(&output.stdout);
    text.push_str(&crate::runtime::encoding::decode_process_bytes(
        &output.stderr,
    ));
    if !output.status.success() {
        return Err(ToolError::new(format!(
            "git {action} failed: {}",
            text.trim()
        )));
    }
    Ok(text)
}
