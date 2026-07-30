use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde_json::Value;

use crate::core::tools::context::ToolContext;

use super::{AgentTool, AgentToolError, AgentToolOutput, FileTool, GitTool, ShellTool};

pub struct AgentToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn AgentTool>>>,
}

impl AgentToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    pub fn v1(manager: Arc<crate::runtime::ToolManager>) -> Self {
        let registry = Self::new();
        registry.register(Arc::new(ShellTool::new(Arc::clone(&manager))));
        registry.register(Arc::new(FileTool::new(Arc::clone(&manager))));
        registry.register(Arc::new(GitTool::new(manager)));
        registry
    }

    pub fn register(&self, tool: Arc<dyn AgentTool>) {
        if let Ok(mut tools) = self.tools.write() {
            tools.insert(tool.name().to_string(), tool);
        }
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn AgentTool>> {
        self.tools.read().ok()?.get(name).cloned()
    }

    pub async fn execute(
        &self,
        context: &ToolContext,
        name: &str,
        input: Value,
    ) -> Result<AgentToolOutput, AgentToolError> {
        let tool = self
            .get(name)
            .ok_or_else(|| AgentToolError(format!("unknown agent tool: {name}")))?;
        tool.execute(context, input).await
    }

    pub fn names(&self) -> Vec<String> {
        let mut names = self
            .tools
            .read()
            .map(|tools| tools.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        names.sort();
        names
    }
}

impl Default for AgentToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    use crate::core::chat::conversation_manager::ConversationManager;
    use crate::core::event::{BusEvent, EventBus};
    use crate::core::runtime::RequestContext;
    use crate::core::tools::context::{AskStore, PathPermissionStore, TaskItem};

    struct NullEventBus;

    impl EventBus for NullEventBus {
        fn emit(&self, _event: BusEvent) {}
    }

    struct EchoTool;

    #[async_trait]
    impl AgentTool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "Echo input."
        }

        async fn execute(
            &self,
            _context: &ToolContext,
            input: Value,
        ) -> Result<AgentToolOutput, AgentToolError> {
            Ok(AgentToolOutput::text(
                input["value"].as_str().unwrap_or_default().to_string(),
            ))
        }
    }

    #[test]
    fn registers_and_queries_a_tool() {
        let registry = AgentToolRegistry::new();
        registry.register(Arc::new(EchoTool));
        assert!(registry.get("echo").is_some());
        assert_eq!(registry.names(), vec!["echo"]);
    }

    #[test]
    fn v1_registry_contains_only_shell_file_and_git() {
        let registry = AgentToolRegistry::v1(Arc::new(crate::runtime::ToolManager::new(
            crate::core::tools::registry::ToolRegistry::new(),
        )));
        assert_eq!(registry.names(), vec!["file", "git", "shell"]);
    }

    #[tokio::test]
    async fn executes_a_registered_tool() {
        let registry = AgentToolRegistry::new();
        registry.register(Arc::new(EchoTool));
        let db_path = std::env::temp_dir().join(format!(
            "peek-agent-tool-registry-{}.db",
            uuid::Uuid::new_v4()
        ));
        let context = ToolContext {
            workspace_root: std::env::temp_dir(),
            request_context: RequestContext::default(),
            session_id: "test".to_string(),
            assistant_message_id: "assistant".to_string(),
            conversation: Arc::new(ConversationManager::new(db_path.clone())),
            event_bus: Arc::new(NullEventBus),
            tasks: Arc::new(Mutex::new(Vec::<TaskItem>::new())),
            ask_store: Arc::new(AskStore::new()),
            path_permission_store: Arc::new(PathPermissionStore::new()),
            registry: None,
            provider: None,
            subagent_depth: 0,
            max_subagent_depth: 0,
            subagent_id: None,
            parent_activity_id: None,
            app_handle: None,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        let output = registry
            .execute(&context, "echo", serde_json::json!({ "value": "ok" }))
            .await
            .unwrap();
        assert_eq!(output.content, "ok");
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }
}
