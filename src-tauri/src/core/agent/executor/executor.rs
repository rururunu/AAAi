use std::sync::Arc;

use serde_json::Value;

use crate::core::agent::tools::{AgentToolError, AgentToolOutput, AgentToolRegistry};
use crate::core::tools::context::ToolContext;

pub struct AgentExecutor {
    tools: Arc<AgentToolRegistry>,
}

impl AgentExecutor {
    pub fn new(tools: Arc<AgentToolRegistry>) -> Self {
        Self { tools }
    }

    pub async fn execute(
        &self,
        context: &ToolContext,
        tool: &str,
        input: Value,
    ) -> Result<AgentToolOutput, AgentToolError> {
        self.tools.execute(context, tool, input).await
    }

    pub fn tools(&self) -> Arc<AgentToolRegistry> {
        Arc::clone(&self.tools)
    }
}
