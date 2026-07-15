use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::context::{Tool, ToolContext};
use super::error::ToolError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Create,
    Modify,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPreview {
    pub path: String,
    pub kind: ChangeKind,
    pub old_text: Option<String>,
    pub new_text: Option<String>,
    pub unified_diff: String,
}

pub fn unified_diff(path: &str, old: &str, new: &str) -> String {
    let mut out = format!("--- a/{path}\n+++ b/{path}\n");
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    out.push_str(&format!("@@ -1,{} +1,{} @@\n", old_lines.len(), new_lines.len()));
    for line in &old_lines {
        out.push_str(&format!("-{line}\n"));
    }
    for line in &new_lines {
        out.push_str(&format!("+{line}\n"));
    }
    out
}

/// Optional preview hook used by approval and checkpoints.
#[allow(dead_code)]
pub trait Previewer: Tool {
    fn preview(&self, ctx: &ToolContext, args: &Value) -> Result<Option<ToolPreview>, ToolError>;
}

/// Downcast helpers for tools that implement preview via the Tool trait method.
#[allow(dead_code)]
pub fn tool_preview(
    tool: &dyn Tool,
    ctx: &ToolContext,
    args: &Value,
) -> Result<Option<ToolPreview>, ToolError> {
    tool.preview(ctx, args)
}
