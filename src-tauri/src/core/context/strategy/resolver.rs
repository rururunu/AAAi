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
