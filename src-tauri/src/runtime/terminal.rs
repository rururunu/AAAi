//! Helpers for spawning child processes without flashing a console on Windows.

use std::process::Command;

/// Prevent Windows from allocating a visible console for console-subsystem children
/// (PowerShell, cmd, git, etc.) when AAAi itself is a GUI app.
pub fn prepare_command(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    // Prefer UTF-8 from Python and similar tools so Chinese output is not CP936.
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd.env("PYTHONUTF8", "1");
}

/// Build a PowerShell `-Command` script that forces UTF-8 stdout/stderr before
/// running `command`. Windows PowerShell defaults to the system ANSI code page.
pub fn powershell_utf8_command(command: &str) -> String {
    format!(
        "$OutputEncoding = [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); {command}"
    )
}

/// Configure a PowerShell invocation for agent tools.
pub fn prepare_powershell(cmd: &mut Command, command: &str) {
    let wrapped = powershell_utf8_command(command);
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &wrapped]);
    prepare_command(cmd);
}
