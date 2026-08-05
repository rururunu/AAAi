//! Office runtime debug events (no sensitive document text).

use std::sync::{Mutex, OnceLock};

use crate::core::agent::AgentDebugEvent;
use crate::core::event::BusEvent;
use crate::core::tools::context::ToolContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeDebugRecord {
    pub phase: String,
    pub tool: Option<String>,
    pub success: bool,
    pub summary: String,
    pub error: Option<String>,
}

static LAST_DEBUG: OnceLock<Mutex<Option<OfficeDebugRecord>>> = OnceLock::new();

fn store() -> &'static Mutex<Option<OfficeDebugRecord>> {
    LAST_DEBUG.get_or_init(|| Mutex::new(None))
}

/// Pull-based accessor for the most recent Office debug record. Nothing in the
/// production IPC surface reads this yet (tool/context events are pushed live
/// via `emit_tool_debug`/the event bus instead), but it's kept as the
/// inspection point for a future Office debug command/panel and is exercised
/// by `context_record_round_trips` below.
#[allow(dead_code)]
pub fn last_debug_record() -> Option<OfficeDebugRecord> {
    store().lock().ok().and_then(|guard| guard.clone())
}

pub fn record_context_collection(success: bool, summary: &str, error: Option<&str>) {
    let record = OfficeDebugRecord {
        phase: "context".to_string(),
        tool: None,
        success,
        summary: summary.to_string(),
        error: error.map(str::to_string),
    };
    if let Ok(mut guard) = store().lock() {
        *guard = Some(record.clone());
    }
    tracing::debug!(
        target = "office",
        phase = "context",
        success,
        summary,
        error = error.unwrap_or_default(),
        "office context collection"
    );
}

pub fn emit_tool_debug(
    ctx: &ToolContext,
    tool: &str,
    success: bool,
    summary: &str,
    error: Option<&str>,
) {
    let record = OfficeDebugRecord {
        phase: "tool".to_string(),
        tool: Some(tool.to_string()),
        success,
        summary: summary.to_string(),
        error: error.map(str::to_string),
    };
    if let Ok(mut guard) = store().lock() {
        *guard = Some(record.clone());
    }
    tracing::debug!(
        target = "office",
        phase = "tool",
        tool,
        success,
        summary,
        error = error.unwrap_or_default(),
        "office tool execution"
    );
    ctx.event_bus.emit(BusEvent::AgentDebugEvent {
        event: AgentDebugEvent::OfficeRuntime {
            session_id: ctx.session_id.clone(),
            phase: "tool".to_string(),
            tool: Some(tool.to_string()),
            success,
            summary: summary.to_string(),
            error: error.map(str::to_string),
        },
    });
}

pub fn sanitize_summary(text: &str, max_chars: usize) -> String {
    let chars = text.chars().count();
    if chars <= max_chars {
        return format!("{chars} chars");
    }
    format!("{max_chars}+ chars (truncated)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_summary_hides_content() {
        assert_eq!(sanitize_summary("secret document body", 80), "20 chars");
        let long = "a".repeat(120);
        assert!(sanitize_summary(&long, 80).contains("truncated"));
    }

    #[test]
    fn context_record_round_trips() {
        record_context_collection(true, "word foreground", None);
        let record = last_debug_record().expect("record");
        assert_eq!(record.phase, "context");
        assert!(record.success);
    }
}
