use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use glob::glob;
use regex::Regex;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::runtime::terminal::prepare_command;

use super::super::context::{Tool, ToolContext};
use super::super::error::ToolError;
use super::super::file_io::atomic_write;
use super::super::fuzzy::apply_old_string_edit;
use super::super::path::resolve_tool_path;
use super::super::path_permission::PathAccess;
use super::super::preview::{unified_diff, ChangeKind, ToolPreview};

fn resolve_read(
    ctx: &ToolContext,
    tool_name: &str,
    raw: &str,
) -> Result<std::path::PathBuf, ToolError> {
    resolve_tool_path(ctx, raw, PathAccess::Read, tool_name)
}

fn resolve_write(
    ctx: &ToolContext,
    tool_name: &str,
    raw: &str,
) -> Result<std::path::PathBuf, ToolError> {
    resolve_tool_path(ctx, raw, PathAccess::Write, tool_name)
}

pub struct ReadFileTool;
pub struct ListFolderTool;
pub struct FindFilesTool;
pub struct SearchFilesTool;
pub struct ListSymbolsTool;
pub struct WriteFileTool;
pub struct ReplaceInFileTool;
pub struct ReplaceManyInFileTool;
pub struct MovePathTool;
pub struct EditNotebookCellTool;
pub struct DeleteTextRangeTool;
pub struct DeleteGoSymbolTool;

fn required_string<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ToolError::new(format!("{key} is required")))
}

fn apply_many_edits(content: &str, args: &Value) -> Result<(String, usize), ToolError> {
    let edits = args
        .get("edits")
        .and_then(Value::as_array)
        .filter(|edits| !edits.is_empty())
        .ok_or_else(|| ToolError::new("edits must contain at least one replacement"))?;
    let mut updated = content.to_string();
    let mut fuzzy_count = 0usize;
    for (index, edit) in edits.iter().enumerate() {
        let old = required_string(edit, "old_string")
            .map_err(|error| ToolError::new(format!("edit {index}: {error}")))?;
        let new = edit
            .get("new_string")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new(format!("edit {index}: new_string is required")))?;
        let applied = apply_old_string_edit(&updated, old, new, false);
        if applied.applied != 1 {
            return Err(ToolError::new(format!(
                "edit {index}: old_string must appear exactly once, found {}",
                applied.matches
            )));
        }
        fuzzy_count += usize::from(applied.fuzzy);
        updated = applied.updated;
    }
    Ok((updated, fuzzy_count))
}

fn single_edit_preview(path: &str, content: String, old: &str, new: &str) -> Option<ToolPreview> {
    let applied = apply_old_string_edit(&content, old, new, false);
    if applied.applied != 1 {
        return None;
    }
    let diff = unified_diff(path, &content, &applied.updated);
    Some(ToolPreview {
        path: path.to_string(),
        affected_paths: vec![path.to_string()],
        kind: ChangeKind::Modify,
        old_text: Some(content),
        new_text: Some(applied.updated),
        unified_diff: diff,
    })
}

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Read a text file by path. Path is relative to the workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "offset": { "type": "integer" },
                "limit": { "type": "integer" }
            },
            "required": ["path"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let offset = args["offset"].as_u64().unwrap_or(1).max(1) as usize;
        let limit = args["limit"].as_u64().unwrap_or(200) as usize;
        let resolved = resolve_read(ctx, self.name(), path)?;
        let file = fs::File::open(&resolved)?;
        let reader = BufReader::new(file);
        let mut out = String::new();
        for (idx, line) in reader.lines().enumerate() {
            ctx.ensure_not_cancelled()?;
            let line_no = idx + 1;
            if line_no < offset {
                continue;
            }
            if line_no >= offset + limit {
                break;
            }
            out.push_str(&format!("{line_no:>6}|{}\n", line?));
        }
        Ok(out)
    }
}

