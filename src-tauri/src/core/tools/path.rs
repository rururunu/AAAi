use std::path::{Component, Path, PathBuf};

use super::context::ToolContext;
use super::error::ToolError;
use super::path_permission::PathAccess;

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

pub fn resolve_path_candidate(workspace: &Path, raw: &str) -> Result<PathBuf, ToolError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ToolError::new("path is required"));
    }

    let candidate = PathBuf::from(trimmed);
    let resolved = if candidate.is_absolute() {
        candidate
    } else {
        workspace.join(candidate)
    };

    Ok(normalize_path(&resolved))
}

#[allow(dead_code)]
pub fn resolve_in_workspace(workspace: &Path, raw: &str) -> Result<PathBuf, ToolError> {
    let normalized = resolve_path_candidate(workspace, raw)?;
    let workspace = normalize_path(workspace);

    if !normalized.starts_with(&workspace) {
        return Err(ToolError::new(format!(
            "path escapes workspace: {}",
            normalized.display()
        )));
    }

    Ok(normalized)
}

pub fn resolve_tool_path(
    ctx: &ToolContext,
    raw: &str,
    access: PathAccess,
    tool_name: &str,
) -> Result<PathBuf, ToolError> {
    let normalized = resolve_path_candidate(&ctx.workspace_root, raw)?;
    let workspace = normalize_path(&ctx.workspace_root);

    // Workspace-local paths do not need a path-permission prompt.
    // Mutating tools are already gated by tool_approval (ask / auto / alwaysAllow).
    // Asking again here produced a second, redundant "询问" UI for writes.
    if normalized.starts_with(&workspace) {
        return Ok(normalized);
    }

    if ctx
        .path_permission_store
        .is_granted(ctx.root_session_id(), &normalized, access)
    {
        return Ok(normalized);
    }

    ctx.path_permission_store.request_and_grant(
        ctx.root_session_id(),
        &ctx.event_bus,
        normalized,
        access,
        tool_name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_escape() {
        let ws = PathBuf::from("/workspace/project");
        let err = resolve_in_workspace(&ws, "../outside.txt").unwrap_err();
        assert!(err.message.contains("escapes workspace"));
    }
}
