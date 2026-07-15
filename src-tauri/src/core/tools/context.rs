use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::event::EventBus;
use crate::core::tools::error::ToolError;
use crate::core::tools::preview::ToolPreview;

pub use crate::core::tools::path_permission::PathPermissionStore;

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    fn read_only(&self) -> bool {
        false
    }
    /// When false, the tool is omitted from model-facing schemas.
    fn available(&self) -> bool {
        true
    }

    /// Optional pre-execution preview for approval / checkpoints.
    fn preview(&self, _ctx: &ToolContext, _args: &Value) -> Result<Option<ToolPreview>, ToolError> {
        Ok(None)
    }

    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError>;

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": self.description(),
                "parameters": self.parameters_schema(),
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskItem {
    pub content: String,
    pub status: String,
    pub active_form: Option<String>,
    #[serde(default)]
    pub level: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskOption {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskQuestion {
    pub header: String,
    pub question: String,
    pub options: Vec<AskOption>,
    #[serde(default)]
    pub multi_select: bool,
}

pub struct PendingAsk {
    pub sender: mpsc::Sender<String>,
}

pub struct AskStore {
    inner: Mutex<std::collections::HashMap<String, PendingAsk>>,
}

impl AskStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn insert(&self, request_id: String, sender: mpsc::Sender<String>) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(request_id, PendingAsk { sender });
        }
    }

    pub fn complete(&self, request_id: &str, answer: String) -> bool {
        let sender = self
            .inner
            .lock()
            .ok()
            .and_then(|mut guard| guard.remove(request_id).map(|pending| pending.sender));
        if let Some(sender) = sender {
            let _ = sender.send(answer);
            return true;
        }
        false
    }
}

impl Default for AskStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct ToolContext {
    pub workspace_root: PathBuf,
    pub request_context: crate::core::runtime::RequestContext,
    pub session_id: String,
    pub assistant_message_id: String,
    pub conversation: Arc<ConversationManager>,
    pub event_bus: Arc<dyn EventBus>,
    pub tasks: Arc<Mutex<Vec<TaskItem>>>,
    pub ask_store: Arc<AskStore>,
    pub path_permission_store: Arc<PathPermissionStore>,
    pub registry: Option<Arc<super::registry::ToolRegistry>>,
    pub provider: Option<Arc<dyn crate::core::ai::provider::AIProvider>>,
    pub subagent_depth: u32,
    pub max_subagent_depth: u32,
    pub app_handle: Option<tauri::AppHandle>,
}

impl ToolContext {
    pub fn child_subagent(&self, _prompt: &str) -> ToolContext {
        ToolContext {
            workspace_root: self.workspace_root.clone(),
            request_context: self.request_context.clone(),
            session_id: format!("{}-sub", self.session_id),
            assistant_message_id: self.assistant_message_id.clone(),
            conversation: Arc::clone(&self.conversation),
            event_bus: Arc::clone(&self.event_bus),
            tasks: Arc::clone(&self.tasks),
            ask_store: Arc::clone(&self.ask_store),
            path_permission_store: Arc::clone(&self.path_permission_store),
            registry: self.registry.clone(),
            provider: self.provider.clone(),
            subagent_depth: self.subagent_depth + 1,
            max_subagent_depth: self.max_subagent_depth,
            app_handle: self.app_handle.clone(),
        }
    }

    pub fn can_spawn_subagent(&self) -> bool {
        self.subagent_depth < self.max_subagent_depth
    }
}
