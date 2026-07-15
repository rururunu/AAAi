use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use crate::core::event::{BusEvent, EventBus};
use crate::core::tools::error::ToolError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathAccess {
    Read,
    Write,
}

impl PathAccess {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

struct PendingPermission {
    sender: mpsc::Sender<PermissionDecision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PermissionDecision {
    AllowOnce,
    AllowAlways,
    Deny,
}

impl PermissionDecision {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "allow_once" => Some(Self::AllowOnce),
            "allow_always" => Some(Self::AllowAlways),
            "deny" => Some(Self::Deny),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathGrant {
    prefix: PathBuf,
    access: PathAccess,
}

pub struct PathPermissionStore {
    pending: Mutex<HashMap<String, PendingPermission>>,
    grants: Mutex<HashMap<String, Vec<PathGrant>>>,
}

impl PathPermissionStore {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            grants: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_granted(&self, session_id: &str, path: &Path, access: PathAccess) -> bool {
        let grants = match self.grants.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };
        let Some(prefixes) = grants.get(session_id) else {
            return false;
        };
        let normalized = super::path::normalize_path(path);
        prefixes
            .iter()
            .any(|grant| grant.access == access && path_starts_with(&normalized, &grant.prefix))
    }

    pub fn complete(&self, request_id: &str, decision: &str) -> bool {
        let Some(decision) = PermissionDecision::parse(decision) else {
            return false;
        };
        let sender = self
            .pending
            .lock()
            .ok()
            .and_then(|mut guard| guard.remove(request_id).map(|pending| pending.sender));
        if let Some(sender) = sender {
            let _ = sender.send(decision);
            return true;
        }
        false
    }

    pub fn request_and_grant(
        &self,
        session_id: &str,
        event_bus: &Arc<dyn EventBus>,
        path: PathBuf,
        access: PathAccess,
        tool_name: &str,
    ) -> Result<PathBuf, ToolError> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel();
        if let Ok(mut guard) = self.pending.lock() {
            guard.insert(request_id.clone(), PendingPermission { sender: tx });
        }

        event_bus.emit(BusEvent::PathPermissionRequest {
            session_id: session_id.to_string(),
            request_id: request_id.clone(),
            path: path.display().to_string(),
            operation: access.as_str().to_string(),
            tool_name: tool_name.to_string(),
        });

        let decision = rx
            .recv_timeout(Duration::from_secs(600))
            .map_err(|_| ToolError::new("path permission request timed out or cancelled"))?;

        if decision == PermissionDecision::Deny {
            return Err(ToolError::user_denied("path access denied by user"));
        }

        if decision == PermissionDecision::AllowAlways {
            let grant = PathGrant {
                prefix: grant_prefix_for_path(&path, access),
                access,
            };
            if let Ok(mut guard) = self.grants.lock() {
                let entry = guard.entry(session_id.to_string()).or_default();
                if !entry.iter().any(|item| item == &grant) {
                    entry.push(grant);
                }
            }
        }

        Ok(path)
    }
}

impl Default for PathPermissionStore {
    fn default() -> Self {
        Self::new()
    }
}

fn grant_prefix_for_path(path: &Path, access: PathAccess) -> PathBuf {
    let normalized = super::path::normalize_path(path);
    match access {
        PathAccess::Write => normalized
            .parent()
            .map(super::path::normalize_path)
            .unwrap_or(normalized),
        PathAccess::Read => normalized,
    }
}

fn path_starts_with(path: &Path, prefix: &Path) -> bool {
    path.starts_with(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_grant_uses_parent_directory() {
        let path = PathBuf::from(r"C:\Users\demo\Desktop\test.txt");
        let grant = grant_prefix_for_path(&path, PathAccess::Write);
        assert_eq!(grant, PathBuf::from(r"C:\Users\demo\Desktop"));
    }

    #[test]
    fn permission_decisions_are_explicit() {
        assert_eq!(
            PermissionDecision::parse("allow_once"),
            Some(PermissionDecision::AllowOnce)
        );
        assert_eq!(
            PermissionDecision::parse("allow_always"),
            Some(PermissionDecision::AllowAlways)
        );
        assert_eq!(
            PermissionDecision::parse("deny"),
            Some(PermissionDecision::Deny)
        );
        assert_eq!(PermissionDecision::parse("yes"), None);
    }
}
