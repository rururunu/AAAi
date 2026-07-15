use std::sync::{Mutex, OnceLock};

use crate::core::context::manager::{ContextCaptureOutcome, ContextManager};
use crate::core::context::models::ChatContext;
use crate::core::runtime::RequestContext;

static CONTEXT_STORE: OnceLock<Mutex<Option<ChatContext>>> = OnceLock::new();

fn store() -> &'static Mutex<Option<ChatContext>> {
    CONTEXT_STORE.get_or_init(|| Mutex::new(None))
}

/// 在 overlay 显示前采集前台上下文并缓存。
pub fn capture_now() -> RequestContext {
    let manager = ContextManager::new();
    let captured = match manager.capture() {
        ContextCaptureOutcome::Success(context) => context,
        ContextCaptureOutcome::Empty => {
            if let Ok(guard) = store().lock() {
                if let Some(previous) = guard.as_ref() {
                    if previous.has_content() {
                        return map_to_request_context(Some(previous));
                    }
                }
            }
            ChatContext::empty()
        }
        ContextCaptureOutcome::Failed(error) => {
            eprintln!("context capture failed: {error}");
            if let Ok(guard) = store().lock() {
                if let Some(previous) = guard.as_ref() {
                    return map_to_request_context(Some(previous));
                }
            }
            ChatContext::empty()
        }
    };

    if let Ok(mut guard) = store().lock() {
        *guard = Some(captured.clone());
    }

    map_to_request_context(Some(&captured))
}

pub fn latest_request_context() -> RequestContext {
    let guard = store().lock().ok();
    let context = guard.and_then(|value| value.clone());

    map_to_request_context(context.as_ref())
}

fn map_to_request_context(context: Option<&ChatContext>) -> RequestContext {
    let Some(context) = context else {
        return RequestContext::default();
    };

    let selected_files = context
        .selected_files
        .iter()
        .map(|path| path.display().to_string())
        .collect();

    RequestContext {
        selection: context.selected_text.clone(),
        selected_files,
        active_window: None,
        workspace: None,
        clipboard: None,
    }
}
