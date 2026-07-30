use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::core::runtime::RequestContext;

/// Infer an active file without IDE integration. Explicit absolute file paths
/// captured from the foreground application take precedence over title hints.
pub fn infer(context: &RequestContext) -> Option<PathBuf> {
    context
        .selected_files
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_absolute() && is_file_candidate(path))
        .or_else(|| context.active_window.as_deref().and_then(path_from_title))
}

pub fn infer_project_root(active_file: &Path) -> Option<PathBuf> {
    let parent = active_file.parent()?;
    git_root(parent)
        .or_else(|| project_marker_root(parent))
        .or_else(|| Some(parent.to_path_buf()))
}

fn is_file_candidate(path: &Path) -> bool {
    path.is_file() || (!path.is_dir() && path.extension().is_some())
}

fn path_from_title(title: &str) -> Option<PathBuf> {
    title
        .split(" - ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(PathBuf::from)
        .find(|path| path.is_absolute() && is_file_candidate(path))
}

fn git_root(start: &Path) -> Option<PathBuf> {
    let output = match Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(start)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            tracing::warn!(provider = "active_file_git_root", error = %error, "context provider failed");
            return None;
        }
    };
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!root.is_empty()).then(|| PathBuf::from(root))
}

fn project_marker_root(start: &Path) -> Option<PathBuf> {
    const MARKERS: [&str; 8] = [
        ".git",
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
    ];
    start
        .ancestors()
        .find(|dir| MARKERS.iter().any(|marker| dir.join(marker).exists()))
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_selected_file_has_priority_over_window_title() {
        let context = RequestContext {
            selected_files: vec![r"C:\project-a\src\main.rs".to_string()],
            active_window: Some(r"C:\project-b\index.ts - Visual Studio Code".to_string()),
            ..RequestContext::default()
        };
        assert_eq!(
            infer(&context),
            Some(PathBuf::from(r"C:\project-a\src\main.rs"))
        );
    }

    #[test]
    fn absolute_path_can_be_inferred_from_window_title() {
        let context = RequestContext {
            active_window: Some(r"C:\project-a\src\main.rs - Visual Studio Code".to_string()),
            ..RequestContext::default()
        };
        assert_eq!(
            infer(&context),
            Some(PathBuf::from(r"C:\project-a\src\main.rs"))
        );
    }
}
