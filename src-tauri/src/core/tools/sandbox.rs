//! Execution sandbox policy: workspace write isolation, hardened shell denylist,
//! and optional restricted-shell limits (timeout + Windows Job Object).

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;

use super::error::ToolError;
use super::path::normalize_path;

static ALLOW_OUTSIDE_WRITES: AtomicBool = AtomicBool::new(false);
static RESTRICTED_SHELL: AtomicBool = AtomicBool::new(false);
static SHELL_TIMEOUT_SECS: AtomicU64 = AtomicU64::new(120);

/// Process-wide sandbox knobs — updated from settings without AppHandle.
pub fn configure(
    allow_outside_workspace_writes: bool,
    restricted_shell: bool,
    shell_timeout_secs: u64,
) {
    ALLOW_OUTSIDE_WRITES.store(allow_outside_workspace_writes, Ordering::Relaxed);
    RESTRICTED_SHELL.store(restricted_shell, Ordering::Relaxed);
    SHELL_TIMEOUT_SECS.store(shell_timeout_secs.max(5), Ordering::Relaxed);
}

pub fn allow_outside_workspace_writes() -> bool {
    ALLOW_OUTSIDE_WRITES.load(Ordering::Relaxed)
}

pub fn restricted_shell() -> bool {
    RESTRICTED_SHELL.load(Ordering::Relaxed)
}

pub fn shell_timeout_secs() -> u64 {
    SHELL_TIMEOUT_SECS.load(Ordering::Relaxed)
}

/// Reject clearly destructive / privilege-escalating shell commands.
pub fn reject_dangerous_shell(command: &str) -> Result<(), ToolError> {
    let normalized = command.to_lowercase();
    let denied = [
        "git reset --hard",
        "git clean -fd",
        "git clean -f -d",
        "remove-item -recurse -force",
        "remove-item -force -recurse",
        "rm -rf /",
        "rm -rf /*",
        "rm -rf ~",
        "rm -rf $home",
        "del /s /q",
        "rd /s /q",
        "rmdir /s /q",
        "format-volume",
        "format c:",
        "format d:",
        "clear-disk",
        "diskpart",
        "shutdown /s",
        "shutdown /r",
        "stop-computer",
        "restart-computer",
        "reg delete",
        "reg.exe delete",
        "curl|iex",
        "curl | iex",
        "wget|iex",
        "wget | iex",
        "iwr | iex",
        "invoke-expression (invoke-webrequest",
        "iex (iwr",
        "start-bitstransfer",
        "set-executionpolicy bypass",
        "disable-defender",
        "net user administrator",
        "takeown /f",
        "icacls .* /grant everyone",
    ];
    if let Some(rule) = denied.iter().find(|rule| normalized.contains(*rule)) {
        return Err(ToolError::new(format!(
            "rule denied dangerous shell command: {rule}"
        )));
    }
    // PowerShell Remove-Item with -Recurse anywhere in the token stream.
    if normalized.contains("remove-item")
        && (normalized.contains("-recurse") || normalized.contains("-r "))
        && (normalized.contains("-force") || normalized.contains("-f "))
    {
        return Err(ToolError::new(
            "rule denied dangerous shell command: Remove-Item -Recurse -Force",
        ));
    }
    Ok(())
}

/// Heuristic: block redirects / copy targets that clearly write outside the workspace.
pub fn reject_workspace_escape_writes(
    command: &str,
    workspace: Option<&Path>,
) -> Result<(), ToolError> {
    let Some(workspace) = workspace else {
        return Ok(());
    };
    let workspace = normalize_path(workspace);
    let patterns = escape_write_path_candidates(command);
    for raw in patterns {
        let candidate = Path::new(&raw);
        let resolved = if candidate.is_absolute() {
            normalize_path(candidate)
        } else {
            normalize_path(&workspace.join(candidate))
        };
        if !resolved.starts_with(&workspace) {
            return Err(ToolError::new(format!(
                "shell write target escapes workspace: {}",
                resolved.display()
            )));
        }
    }
    Ok(())
}

