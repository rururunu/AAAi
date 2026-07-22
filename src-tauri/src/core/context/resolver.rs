use crate::core::context::store;
use crate::core::runtime::RequestContext;

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
