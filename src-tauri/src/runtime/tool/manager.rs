use std::sync::Arc;

use serde_json::Value;

use crate::core::tools::registry::ToolRegistry;
use crate::runtime::tool::{Tool, ToolContext, ToolError};

/// The only dispatch boundary exposed to the AI runtime.
pub struct ToolManager {
    registry: Arc<ToolRegistry>,
}

#[allow(dead_code)]
impl ToolManager {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    pub(crate) fn from_registry(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }

    pub fn schemas(&self) -> Vec<Value> {
        self.registry.schemas()
    }

    pub fn schemas_arc(&self) -> std::sync::Arc<[Value]> {
        self.registry.schemas_arc()
    }

    pub fn dispatch(
        &self,
        context: &ToolContext,
        name: &str,
        arguments: Value,
    ) -> Result<String, ToolError> {
        self.registry.execute(context, name, arguments)
    }

    /// Prefer this from the agent loop: async tools avoid nested `block_on`,
    /// sync tools still run on the blocking pool (including approval waits).
    pub async fn dispatch_async(
        &self,
        context: &ToolContext,
        name: &str,
        arguments: Value,
    ) -> Result<String, ToolError> {
        if crate::core::tools::agent::is_async_runtime_tool(name) {
            let registry = Arc::clone(&self.registry);
            let auth_ctx = context.clone();
            let auth_name = name.to_string();
            let auth_args = arguments.clone();
            let tool_name = tauri::async_runtime::spawn_blocking(move || {
                let tool = registry.prepare_execution(&auth_ctx, &auth_name, &auth_args)?;
                Ok::<String, ToolError>(tool.name().to_string())
            })
            .await
            .unwrap_or_else(|error| Err(ToolError::new(format!("tool task failed: {error}"))))?;
            return crate::core::tools::agent::execute_async_tool(&tool_name, context, arguments)
                .await;
        }

        let registry = Arc::clone(&self.registry);
        let context = context.clone();
        let name = name.to_string();
        tauri::async_runtime::spawn_blocking(move || registry.execute(&context, &name, arguments))
            .await
            .unwrap_or_else(|error| Err(ToolError::new(format!("tool task failed: {error}"))))
    }

    /// MCP and other runtime adapters register tools through this method.
    pub fn register_dynamic(&self, tool: Arc<dyn Tool>) {
        self.registry.register_dynamic(tool);
    }

    pub fn names(&self) -> Vec<String> {
        self.registry.names()
    }

    pub fn read_only(&self) -> Self {
        Self::new(self.registry.filter_read_only())
    }

    /// Unknown or missing tools are treated as non-read-only so the agent stays serial.
    pub fn is_read_only(&self, name: &str) -> bool {
        self.registry
            .get(name)
            .map(|tool| tool.read_only())
            .unwrap_or(false)
    }

    pub fn registry(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::core::chat::conversation_manager::ConversationManager;
    use crate::core::event::{BusEvent, EventBus};
    use crate::core::runtime::RequestContext;
    use crate::core::tools::context::{AskStore, PathPermissionStore};
    use crate::core::tools::error::ToolError;

    struct NullEventBus;
    impl EventBus for NullEventBus {
        fn emit(&self, _event: BusEvent) {}
    }

    struct EchoTool;

    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echo a value."
        }
        fn parameters_schema(&self) -> Value {
            serde_json::json!({ "type": "object" })
        }
        fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
            Ok(args["value"].as_str().unwrap_or_default().to_string())
        }
    }

    #[test]
    fn dynamic_tools_appear_in_manager_schemas() {
        let manager = ToolManager::new(ToolRegistry::new());
        manager.register_dynamic(Arc::new(EchoTool));
        assert_eq!(manager.names(), vec!["echo"]);
        assert_eq!(manager.schemas()[0]["function"]["name"], "echo");

        let db_path = std::env::temp_dir().join(format!("peek-v3-{}.db", uuid::Uuid::new_v4()));
        let context = ToolContext {
            workspace_root: std::env::temp_dir(),
            request_context: RequestContext::default(),
            session_id: "test".into(),
            assistant_message_id: "assistant".into(),
            conversation: Arc::new(ConversationManager::new(db_path.clone())),
            event_bus: Arc::new(NullEventBus),
            tasks: Arc::new(Mutex::new(Vec::new())),
            ask_store: Arc::new(AskStore::new()),
            path_permission_store: Arc::new(PathPermissionStore::new()),
            registry: Some(manager.registry()),
            provider: None,
            subagent_depth: 0,
            max_subagent_depth: 0,
            app_handle: None,
        };
        assert_eq!(
            manager
                .dispatch(&context, "echo", serde_json::json!({ "value": "ok" }))
                .unwrap(),
            "ok"
        );
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }
}
