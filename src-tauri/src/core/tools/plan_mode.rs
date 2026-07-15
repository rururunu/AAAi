use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::core::tools::error::ToolError;

/// Hard plan-mode gate: when active, reject non-readonly mutating tools.
pub struct PlanModeStore {
    active_sessions: Mutex<HashSet<String>>,
}

impl PlanModeStore {
    pub fn new() -> Self {
        Self {
            active_sessions: Mutex::new(HashSet::new()),
        }
    }

    pub fn set_active(&self, session_id: &str, active: bool) {
        if let Ok(mut guard) = self.active_sessions.lock() {
            if active {
                guard.insert(session_id.to_string());
            } else {
                guard.remove(session_id);
            }
        }
    }

    pub fn is_active(&self, session_id: &str) -> bool {
        self.active_sessions
            .lock()
            .ok()
            .is_some_and(|g| g.contains(session_id))
    }

    pub fn authorize(&self, session_id: &str, tool_name: &str, read_only: bool) -> Result<(), ToolError> {
        if !self.is_active(session_id) {
            return Ok(());
        }
        if plan_mode_allowed(tool_name, read_only) {
            return Ok(());
        }
        Err(ToolError::new(
            "plan mode is active: writer tools are blocked until the user approves the plan",
        ))
    }
}

fn plan_mode_allowed(tool_name: &str, read_only: bool) -> bool {
    if read_only {
        return true;
    }
    matches!(
        tool_name,
        "update_tasks" | "ask_user" | "complete_plan_step" | "todo_write"
    )
}

pub fn shared_plan_mode_store() -> &'static PlanModeStore {
    static STORE: OnceLock<PlanModeStore> = OnceLock::new();
    STORE.get_or_init(PlanModeStore::new)
}