impl Tool for ListFolderTool {
    fn name(&self) -> &str {
        "list_folder"
    }
    fn description(&self) -> &str {
        "List files and directories under a path relative to workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path relative to workspace root" },
                "recursive": { "type": "boolean" }
            },
            "required": ["path"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or(".");
        let recursive = args["recursive"].as_bool().unwrap_or(false);
        let resolved = resolve_read(ctx, self.name(), path)?;
        if !recursive {
            let mut entries = Vec::new();
            for entry in fs::read_dir(&resolved)? {
                ctx.ensure_not_cancelled()?;
                let entry = entry.map_err(|error| ToolError::new(error.to_string()))?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let kind = if entry.file_type()?.is_dir() {
                    "dir"
                } else {
                    "file"
                };
                entries.push(format!("[{kind}] {name}"));
            }
            entries.sort();
            return Ok(entries.join("\n"));
        }
        let mut lines = Vec::new();
        for entry in WalkDir::new(&resolved)
            .into_iter()
            .filter_entry(|e| !should_skip(e.path()))
        {
            ctx.ensure_not_cancelled()?;
            let entry = entry.map_err(|error| ToolError::new(error.to_string()))?;
            let rel = entry
                .path()
                .strip_prefix(&resolved)
                .unwrap_or(entry.path())
                .display();
            let kind = if entry.file_type().is_dir() {
                "dir"
            } else {
                "file"
            };
            lines.push(format!("[{kind}] {rel}"));
        }
        Ok(lines.join("\n"))
    }
}

impl Tool for FindFilesTool {
    fn name(&self) -> &str {
        "find_files"
    }
    fn description(&self) -> &str {
        "Find files by glob pattern relative to workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "path": { "type": "string", "description": "Base directory relative to workspace root (default: workspace root)" }
            },
            "required": ["pattern"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let pattern = args["pattern"].as_str().unwrap_or("");
        let base = args["path"].as_str().unwrap_or(".");
        let resolved = resolve_read(ctx, self.name(), base)?;
        let full_pattern = resolved.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();
        let mut hits = Vec::new();
        for entry in glob(&pattern_str).map_err(|e| ToolError::new(e.to_string()))? {
            ctx.ensure_not_cancelled()?;
            let path = entry.map_err(|error| ToolError::new(error.to_string()))?;
            if should_skip(&path) {
                continue;
            }
            hits.push(path.display().to_string());
            if hits.len() >= 200 {
                break;
            }
        }
        Ok(hits.join("\n"))
    }
}

impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
    }
    fn description(&self) -> &str {
        "Preferred content-search tool. Regex search in files using ripgrep when available, with an internal recursive regex fallback when it is not. Path is relative to workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "path": { "type": "string", "description": "Search directory relative to workspace root (default: workspace root)" }
            },
            "required": ["pattern"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let pattern = args["pattern"].as_str().unwrap_or("");
        let base = args["path"].as_str().unwrap_or(".");
        let resolved = resolve_read(ctx, self.name(), base)?;
        let mut rg = Command::new("rg");
        rg.args([
            "--no-heading",
            "--line-number",
            pattern,
            resolved.to_str().unwrap_or(""),
        ]);
        prepare_command(&mut rg);
        if let Some(output) = run_command_cancellable(ctx, &mut rg)? {
            if output.status.success() || !output.stdout.is_empty() {
                let text = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<_> = text.lines().take(200).collect();
                return Ok(lines.join("\n"));
            }
        }
        let re = Regex::new(pattern).map_err(|e| ToolError::new(e.to_string()))?;
        let mut hits = Vec::new();
        for entry in WalkDir::new(&resolved)
            .into_iter()
            .filter_entry(|e| !should_skip(e.path()))
        {
            ctx.ensure_not_cancelled()?;
            let entry = entry.map_err(|error| ToolError::new(error.to_string()))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(content) = fs::read_to_string(entry.path()) else {
                continue;
            };
            for (idx, line) in content.lines().enumerate() {
                ctx.ensure_not_cancelled()?;
                if re.is_match(line) {
                    hits.push(format!("{}:{}:{}", entry.path().display(), idx + 1, line));
                    if hits.len() >= 200 {
                        return Ok(hits.join("\n"));
                    }
                }
            }
        }
        Ok(hits.join("\n"))
    }
}

