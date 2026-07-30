use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::tools::context::ToolContext;
use crate::runtime::ToolManager;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolOutput {
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_files: Vec<String>,
}

impl AgentToolOutput {
    pub fn text(content: String) -> Self {
        Self {
            content,
            stdout: None,
            stderr: None,
            exit_code: None,
            changed_files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentToolError(pub String);

impl std::fmt::Display for AgentToolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AgentToolError {}

impl From<crate::core::tools::error::ToolError> for AgentToolError {
    fn from(error: crate::core::tools::error::ToolError) -> Self {
        Self(error.to_string())
    }
}

#[async_trait]
pub trait AgentTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(
        &self,
        context: &ToolContext,
        input: Value,
    ) -> Result<AgentToolOutput, AgentToolError>;
}

pub struct ShellTool {
    manager: Arc<ToolManager>,
}

impl ShellTool {
    pub fn new(manager: Arc<ToolManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl AgentTool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute one foreground PowerShell command through the existing run_shell tool."
    }

    async fn execute(
        &self,
        context: &ToolContext,
        input: Value,
    ) -> Result<AgentToolOutput, AgentToolError> {
        let command = input
            .get("command")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AgentToolError("command is required".to_string()))?;
        let output = self
            .manager
            .dispatch_async(
                context,
                "run_shell",
                json!({ "command": command, "run_in_background": false }),
            )
            .await?;
        Ok(parse_shell_output(output))
    }
}

pub struct FileTool {
    manager: Arc<ToolManager>,
}

impl FileTool {
    pub fn new(manager: Arc<ToolManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl AgentTool for FileTool {
    fn name(&self) -> &str {
        "file"
    }

    fn description(&self) -> &str {
        "Read or write a workspace file through the existing file tools."
    }

    async fn execute(
        &self,
        context: &ToolContext,
        input: Value,
    ) -> Result<AgentToolOutput, AgentToolError> {
        let operation = input.get("operation").and_then(Value::as_str).unwrap_or("");
        let path = input
            .get("path")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AgentToolError("path is required".to_string()))?;
        match operation {
            "read" => {
                let result = self
                    .manager
                    .dispatch_async(context, "read_file", json!({ "path": path }))
                    .await?;
                Ok(AgentToolOutput::text(result))
            }
            "write" => {
                let content = input
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| AgentToolError("content is required".to_string()))?;
                let result = self
                    .manager
                    .dispatch_async(
                        context,
                        "write_file",
                        json!({ "path": path, "content": content }),
                    )
                    .await?;
                Ok(AgentToolOutput {
                    changed_files: vec![path.to_string()],
                    ..AgentToolOutput::text(result)
                })
            }
            _ => Err(AgentToolError(
                "operation must be read or write".to_string(),
            )),
        }
    }
}

pub struct GitTool {
    manager: Arc<ToolManager>,
}

impl GitTool {
    pub fn new(manager: Arc<ToolManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl AgentTool for GitTool {
    fn name(&self) -> &str {
        "git"
    }

    fn description(&self) -> &str {
        "Read git status or diff through the existing Git tool."
    }

    async fn execute(
        &self,
        context: &ToolContext,
        input: Value,
    ) -> Result<AgentToolOutput, AgentToolError> {
        let action = input.get("action").and_then(Value::as_str).unwrap_or("");
        if !matches!(action, "status" | "diff") {
            return Err(AgentToolError(
                "Agent Runtime v1 GitTool supports only status and diff".to_string(),
            ));
        }
        let result = self.manager.dispatch_async(context, "git", input).await?;
        Ok(AgentToolOutput::text(result))
    }
}

fn parse_shell_output(content: String) -> AgentToolOutput {
    let mut exit_code = None;
    let mut stdout = None;
    let mut stderr = None;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("exit_code: ") {
            exit_code = value.trim().parse().ok();
        }
    }
    if let Some((_, streams)) = content.split_once('\n') {
        if let Some((out, err)) = streams.split_once("\nstderr:\n") {
            stdout = Some(out.trim_start_matches("stdout:\n").to_string());
            stderr = Some(err.to_string());
        } else {
            stdout = Some(streams.to_string());
        }
    }
    AgentToolOutput {
        content,
        stdout,
        stderr,
        exit_code,
        changed_files: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shell_streams_and_exit_code() {
        let output =
            parse_shell_output("exit_code: 2\nstdout:\ncompiled\nstderr:\nwarning".to_string());
        assert_eq!(output.exit_code, Some(2));
        assert_eq!(output.stdout.as_deref(), Some("compiled"));
        assert_eq!(output.stderr.as_deref(), Some("warning"));
    }
}
