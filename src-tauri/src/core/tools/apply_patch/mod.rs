//! Codex-compatible apply_patch: parse `*** Begin Patch` envelopes and apply Add/Update/Delete.
mod parser;
mod seek_sequence;

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::context::{Tool, ToolContext};
use super::error::ToolError;
use super::path::{normalize_path, resolve_tool_path};
use super::path_permission::PathAccess;
use super::preview::{unified_diff, ChangeKind, ToolPreview};
use parser::{parse_patch, Hunk, ParseError, UpdateFileChunk};
use seek_sequence::seek_sequence;

pub fn register(registry: &mut super::registry::ToolRegistry) {
    registry.register(std::sync::Arc::new(ApplyPatchTool));
}

struct ApplyPatchTool;

impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Preferred file editor. Apply a Codex-style patch (NOT unified git diff). Wrap with `*** Begin Patch` / `*** End Patch`. File ops must use three asterisks: `*** Update File: path`, `*** Add File: path`, `*** Delete File: path`. Hunks use `@@` then lines starting with ` ` / `-` / `+`. Example:\n*** Begin Patch\n*** Update File: README.md\n@@\n-old\n+new\n*** End Patch"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Codex apply_patch text. Must use *** Begin Patch / *** End Patch and *** Update File: / *** Add File: / *** Delete File:. Do not pass `--- a/file` unified-diff headers."
                }
            },
            "required": ["input"]
        })
    }

    fn preview(&self, ctx: &ToolContext, args: &Value) -> Result<Option<ToolPreview>, ToolError> {
        let patch = patch_arg(args)?;
        let plan = plan_patch(ctx, &patch)?;
        let first = plan
            .ops
            .first()
            .ok_or_else(|| ToolError::new("patch contains no file operations"))?;
        Ok(Some(ToolPreview {
            path: first.display_path.clone(),
            kind: first.kind,
            old_text: first.old_text.clone(),
            new_text: first.new_text.clone(),
            unified_diff: first.diff.clone(),
        }))
    }

    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let patch = patch_arg(&args)?;
        let plan = plan_patch(ctx, &patch)?;
        for op in &plan.ops {
            apply_op(op)?;
        }
        let summary = plan
            .ops
            .iter()
            .map(|op| match op.kind {
                ChangeKind::Create => format!("A {}", op.display_path),
                ChangeKind::Delete => format!("D {}", op.display_path),
                ChangeKind::Modify => {
                    if let Some(moved) = &op.move_to {
                        format!("M {} -> {}", op.display_path, moved)
                    } else {
                        format!("M {}", op.display_path)
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(format!("applied {} file(s)\n{summary}", plan.ops.len()))
    }
}

fn patch_arg(args: &Value) -> Result<String, ToolError> {
    let raw = args
        .get("input")
        .or_else(|| args.get("patch"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if raw.is_empty() {
        return Err(ToolError::new("input patch is required"));
    }
    Ok(raw.to_string())
}

#[derive(Debug)]
struct PlannedOp {
    display_path: String,
    absolute: PathBuf,
    move_to: Option<String>,
    move_absolute: Option<PathBuf>,
    kind: ChangeKind,
    old_text: Option<String>,
    new_text: Option<String>,
    diff: String,
}

#[derive(Debug)]
struct PatchPlan {
    ops: Vec<PlannedOp>,
}

fn plan_patch(ctx: &ToolContext, patch: &str) -> Result<PatchPlan, ToolError> {
    let parsed = parse_patch(patch).map_err(|e| ToolError::new(e.to_string()))?;
    if parsed.hunks.is_empty() {
        return Err(ToolError::new("patch contains no file operations"));
    }
    let mut ops = Vec::new();
    for hunk in parsed.hunks {
        ops.push(plan_hunk(ctx, hunk)?);
    }
    Ok(PatchPlan { ops })
}

fn plan_hunk(ctx: &ToolContext, hunk: Hunk) -> Result<PlannedOp, ToolError> {
    match hunk {
        Hunk::AddFile { path, contents } => {
            let display = path_to_display(&path)?;
            let absolute = resolve_write(ctx, &display)?;
            let old = if absolute.exists() {
                Some(fs::read_to_string(&absolute).unwrap_or_default())
            } else {
                None
            };
            let kind = if old.is_some() {
                ChangeKind::Modify
            } else {
                ChangeKind::Create
            };
            let old_str = old.clone().unwrap_or_default();
            Ok(PlannedOp {
                display_path: display.clone(),
                absolute,
                move_to: None,
                move_absolute: None,
                kind,
                old_text: old,
                new_text: Some(contents.clone()),
                diff: unified_diff(&display, &old_str, &contents),
            })
        }
        Hunk::DeleteFile { path } => {
            let display = path_to_display(&path)?;
            let absolute = resolve_write(ctx, &display)?;
            if !absolute.is_file() {
                return Err(ToolError::new(format!(
                    "cannot delete missing file: {display}"
                )));
            }
            let old = fs::read_to_string(&absolute).unwrap_or_default();
            Ok(PlannedOp {
                display_path: display.clone(),
                absolute,
                move_to: None,
                move_absolute: None,
                kind: ChangeKind::Delete,
                old_text: Some(old.clone()),
                new_text: None,
                diff: unified_diff(&display, &old, ""),
            })
        }
        Hunk::UpdateFile {
            path,
            move_path,
            chunks,
        } => {
            let display = path_to_display(&path)?;
            let absolute = resolve_write(ctx, &display)?;
            if !absolute.is_file() {
                return Err(ToolError::new(format!(
                    "cannot update missing file: {display}"
                )));
            }
            let original = fs::read_to_string(&absolute)?;
            let new_contents = apply_chunks_to_contents(&original, &display, &chunks)?;
            let (move_to, move_absolute) = if let Some(dest) = move_path {
                let dest_display = path_to_display(&dest)?;
                let dest_abs = resolve_write(ctx, &dest_display)?;
                (Some(dest_display), Some(dest_abs))
            } else {
                (None, None)
            };
            let diff_path = move_to.clone().unwrap_or_else(|| display.clone());
            Ok(PlannedOp {
                display_path: display,
                absolute,
                move_to,
                move_absolute,
                kind: ChangeKind::Modify,
                old_text: Some(original.clone()),
                new_text: Some(new_contents.clone()),
                diff: unified_diff(&diff_path, &original, &new_contents),
            })
        }
    }
}

fn apply_op(op: &PlannedOp) -> Result<(), ToolError> {
    match op.kind {
        ChangeKind::Delete => {
            fs::remove_file(&op.absolute)?;
            Ok(())
        }
        ChangeKind::Create | ChangeKind::Modify => {
            let target = op.move_absolute.as_ref().unwrap_or(&op.absolute);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = op.new_text.clone().unwrap_or_default();
            fs::write(target, content)?;
            if op.move_absolute.is_some() && &op.absolute != target && op.absolute.exists() {
                fs::remove_file(&op.absolute)?;
            }
            Ok(())
        }
    }
}

fn resolve_write(ctx: &ToolContext, display: &str) -> Result<PathBuf, ToolError> {
    resolve_tool_path(ctx, display, PathAccess::Write, "apply_patch")
}

fn path_to_display(path: &Path) -> Result<String, ToolError> {
    let raw = path.to_string_lossy();
    if raw.trim().is_empty() {
        return Err(ToolError::new("patch path is empty"));
    }
    if path.is_absolute() {
        return Err(ToolError::new(format!(
            "patch paths must be relative, got {}",
            path.display()
        )));
    }
    let normalized = normalize_path(path);
    Ok(normalized.to_string_lossy().replace('\\', "/"))
}

fn apply_chunks_to_contents(
    original: &str,
    path: &str,
    chunks: &[UpdateFileChunk],
) -> Result<String, ToolError> {
    let mut original_lines: Vec<String> = original.split('\n').map(str::to_string).collect();
    if original_lines.last().is_some_and(String::is_empty) {
        original_lines.pop();
    }
    let replacements = compute_replacements(&original_lines, path, chunks)?;
    let mut new_lines = apply_replacements(original_lines, &replacements);
    if !new_lines.last().is_some_and(String::is_empty) {
        new_lines.push(String::new());
    }
    Ok(new_lines.join("\n"))
}

fn compute_replacements(
    original_lines: &[String],
    path: &str,
    chunks: &[UpdateFileChunk],
) -> Result<Vec<(usize, usize, Vec<String>)>, ToolError> {
    let mut replacements = Vec::new();
    let mut line_index = 0usize;

    for chunk in chunks {
        if let Some(ctx_line) = &chunk.change_context {
            if let Some(idx) =
                seek_sequence(original_lines, std::slice::from_ref(ctx_line), line_index, false)
            {
                line_index = idx + 1;
            } else {
                return Err(ToolError::new(format!(
                    "Failed to find context '{ctx_line}' in {path}"
                )));
            }
        }

        if chunk.old_lines.is_empty() {
            let insertion_idx = if original_lines.last().is_some_and(String::is_empty) {
                original_lines.len() - 1
            } else {
                original_lines.len()
            };
            replacements.push((insertion_idx, 0, chunk.new_lines.clone()));
            continue;
        }

        let mut pattern: &[String] = &chunk.old_lines;
        let mut found = seek_sequence(original_lines, pattern, line_index, chunk.is_end_of_file);
        let mut new_slice: &[String] = &chunk.new_lines;

        if found.is_none() && pattern.last().is_some_and(String::is_empty) {
            pattern = &pattern[..pattern.len() - 1];
            if new_slice.last().is_some_and(String::is_empty) {
                new_slice = &new_slice[..new_slice.len() - 1];
            }
            found = seek_sequence(original_lines, pattern, line_index, chunk.is_end_of_file);
        }

        if let Some(start_idx) = found {
            replacements.push((start_idx, pattern.len(), new_slice.to_vec()));
            line_index = start_idx + pattern.len();
        } else {
            return Err(ToolError::new(format!(
                "Failed to find expected lines in {path}:\n{}",
                chunk.old_lines.join("\n")
            )));
        }
    }

    replacements.sort_by_key(|(index, _, _)| *index);
    Ok(replacements)
}

fn apply_replacements(
    mut lines: Vec<String>,
    replacements: &[(usize, usize, Vec<String>)],
) -> Vec<String> {
    for (start_idx, old_len, new_segment) in replacements.iter().rev() {
        let start_idx = *start_idx;
        for _ in 0..*old_len {
            if start_idx < lines.len() {
                lines.remove(start_idx);
            }
        }
        for (offset, line) in new_segment.iter().enumerate() {
            lines.insert(start_idx + offset, line.clone());
        }
    }
    lines
}

impl From<ParseError> for ToolError {
    fn from(value: ParseError) -> Self {
        ToolError::new(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::conversation_manager::ConversationManager;
    use crate::core::event::{BusEvent, EventBus};
    use crate::core::runtime::RequestContext;
    use crate::core::tools::context::{AskStore, PathPermissionStore, ToolContext};
    use std::sync::{Arc, Mutex};

    struct NoopBus;
    impl EventBus for NoopBus {
        fn emit(&self, _event: BusEvent) {}
    }

    fn make_ctx(root: PathBuf) -> (ToolContext, PathBuf) {
        let db_path =
            std::env::temp_dir().join(format!("peek-apply-patch-{}.db", uuid::Uuid::new_v4()));
        let ctx = ToolContext {
            workspace_root: root,
            request_context: RequestContext::default(),
            session_id: "s".into(),
            assistant_message_id: "m".into(),
            conversation: Arc::new(ConversationManager::new(db_path.clone())),
            event_bus: Arc::new(NoopBus),
            tasks: Arc::new(Mutex::new(Vec::new())),
            ask_store: Arc::new(AskStore::new()),
            path_permission_store: Arc::new(PathPermissionStore::new()),
            registry: None,
            provider: None,
            subagent_depth: 0,
            max_subagent_depth: 0,
            app_handle: None,
        };
        (ctx, db_path)
    }

    #[test]
    fn parses_and_applies_add_update_delete() {
        let root = std::env::temp_dir().join(format!("peek-patch-ws-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("old.txt"), "hello\nworld\n").unwrap();
        fs::write(root.join("gone.txt"), "bye\n").unwrap();

        let patch = r#"*** Begin Patch
*** Add File: new.txt
+alpha
+beta
*** Update File: old.txt
@@
-hello
+HELLO
*** Delete File: gone.txt
*** End Patch"#;

        let (ctx, db) = make_ctx(root.clone());
        let result = ApplyPatchTool
            .execute(&ctx, json!({ "input": patch }))
            .expect("apply");
        assert!(result.contains("applied 3"));
        assert_eq!(
            fs::read_to_string(root.join("new.txt")).unwrap(),
            "alpha\nbeta\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("old.txt")).unwrap(),
            "HELLO\nworld\n"
        );
        assert!(!root.join("gone.txt").exists());
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn rejects_absolute_paths() {
        let root = std::env::temp_dir().join(format!("peek-patch-abs-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let abs = root.join("abs.txt");
        let patch = format!(
            "*** Begin Patch\n*** Add File: {}\n+x\n*** End Patch",
            abs.display()
        );
        let (ctx, db) = make_ctx(root.clone());
        let err = ApplyPatchTool
            .execute(&ctx, json!({ "input": patch }))
            .unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("relative")
                || err.to_string().to_lowercase().contains("path")
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(db);
    }
}
