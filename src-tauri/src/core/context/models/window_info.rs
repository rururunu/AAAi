/// 前台窗口信息 — 由 WindowDetector 填充。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub pid: u32,
    pub process_name: String,
    pub title: String,
}

impl WindowInfo {
    pub fn is_explorer(&self) -> bool {
        self.process_name.eq_ignore_ascii_case("explorer.exe")
    }

    /// Terminal / console hosts where Ctrl+C means SIGINT, not “copy”.
    #[allow(dead_code)]
    pub fn is_terminal(&self) -> bool {
        let name = self.process_name.to_ascii_lowercase();
        matches!(
            name.as_str(),
            "windowsterminal.exe"
                | "cmd.exe"
                | "powershell.exe"
                | "pwsh.exe"
                | "conhost.exe"
                | "bash.exe"
                | "wsl.exe"
                | "ubuntu.exe"
                | "debian.exe"
                | "openssh.exe"
                | "ssh.exe"
                | "mintty.exe"
                | "alacritty.exe"
                | "wezterm-gui.exe"
                | "wezterm.exe"
                | "kitty.exe"
                | "hyper.exe"
                | "tabby.exe"
                | "fluentterminal.exe"
                | "windowsterminalpreview.exe"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_terminals() {
        let wt = WindowInfo {
            hwnd: 0,
            pid: 1,
            process_name: "WindowsTerminal.exe".into(),
            title: "Terminal".into(),
        };
        assert!(wt.is_terminal());
        let notepad = WindowInfo {
            hwnd: 0,
            pid: 1,
            process_name: "notepad.exe".into(),
            title: "notes".into(),
        };
        assert!(!notepad.is_terminal());
    }
}
