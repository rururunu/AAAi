use std::sync::Arc;

use serde_json::{json, Value};

use crate::core::chat::agent::AgentRunner;
use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::registry::ToolRegistry;

const SUBAGENT_PROMPT: &str = include_str!("../../../../prompts/subagent.md");

pub fn register_all(registry: &mut ToolRegistry) {
    registry.register(Arc::new(RunSubagentTool));
    registry.register(Arc::new(RunReadonlySubagentTool));
    registry.register(Arc::new(RunParallelSubagentsTool));
}

pub fn is_async_runtime_tool(name: &str) -> bool {
    matches!(
        name,
        "run_subagent"
            | "run_readonly_subagent"
            | "run_parallel_subagents"
            | "run_skill"
            | "run_readonly_skill"
    )
}

/// Prefer awaiting on the current Tokio runtime (no nested `block_on` from `spawn_blocking`).
/// Sync tools still reach this via [`block_on_tool_future`] for skills / legacy callers.
pub async fn run_subagent(
    ctx: &ToolContext,
    prompt: &str,
    read_only: bool,
) -> Result<String, ToolError> {
    if !ctx.can_spawn_subagent() {
        return Err(ToolError::new("subagent depth limit reached"));
    }
    let provider = ctx
        .provider
        .clone()
        .ok_or_else(|| ToolError::new("provider unavailable"))?;
    let registry = ctx
        .registry
        .clone()
        .ok_or_else(|| ToolError::new("registry unavailable"))?;
    let child = ctx.child_subagent(prompt);
    let full_prompt = format!("{SUBAGENT_PROMPT}\n\n## Assignment\n{prompt}");
    AgentRunner::run_subagent(provider, registry, child, full_prompt, read_only).await
}

pub fn run_subagent_sync(ctx: &ToolContext, prompt: &str, read_only: bool) -> Result<String, ToolError> {
    block_on_tool_future(run_subagent(ctx, prompt, read_only))
}

pub async fn run_parallel_subagents(
    ctx: &ToolContext,
    tasks: Vec<Value>,
) -> Result<String, ToolError> {
    use futures_util::future::join_all;

    let mut jobs = Vec::with_capacity(tasks.len());
    for (idx, task) in tasks.into_iter().enumerate() {
        if !ctx.can_spawn_subagent() {
            return Err(ToolError::new("subagent depth limit reached"));
        }
        let prompt = task["prompt"].as_str().unwrap_or("").to_string();
        let child = ctx.child_subagent(&prompt);
        let provider = ctx.provider.clone();
        let registry = ctx.registry.clone();
        jobs.push(async move {
            let result = if let (Some(provider), Some(registry)) = (provider, registry) {
                let full = format!("{SUBAGENT_PROMPT}\n\n## Assignment\n{prompt}");
                AgentRunner::run_subagent(provider, registry, child, full, true).await
            } else {
                Err(ToolError::new("subagent runtime unavailable"))
            };
            (idx, result)
        });
    }

    let mut ordered = join_all(jobs).await;
    ordered.sort_by_key(|(idx, _)| *idx);

    let mut formatted = Vec::new();
    for (idx, result) in ordered {
        let result = result?;
        formatted.push(format!("### Task {}\n{result}", idx + 1));
    }
    Ok(formatted.join("\n\n"))
}

/// Run after authorization. Used by [`ToolManager::dispatch_async`].
pub async fn execute_async_tool(
    name: &str,
    ctx: &ToolContext,
    args: Value,
) -> Result<String, ToolError> {
    match name {
        "run_subagent" => {
            let prompt = args["prompt"].as_str().unwrap_or("");
            run_subagent(ctx, prompt, false).await
        }
        "run_readonly_subagent" => {
            let prompt = args["prompt"].as_str().unwrap_or("");
            run_subagent(ctx, prompt, true).await
        }
        "run_parallel_subagents" => {
            let tasks = args["tasks"].as_array().cloned().unwrap_or_default();
            run_parallel_subagents(ctx, tasks).await
        }
        "run_skill" => {
            let skill = args["name"].as_str().unwrap_or("");
            let task = args["task"].as_str().unwrap_or("");
            let body = crate::core::tools::skills::resolve_skill_body(skill)?;
            let prompt = format!("{body}\n\n## Task\n{task}");
            run_subagent(ctx, &prompt, false).await
        }
        "run_readonly_skill" => {
            let skill = args["name"].as_str().unwrap_or("");
            let task = args["task"].as_str().unwrap_or("");
            let body = crate::core::tools::skills::resolve_skill_body(skill)?;
            let prompt = format!("{body}\n\n## Task\n{task}");
            run_subagent(ctx, &prompt, true).await
        }
        other => Err(ToolError::new(format!(
            "tool `{other}` is not an async-runtime tool"
        ))),
    }
}

fn block_on_tool_future<F>(future: F) -> F::Output
where
    F: std::future::Future,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
        Err(_) => tauri::async_runtime::block_on(future),
    }
}

struct RunSubagentTool;

impl Tool for RunSubagentTool {
    fn name(&self) -> &str {
        "run_subagent"
    }
    fn description(&self) -> &str {
        "Spawn a focused sub-agent; only its final answer returns."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": { "type": "string" },
                "prompt": { "type": "string" }
            },
            "required": ["prompt"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let prompt = args["prompt"].as_str().unwrap_or("");
        run_subagent_sync(ctx, prompt, false)
    }
}

struct RunReadonlySubagentTool;

impl Tool for RunReadonlySubagentTool {
    fn name(&self) -> &str {
        "run_readonly_subagent"
    }
    fn description(&self) -> &str {
        "Spawn a read-only sub-agent for research."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "prompt": { "type": "string" } },
            "required": ["prompt"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        run_subagent_sync(ctx, args["prompt"].as_str().unwrap_or(""), true)
    }
}

struct RunParallelSubagentsTool;

impl Tool for RunParallelSubagentsTool {
    fn name(&self) -> &str {
        "run_parallel_subagents"
    }
    fn description(&self) -> &str {
        "Dispatch multiple read-only sub-agents in parallel."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": { "prompt": { "type": "string" } },
                        "required": ["prompt"]
                    }
                }
            },
            "required": ["tasks"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let tasks = args["tasks"].as_array().cloned().unwrap_or_default();
        block_on_tool_future(run_parallel_subagents(ctx, tasks))
    }
}
