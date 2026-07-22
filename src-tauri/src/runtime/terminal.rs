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
}