impl Tool for ListSymbolsTool {
    fn name(&self) -> &str {
        "list_symbols"
    }
    fn description(&self) -> &str {
        "Lightweight symbol outline for a source file. Path is relative to workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "File path relative to workspace root" } },
            "required": ["path"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let resolved = resolve_read(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        let re = Regex::new(
            r"(?m)^\s*(pub\s+)?(fn|struct|enum|trait|impl|class|def|func)\s+([A-Za-z0-9_]+)",
        )
        .map_err(|e| ToolError::new(e.to_string()))?;
        let mut out = Vec::new();
        for cap in re.captures_iter(&content) {
            ctx.ensure_not_cancelled()?;
            out.push(format!(
                "{} {}",
                cap.get(2).map(|m| m.as_str()).unwrap_or(""),
                cap.get(3).map(|m| m.as_str()).unwrap_or("")
            ));
        }
        Ok(out.join("\n"))
    }
}

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }
    fn description(&self) -> &str {
        "Create a new file, or perform an explicitly requested full-file replacement. This overwrites existing content: do not use it for localized edits to an existing file; use replace_in_file or replace_many_in_file instead. Parent directories are created automatically. Path is relative to workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "content": { "type": "string" }
            },
            "required": ["path", "content"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let content = args["content"].as_str().unwrap_or("");
        let resolved = resolve_write(ctx, self.name(), path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&resolved, content)?;
        Ok("written".into())
    }

    fn preview(&self, ctx: &ToolContext, args: &Value) -> Result<Option<ToolPreview>, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let new_text = args["content"].as_str().unwrap_or("").to_string();
        let resolved = resolve_write(ctx, self.name(), path)?;
        let (kind, old_text) = if resolved.exists() {
            (
                ChangeKind::Modify,
                Some(fs::read_to_string(&resolved).unwrap_or_default()),
            )
        } else {
            (ChangeKind::Create, None)
        };
        let old = old_text.clone().unwrap_or_default();
        Ok(Some(ToolPreview {
            path: path.to_string(),
            affected_paths: vec![path.to_string()],
            kind,
            old_text,
            new_text: Some(new_text.clone()),
            unified_diff: unified_diff(path, &old, &new_text),
        }))
    }
}

impl Tool for ReplaceInFileTool {
    fn name(&self) -> &str {
        "replace_in_file"
    }
    fn description(&self) -> &str {
        "First choice for one localized change to an existing file. Replace the smallest unique old string that remains unambiguous (exact match first, then narrow fuzzy matching for whitespace/indentation), preserving all other content. Path is relative to workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "old_string": { "type": "string" },
                "new_string": { "type": "string" }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = required_string(&args, "path")?;
        let old = required_string(&args, "old_string")?;
        let new = args
            .get("new_string")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new("new_string is required"))?;
        let resolved = resolve_write(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        let applied = apply_old_string_edit(&content, old, new, false);
        if applied.applied != 1 {
            return Err(ToolError::new(format!(
                "old_string must appear exactly once, found {}",
                applied.matches
            )));
        }
        atomic_write(&resolved, &applied.updated)?;
        if applied.fuzzy {
            Ok(format!(
                "replaced (fuzzy match, {} replacement)",
                applied.applied
            ))
        } else {
            Ok("replaced".into())
        }
    }

    fn preview(&self, ctx: &ToolContext, args: &Value) -> Result<Option<ToolPreview>, ToolError> {
        let path = required_string(args, "path")?;
        let old = required_string(args, "old_string")?;
        let new = args
            .get("new_string")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new("new_string is required"))?;
        let resolved = resolve_write(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        Ok(single_edit_preview(path, content, old, new))
    }
}

