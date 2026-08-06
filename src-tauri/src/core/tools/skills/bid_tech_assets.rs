//! Materialize the built-in `bid_tech` Python helpers into the workspace.
//!
//! The playbook (`generate_bid_tech.md`) tells the agent to import these modules.
//! Shipping them as `include_str!` assets keeps a single source of truth in-repo
//! and works in packaged builds where the `prompts/` tree is not on disk.

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::tools::error::ToolError;

/// Relative destination under the workspace root.
/// Relative destination under the workspace root (documented for callers / tests).
#[allow(dead_code)]
pub const BID_TECH_REL_DIR: &str = ".aaai/bid_tech";

/// `(relative path under bid_tech/, file contents)`.
const BID_TECH_FILES: &[(&str, &str)] = &[
    (
        "__init__.py",
        include_str!("../../../../prompts/skills/bid_tech/__init__.py"),
    ),
    (
        "style.py",
        include_str!("../../../../prompts/skills/bid_tech/style.py"),
    ),
    (
        "tables.py",
        include_str!("../../../../prompts/skills/bid_tech/tables.py"),
    ),
    (
        "planner.py",
        include_str!("../../../../prompts/skills/bid_tech/planner.py"),
    ),
    (
        "docx_inspect.py",
        include_str!("../../../../prompts/skills/bid_tech/docx_inspect.py"),
    ),
    (
        "gate.py",
        include_str!("../../../../prompts/skills/bid_tech/gate.py"),
    ),
    (
        "quality.py",
        include_str!("../../../../prompts/skills/bid_tech/quality.py"),
    ),
    (
        "reference.py",
        include_str!("../../../../prompts/skills/bid_tech/reference.py"),
    ),
    (
        "align.py",
        include_str!("../../../../prompts/skills/bid_tech/align.py"),
    ),
    (
        "cli.py",
        include_str!("../../../../prompts/skills/bid_tech/cli.py"),
    ),
    (
        "example_build_demo.py",
        include_str!("../../../../prompts/skills/bid_tech/example_build_demo.py"),
    ),
];

/// Write all bid_tech helpers into `{workspace}/.aaai/bid_tech/`.
///
/// Existing files are overwritten so skill updates ship on the next run.
pub fn materialize_bid_tech_lib(workspace_root: &Path) -> Result<PathBuf, ToolError> {
    let dest = workspace_root.join(".aaai").join("bid_tech");
    fs::create_dir_all(&dest).map_err(|error| {
        ToolError::new(format!(
            "cannot create bid_tech dir {}: {error}",
            dest.display()
        ))
    })?;

    for (rel, contents) in BID_TECH_FILES {
        let path = dest.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                ToolError::new(format!("cannot create {}: {error}", parent.display()))
            })?;
        }
        fs::write(&path, contents)
            .map_err(|error| ToolError::new(format!("cannot write {}: {error}", path.display())))?;
    }

    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materialize_writes_expected_modules() {
        let tmp = std::env::temp_dir().join(format!("aaai-bid-tech-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).expect("tmpdir");
        let dest = materialize_bid_tech_lib(&tmp).expect("materialize");
        assert!(dest.join("style.py").is_file());
        assert!(dest.join("tables.py").is_file());
        assert!(dest.join("gate.py").is_file());
        assert!(dest.join("quality.py").is_file());
        assert!(dest.join("reference.py").is_file());
        assert!(dest.join("docx_inspect.py").is_file());
        assert!(dest.join("__init__.py").is_file());
        let _ = fs::remove_dir_all(&tmp);
    }
}
