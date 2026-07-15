use crate::core::context::models::{CaptureError, ChatContext};
use crate::core::context::platform::WindowDetector;
use crate::core::context::provider::{CaptureProvider, CaptureResult};
use crate::core::context::strategy::{ActiveProvider, StrategyResolver};

pub enum ContextCaptureOutcome {
    Success(ChatContext),
    Empty,
    Failed(CaptureError),
}

/// Context Engine 唯一入口 — 负责采集并生成 ChatContext。
pub struct ContextManager {
    strategy: StrategyResolver,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            strategy: StrategyResolver::new(),
        }
    }

    pub fn capture(&self) -> ContextCaptureOutcome {
        let window = match WindowDetector::detect() {
            Ok(window) => window,
            Err(error) => return ContextCaptureOutcome::Failed(error),
        };

        if window.process_name.eq_ignore_ascii_case("peek.exe") {
            return ContextCaptureOutcome::Empty;
        }

        let provider = self.strategy.resolve(&window);
        let capture_result = match provider {
            ActiveProvider::Explorer(provider) => provider.capture(&window),
            ActiveProvider::Clipboard(provider) => provider.capture(&window),
        };

        match capture_result {
            CaptureResult::Success(partial) => ContextCaptureOutcome::Success(ChatContext {
                selected_text: partial.selected_text,
                selected_files: partial.selected_files,
                source: Some(partial.source),
                window: Some(window),
            }),
            CaptureResult::Empty => ContextCaptureOutcome::Success(ChatContext {
                selected_text: None,
                selected_files: Vec::new(),
                source: None,
                window: Some(window),
            }),
            CaptureResult::Failed(error) => ContextCaptureOutcome::Failed(error),
        }
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}