impl Tool for ReplaceManyInFileTool {
    fn name(&self) -> &str {
        "replace_many_in_file"
    }
    fn description(&self) -> &str {
        "First choice for several independent localized changes in one existing file. Apply multiple unique replacements atomically (exact then narrow fuzzy matching), preserving all other content. Use apply_patch only when the changes form a structural block rewrite or require contextual insertion/deletion. Path is relative to workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "edits": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "old_string": { "type": "string" },
                            "new_string": { "type": "string" }
                        },
                        "required": ["old_string", "new_string"]
                    }
                }
            },
            "required": ["path", "edits"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = required_string(&args, "path")?;
        let resolved = resolve_write(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        let (updated, fuzzy_count) = apply_many_edits(&content, &args)?;
        let total = args["edits"].as_array().map_or(0, Vec::len);
        atomic_write(&resolved, updated)?;
        if fuzzy_count > 0 {
            Ok(format!(
                "replaced ({total} total, {fuzzy_count} fuzzy match)"
            ))
        } else {
            Ok("replaced".into())
        }
    }

    fn preview(&self, ctx: &ToolContext, args: &Value) -> Result<Option<ToolPreview>, ToolError> {
        let path = required_string(args, "path")?;
        let resolved = resolve_write(ctx, self.name(), path)?;
        let original = fs::read_to_string(&resolved)?;
        let (content, _) = apply_many_edits(&original, args)?;
        Ok(Some(ToolPreview {
            path: path.to_string(),
            affected_paths: vec![path.to_string()],
            kind: ChangeKind::Modify,
            old_text: Some(original.clone()),
            new_text: Some(content.clone()),
            unified_diff: unified_diff(path, &original, &content),
        }))
    }
}

impl Tool for MovePathTool {
    fn name(&self) -> &str {
        "move_path"
    }
    fn description(&self) -> &str {
        "Move or rename a file or directory. Paths are relative to workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "from": { "type": "string", "description": "Source path relative to workspace root" },
                "to": { "type": "string", "description": "Destination path relative to workspace root" }
            },
            "required": ["from", "to"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let from = resolve_write(ctx, self.name(), args["from"].as_str().unwrap_or(""))?;
        let to = resolve_write(ctx, self.name(), args["to"].as_str().unwrap_or(""))?;
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(from, to)?;
        Ok("moved".into())
    }
}

impl Tool for EditNotebookCellTool {
    fn name(&self) -> &str {
        "edit_notebook_cell"
    }
    fn description(&self) -> &str {
        "Replace a Jupyter notebook cell by index. Path is relative to workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "cell_index": { "type": "integer" },
                "new_source": { "type": "string" }
            },
            "required": ["path", "cell_index", "new_source"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let index = args["cell_index"].as_u64().unwrap_or(0) as usize;
        let new_source = args["new_source"].as_str().unwrap_or("");
        let resolved = resolve_write(ctx, self.name(), path)?;
        let raw = fs::read_to_string(&resolved)?;
        let mut notebook: Value = serde_json::from_str(&raw)?;
        let cells = notebook["cells"]
            .as_array_mut()
            .ok_or_else(|| ToolError::new("invalid notebook"))?;
        let cell = cells
            .get_mut(index)
            .ok_or_else(|| ToolError::new("cell index out of range"))?;
        cell["source"] = json!(new_source.lines().collect::<Vec<_>>());
        fs::write(&resolved, serde_json::to_string_pretty(&notebook)?)?;
        Ok("updated".into())
    }
}

