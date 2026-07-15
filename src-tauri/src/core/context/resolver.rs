use crate::core::context::store;
use crate::core::runtime::RequestContext;

/// 解析 Windows 当前上下文 — Peek 的核心差异化层。
pub struct ContextResolver;

impl ContextResolver {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve(&self) -> RequestContext {
        store::latest_request_context()
    }
}

impl Default for ContextResolver {
    fn default() -> Self {
        Self::new()
    }
}
