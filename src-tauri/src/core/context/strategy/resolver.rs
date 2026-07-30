use crate::core::context::models::WindowInfo;
use crate::core::context::provider::{ClipboardProvider, ExplorerProvider};

pub enum ActiveProvider<'a> {
    Explorer(&'a ExplorerProvider),
    Clipboard(&'a ClipboardProvider),
}

pub struct StrategyResolver {
    clipboard: ClipboardProvider,
    explorer: ExplorerProvider,
}

impl StrategyResolver {
    pub fn new() -> Self {
        Self {
            clipboard: ClipboardProvider::new(),
            explorer: ExplorerProvider::new(),
        }
    }

    pub fn resolve<'a>(&'a self, window: &WindowInfo) -> ActiveProvider<'a> {
        if window.is_explorer() {
            ActiveProvider::Explorer(&self.explorer)
        } else {
            ActiveProvider::Clipboard(&self.clipboard)
        }
    }
}

impl Default for StrategyResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(process_name: &str) -> WindowInfo {
        WindowInfo {
            hwnd: 1,
            pid: 42,
            process_name: process_name.to_string(),
            title: "test".to_string(),
        }
    }

    #[test]
    fn vscode_uses_clipboard_strategy() {
        let resolver = StrategyResolver::new();
        assert!(matches!(
            resolver.resolve(&window("Code.exe")),
            ActiveProvider::Clipboard(_)
        ));
    }

    #[test]
    fn explorer_uses_explorer_strategy() {
        let resolver = StrategyResolver::new();
        assert!(matches!(
            resolver.resolve(&window("explorer.exe")),
            ActiveProvider::Explorer(_)
        ));
    }
}