impl Tool for DeleteTextRangeTool {
    fn name(&self) -> &str {
        "delete_text_range"
    }
    fn description(&self) -> &str {
        "Delete text between exact start and end anchors. Path is relative to workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "start_anchor": { "type": "string" },
                "end_anchor": { "type": "string" }
            },
            "required": ["path", "start_anchor", "end_anchor"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let start = args["start_anchor"].as_str().unwrap_or("");
        let end = args["end_anchor"].as_str().unwrap_or("");
        let resolved = resolve_write(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        let start_idx = content
            .find(start)
            .ok_or_else(|| ToolError::new("start_anchor not found"))?;
        let end_idx = content[start_idx + start.len()..]
            .find(end)
            .map(|idx| start_idx + start.len() + idx + end.len())
            .ok_or_else(|| ToolError::new("end_anchor not found"))?;
        let updated = format!("{}{}", &content[..start_idx], &content[end_idx..]);
        fs::write(&resolved, updated)?;
        Ok("deleted".into())
    }
}

impl Tool for DeleteGoSymbolTool {
    fn name(&self) -> &str {
        "delete_go_symbol"
    }
    fn description(&self) -> &str {
        "Delete a Go symbol by name using gofmt-compatible heuristics. Path is relative to workspace root."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace root" },
                "symbol": { "type": "string" }
            },
            "required": ["path", "symbol"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let path = args["path"].as_str().unwrap_or("");
        let symbol = args["symbol"].as_str().unwrap_or("");
        let resolved = resolve_write(ctx, self.name(), path)?;
        let content = fs::read_to_string(&resolved)?;
        let pattern =
            format!(r"(?ms)^func\s+(\([^)]*\)\s+)?{symbol}\b.*?(?:\n\}}|\n\)\s*\{{.*?\n\}})");
        let re = Regex::new(&pattern).map_err(|e| ToolError::new(e.to_string()))?;
        let updated = re.replace(&content, "").trim().to_string() + "\n";
        fs::write(&resolved, updated)?;
        Ok("deleted".into())
    }
}

fn should_skip(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    matches!(name, ".git" | "node_modules" | "target" | "dist")
}

fn run_command_cancellable(
    ctx: &ToolContext,
    command: &mut Command,
) -> Result<Option<std::process::Output>, ToolError> {
    ctx.ensure_not_cancelled()?;
    let stdout_path =
        std::env::temp_dir().join(format!("peek-tool-{}.stdout", uuid::Uuid::new_v4()));
    let stderr_path =
        std::env::temp_dir().join(format!("peek-tool-{}.stderr", uuid::Uuid::new_v4()));
    let stdout_file = fs::File::create(&stdout_path)?;
    let stderr_file = fs::File::create(&stderr_path)?;
    command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            let _ = fs::remove_file(&stdout_path);
            let _ = fs::remove_file(&stderr_path);
            return Ok(None);
        }
    };
    let status = loop {
        if ctx.is_cancelled() {
            crate::core::tools::shell_jobs::terminate_process_tree(&mut child);
            let _ = child.wait();
            let _ = fs::remove_file(&stdout_path);
            let _ = fs::remove_file(&stderr_path);
            return Err(ToolError::cancelled());
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                let _ = fs::remove_file(&stdout_path);
                let _ = fs::remove_file(&stderr_path);
                return Err(ToolError::new(error.to_string()));
            }
        }
    };
    let stdout = fs::read(&stdout_path).unwrap_or_default();
    let stderr = fs::read(&stderr_path).unwrap_or_default();
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    Ok(Some(std::process::Output {
        status,
        stdout,
        stderr,
    }))
}

#[cfg(test)]
mod edit_tests {
    use super::*;

    #[test]
    fn replace_many_requires_at_least_one_edit() {
        let error = apply_many_edits("unchanged", &json!({ "edits": [] })).unwrap_err();
        assert!(error.to_string().contains("at least one"));
    }

    #[test]
    fn replace_many_is_all_or_nothing_in_memory() {
        let original = "one\ntwo\n";
        let error = apply_many_edits(
            original,
            &json!({
                "edits": [
                    { "old_string": "one", "new_string": "ONE" },
                    { "old_string": "missing", "new_string": "MISSING" }
                ]
            }),
        )
        .unwrap_err();
        assert!(error.to_string().contains("edit 1"));
        assert_eq!(original, "one\ntwo\n");
    }

    #[test]
    fn replace_single_preview_uses_complete_file_contents() {
        let preview =
            single_edit_preview("file.txt", "before\nold\nafter\n".into(), "old", "new").unwrap();
        assert_eq!(preview.old_text.as_deref(), Some("before\nold\nafter\n"));
        assert_eq!(preview.new_text.as_deref(), Some("before\nnew\nafter\n"));
        assert!(preview.unified_diff.contains(" before"));
        assert!(preview.unified_diff.contains("-old"));
        assert!(preview.unified_diff.contains("+new"));
    }
}
