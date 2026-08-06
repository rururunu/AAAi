//! Materialize the Anthropic-style `docx` skill scripts into the workspace.
//!
//! Vendor tree lives at `prompts/skills/vendor/docx/` (populated by `pnpm sync-skills`).
//! Playbook text is in `prompts/skills/docx.md` (paths rewritten for `.aaai/docx/scripts`).

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::tools::error::ToolError;

/// Relative destination under the workspace root (documented for callers / tests).
#[allow(dead_code)]
pub const DOCX_REL_DIR: &str = ".aaai/docx";

fn vendor_root() -> PathBuf {
    if let Ok(raw) = std::env::var("AAAI_DOCX_SKILL_VENDOR") {
        let path = PathBuf::from(raw);
        if path.is_dir() {
            return path;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("prompts")
        .join("skills")
        .join("vendor")
        .join("docx")
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), ToolError> {
    fs::create_dir_all(dst)
        .map_err(|error| ToolError::new(format!("cannot create {}: {error}", dst.display())))?;
    for entry in fs::read_dir(src)
        .map_err(|error| ToolError::new(format!("cannot read dir {}: {error}", src.display())))?
    {
        let entry = entry.map_err(|error| ToolError::new(error.to_string()))?;
        let ty = entry
            .file_type()
            .map_err(|error| ToolError::new(error.to_string()))?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else if ty.is_file() {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    ToolError::new(format!("cannot create {}: {error}", parent.display()))
                })?;
            }
            fs::copy(entry.path(), &to).map_err(|error| {
                ToolError::new(format!("cannot copy to {}: {error}", to.display()))
            })?;
        }
    }
    Ok(())
}

/// Copy vendor docx skill tree to `{workspace}/.aaai/docx/`.
pub fn materialize_docx_skill(workspace_root: &Path) -> Result<PathBuf, ToolError> {
    let vendor = vendor_root();
    let marker = vendor.join("scripts").join("merge_runs.py");
    if !marker.is_file() {
        return Err(ToolError::new(format!(
            "docx skill vendor missing at {}. Run `pnpm sync-skills` from the repo root \
             (or set AAAI_DOCX_SKILL_VENDOR to an anthropics docx skill directory).",
            vendor.display()
        )));
    }

    let dest = workspace_root.join(".aaai").join("docx");
    if dest.exists() {
        fs::remove_dir_all(&dest)
            .map_err(|error| ToolError::new(format!("cannot reset {}: {error}", dest.display())))?;
    }
    copy_dir_recursive(&vendor, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_marker_exists_when_synced() {
        let vendor = vendor_root();
        let marker = vendor.join("scripts").join("merge_runs.py");
        if !marker.is_file() {
            eprintln!(
                "skip: docx vendor not synced (run pnpm sync-skills): {}",
                vendor.display()
            );
            return;
        }
        let tmp = std::env::temp_dir().join(format!("aaai-docx-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).expect("tmpdir");
        let dest = materialize_docx_skill(&tmp).expect("materialize");
        assert!(dest.join("scripts").join("merge_runs.py").is_file());
        let _ = fs::remove_dir_all(&tmp);
    }
}