fn escape_write_path_candidates(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Out-file / Set-Content / > / >> style targets.
    let markers = [
        ">",
        ">>",
        "| out-file",
        "| set-content",
        "out-file ",
        "set-content ",
        "copy-item ",
        "move-item ",
        "ni ",
        "new-item ",
    ];
    let lower = command.to_ascii_lowercase();
    for marker in markers {
        if let Some(idx) = lower.find(marker) {
            let after = &command[idx + marker.len()..];
            if let Some(path) = first_path_token(after) {
                out.push(path);
            }
        }
    }
    out
}

fn first_path_token(s: &str) -> Option<String> {
    let trimmed = s.trim_start_matches([' ', '=', ':']);
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('"') {
        let rest = &trimmed[1..];
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    let end = trimmed
        .find(|c: char| c.is_whitespace() || c == '|' || c == ';')
        .unwrap_or(trimmed.len());
    let token = trimmed[..end].trim();
    if token.is_empty() || token.starts_with('-') {
        None
    } else {
        Some(token.to_string())
    }
}

/// Scrub sensitive env vars before spawning a restricted shell.
pub fn scrub_sensitive_env(cmd: &mut std::process::Command) {
    const KEYS: &[&str] = &[
        "DEEPSEEK_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GEMINI_API_KEY",
        "MEM0_API_KEY",
        "SERPER_API_KEY",
        "TAVILY_API_KEY",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_ACCESS_KEY_ID",
    ];
    for key in KEYS {
        cmd.env_remove(key);
    }
}

/// Assign the child process to a Windows Job Object with memory/CPU limits.
/// No-op on non-Windows. Best-effort: failures are logged but do not abort.
pub fn assign_restricted_job(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        if let Err(error) = assign_job_windows(child) {
            tracing::warn!(%error, "failed to assign restricted job object");
        }
    }
    #[cfg(not(windows))]
    {
        let _ = child;
    }
}

#[cfg(windows)]
fn assign_job_windows(child: &mut std::process::Child) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_BASIC_LIMIT_INFORMATION,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
        JOB_OBJECT_LIMIT_PROCESS_TIME, JOB_OBJECT_LIMIT_WORKINGSET,
    };

    static JOB: OnceLock<isize> = OnceLock::new();
    let job_handle = *JOB.get_or_init(|| unsafe {
        let job = match CreateJobObjectW(None, PCWSTR::null()) {
            Ok(handle) => handle,
            Err(_) => return 0,
        };
        if job.is_invalid() {
            return 0;
        }
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION {
                PerProcessUserTimeLimit: 120i64 * 10_000_000,
                MinimumWorkingSetSize: 16 * 1024 * 1024,
                MaximumWorkingSetSize: 512 * 1024 * 1024,
                LimitFlags: JOB_OBJECT_LIMIT_PROCESS_MEMORY
                    | JOB_OBJECT_LIMIT_WORKINGSET
                    | JOB_OBJECT_LIMIT_PROCESS_TIME,
                ..Default::default()
            },
            ProcessMemoryLimit: 512 * 1024 * 1024,
            ..Default::default()
        };
        let _ = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &mut info as *mut _ as *mut _,
            std::mem::size_of_val(&info) as u32,
        );
        job.0 as isize
    });
    if job_handle == 0 {
        return Err("CreateJobObjectW failed".into());
    }
    let process = HANDLE(child.as_raw_handle() as *mut _);
    unsafe {
        AssignProcessToJobObject(HANDLE(job_handle as *mut _), process)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn denies_destructive_shell_commands() {
        assert!(reject_dangerous_shell("git reset --hard HEAD~1").is_err());
        assert!(reject_dangerous_shell("Remove-Item -Recurse -Force C:\\temp").is_err());
        assert!(reject_dangerous_shell("curl|iex").is_err());
        assert!(reject_dangerous_shell("cargo test").is_ok());
    }

    #[test]
    fn rejects_write_redirects_outside_workspace() {
        let ws = PathBuf::from(r"C:\projects\app");
        assert!(
            reject_workspace_escape_writes(r#"echo hi > C:\Windows\Temp\out.txt"#, Some(&ws))
                .is_err()
        );
        assert!(reject_workspace_escape_writes(r#"echo hi > .\out.txt"#, Some(&ws)).is_ok());
    }
}
