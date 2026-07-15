use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde_json::Value;

use super::context::ToolContext;
use super::error::ToolError;
use super::Tool;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    dynamic: RwLock<HashMap<String, Arc<dyn Tool>>>,
    /// Cached model-facing schemas; invalidated on register/unregister.
    schema_cache: RwLock<Option<Vec<Value>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            dynamic: RwLock::new(HashMap::new()),
            schema_cache: RwLock::new(None),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
        self.invalidate_schema_cache();
    }

    pub fn register_dynamic(&self, tool: Arc<dyn Tool>) {
        if let Ok(mut guard) = self.dynamic.write() {
            guard.insert(tool.name().to_string(), tool);
        }
        self.invalidate_schema_cache();
    }

    pub fn unregister_dynamic_prefix(&self, prefix: &str) {
        if let Ok(mut guard) = self.dynamic.write() {
            guard.retain(|name, _| !name.starts_with(prefix));
        }
        self.invalidate_schema_cache();
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        if let Ok(guard) = self.dynamic.read() {
            if let Some(tool) = guard.get(name) {
                return Some(Arc::clone(tool));
            }
        }
        self.tools.get(name).cloned()
    }

    pub fn schemas(&self) -> Vec<Value> {
        if let Ok(guard) = self.schema_cache.read() {
            if let Some(cached) = guard.as_ref() {
                return cached.clone();
            }
        }
        let built = self.build_schemas();
        if let Ok(mut guard) = self.schema_cache.write() {
            *guard = Some(built.clone());
        }
        built
    }

    /// Shared-pointer view of schemas for hot agent loops (cheap to clone).
    pub fn schemas_arc(&self) -> Arc<[Value]> {
        Arc::from(self.schemas())
    }

    fn build_schemas(&self) -> Vec<Value> {
        let mut names: Vec<String> = self.tools.keys().cloned().collect();
        if let Ok(guard) = self.dynamic.read() {
            names.extend(guard.keys().cloned());
        }
        names.sort();
        names.dedup();
        names
            .into_iter()
            .filter_map(|name| self.get(&name))
            .filter(|tool| tool.available())
            .map(|tool| tool.schema())
            .collect()
    }

    fn invalidate_schema_cache(&self) {
        if let Ok(mut guard) = self.schema_cache.write() {
            *guard = None;
        }
    }

    pub fn execute(&self, ctx: &ToolContext, name: &str, args: Value) -> Result<String, ToolError> {
        let tool = self.prepare_execution(ctx, name, &args)?;
        tool.execute(ctx, args)
    }

    pub(crate) fn prepare_execution(
        &self,
        ctx: &ToolContext,
        name: &str,
        args: &Value,
    ) -> Result<Arc<dyn Tool>, ToolError> {
        crate::core::rules::RuleEngine::authorize_tool(name, args)?;
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::new(format!("unknown tool: {name}")))?;
        crate::core::tools::plan_mode::shared_plan_mode_store().authorize(
            &ctx.session_id,
            tool.name(),
            tool.read_only(),
        )?;
        let preview = tool.preview(ctx, args)?;
        if let Some(preview) = &preview {
            let _ = crate::core::checkpoint::shared_checkpoint_store().snapshot_preview(
                &ctx.session_id,
                &ctx.workspace_root,
                preview,
            );
        }
        crate::core::tools::tool_approval::shared_tool_approval_store().authorize(
            ctx,
            tool.as_ref(),
            args,
            preview,
        )?;
        Ok(tool)
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tools.keys().cloned().collect();
        if let Ok(guard) = self.dynamic.read() {
            names.extend(guard.keys().cloned());
        }
        names.sort();
        names.dedup();
        names
    }

    pub fn filter_read_only(&self) -> ToolRegistry {
        let mut filtered = ToolRegistry::new();
        for name in self.names() {
            if let Some(tool) = self.get(&name) {
                if tool.read_only() {
                    filtered.register(tool);
                }
            }
        }
        filtered
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
