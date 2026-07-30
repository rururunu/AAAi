use serde::{Deserialize, Serialize};
use serde_json::Value;
use similar::TextDiff;

use super::context::{Tool, ToolContext};
use super::error::ToolError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Create,
    Modify,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolPreview {
    pub path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_paths: Vec<String>,
    pub kind: ChangeKind,
    pub old_text: Option<String>,
    pub new_text: Option<String>,
    pub unified_diff: String,
}

pub fn unified_diff(path: &str, old: &str, new: &str) -> String {
    TextDiff::from_lines(old, new)
        .unified_diff()
        .context_radius(3)
        .header(&format!("a/{path}"), &format!("b/{path}"))
        .to_string()
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

#[cfg(test)]
mod tests {
    use super::unified_diff;

    #[test]
    fn unified_diff_counts_only_changed_lines() {
        let diff = unified_diff("src/main.rs", "one\nold\nthree\n", "one\nnew\nthree\n");
        assert_eq!(
            diff.lines()
                .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
                .count(),
            1
        );
        assert_eq!(
            diff.lines()
                .filter(|line| line.starts_with('-') && !line.starts_with("---"))
                .count(),
            1
        );
    }
}
