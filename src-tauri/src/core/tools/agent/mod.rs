use std::sync::Arc;

use serde_json::{json, Value};

use crate::core::chat::agent::AgentRunner;
use crate::core::ai::provider::AIProvider;
use crate::core::event::BusEvent;
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
            | "explore_codebase"
            | "research_topic"
            | "review_code"
            | "review_security"
            | "generate_word"
    )
}

/// Prefer awaiting on the current Tokio runtime (no nested `block_on` from `spawn_blocking`).
/// Sync tools still reach this via [`block_on_tool_future`] for skills / legacy callers.
pub async fn run_subagent(
    ctx: &ToolContext,
    prompt: &str,
    read_only: bool,
    model: Option<&str>,
) -> Result<String, ToolError> {
    if !ctx.can_spawn_subagent() {
        return Err(ToolError::new("subagent depth limit reached"));
    }
    let provider = resolve_subagent_provider(ctx, model)?;
    let registry = ctx
        .registry
        .clone()
        .ok_or_else(|| ToolError::new("registry unavailable"))?;
    let child = ctx.child_subagent(prompt);
    let full_prompt = format!("{SUBAGENT_PROMPT}\n\n## Assignment\n{prompt}");
    execute_child(
        provider,
        registry,
        child,
        full_prompt,
        prompt,
        read_only,
        ctx.subagent_id.clone(),
    )
    .await
}

pub fn run_subagent_sync(ctx: &ToolContext, prompt: &str, read_only: bool) -> Result<String, ToolError> {
    run_subagent_sync_with_model(ctx, prompt, read_only, None)
}

fn run_subagent_sync_with_model(ctx: &ToolContext, prompt: &str, read_only: bool, model: Option<&str>) -> Result<String, ToolError> {
    block_on_tool_future(run_subagent(ctx, prompt, read_only, model))
}

fn resolve_subagent_provider(
    ctx: &ToolContext,
    requested_model: Option<&str>,
) -> Result<Arc<dyn AIProvider>, ToolError> {
    let Some(model) = requested_model.map(str::trim).filter(|value| !value.is_empty()) else {
        return ctx.provider.clone().ok_or_else(|| ToolError::new("provider unavailable"));
    };
    let app = ctx.app_handle.clone().ok_or_else(|| ToolError::new("model collaboration unavailable"))?;
    let settings = crate::services::settings_store::get_settings(&app)
        .map_err(|error| ToolError::new(format!("failed to load collaboration settings: {error}")))?;
    if !settings.multi_model_collaboration
        || !settings.collaboration_models.iter().any(|allowed| allowed == model)
    {
        return Err(ToolError::new(format!("model `{model}` is not enabled for collaboration")));
    }
    Ok(crate::core::ai::registry::resolve_provider_for_model(app, model.to_string()))
}

pub async fn run_parallel_subagents(
    ctx: &ToolContext,
    tasks: Vec<Value>,
) -> Result<String, ToolError> {
    use futures_util::future::join_all;

    let mut jobs = Vec::with_capacity(tasks.len());
    let parent_subagent_id = ctx.subagent_id.clone();
    for (idx, task) in tasks.into_iter().enumerate() {
        if !ctx.can_spawn_subagent() {
            return Err(ToolError::new("subagent depth limit reached"));
        }
        let prompt = task["prompt"].as_str().unwrap_or("").to_string();
        let model = task["model"].as_str().map(str::to_string);
        let child = ctx.child_subagent(&prompt);
        let provider = resolve_subagent_provider(ctx, model.as_deref()).ok();
        let registry = ctx.registry.clone();
        let parent_subagent_id = parent_subagent_id.clone();
        jobs.push(async move {
            let result = if let (Some(provider), Some(registry)) = (provider, registry) {
                let full = format!("{SUBAGENT_PROMPT}\n\n## Assignment\n{prompt}");
                execute_child(
                    provider,
                    registry,
                    child,
                    full,
                    &prompt,
                    true,
                    parent_subagent_id,
                )
                .await
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

async fn execute_child(
    provider: Arc<dyn AIProvider>,
    registry: Arc<ToolRegistry>,
    child: ToolContext,
    full_prompt: String,
    description: &str,
    read_only: bool,
    parent_subagent_id: Option<String>,
) -> Result<String, ToolError> {
    let subagent_id = child
        .subagent_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let event_bus = Arc::clone(&child.event_bus);
    event_bus.emit(BusEvent::SubagentStarted {
        subagent_id: subagent_id.clone(),
        parent_subagent_id,
        description: truncate_debug_text(description, 240),
        read_only,
        depth: child.subagent_depth,
        timestamp_ms: now_millis(),
    });

    let result = AgentRunner::run_subagent(provider, registry, child, full_prompt, read_only).await;
    let (success, summary) = match &result {
        Ok(answer) => (true, truncate_debug_text(answer, 1_200)),
        Err(error) => (false, truncate_debug_text(&error.to_string(), 1_200)),
    };
    event_bus.emit(BusEvent::SubagentFinished {
        subagent_id,
        success,
        summary,
        timestamp_ms: now_millis(),
    });
    result
}

fn truncate_debug_text(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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
            run_subagent(ctx, prompt, false, args["model"].as_str()).await
        }
        "run_readonly_subagent" => {
            let prompt = args["prompt"].as_str().unwrap_or("");
            run_subagent(ctx, prompt, true, args["model"].as_str()).await
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
            run_subagent(ctx, &prompt, false, args["model"].as_str()).await
        }
        "run_readonly_skill" => {
            let skill = args["name"].as_str().unwrap_or("");
            let task = args["task"].as_str().unwrap_or("");
            let body = crate::core::tools::skills::resolve_skill_body(skill)?;
            let prompt = format!("{body}\n\n## Task\n{task}");
            run_subagent(ctx, &prompt, true, args["model"].as_str()).await
        }
        "explore_codebase" => {
            run_builtin_skill(ctx, "explore", args["task"].as_str().unwrap_or(""), true).await
        }
        "research_topic" => {
            run_builtin_skill(ctx, "research", args["task"].as_str().unwrap_or(""), true).await
        }
        "review_code" => {
            run_builtin_skill(ctx, "review", args["task"].as_str().unwrap_or(""), true).await
        }
        "review_security" => {
            run_builtin_skill(
                ctx,
                "security_review",
                args["task"].as_str().unwrap_or(""),
                true,
            )
            .await
        }
        "generate_word" => {
            run_builtin_skill(
                ctx,
                "generate_word",
                args["task"].as_str().unwrap_or(""),
                false,
            )
            .await
        }
        other => Err(ToolError::new(format!(
            "tool `{other}` is not an async-runtime tool"
        ))),
    }
}

async fn run_builtin_skill(
    ctx: &ToolContext,
    skill: &str,
    task: &str,
    read_only: bool,
) -> Result<String, ToolError> {
    let body = crate::core::tools::skills::resolve_skill_body(skill)?;
    let prompt = format!("{body}\n\n## Task\n{task}");
    run_subagent(ctx, &prompt, read_only, None).await
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
        "Run one bounded child task after judging that delegation fits its difficulty and scope; only the final answer returns."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": { "type": "string" },
                "prompt": { "type": "string" },
                "model": { "type": "string", "description": "Coordinator-selected exact model ID. Required by policy when multi-model collaboration is enabled; otherwise optional." }
            },
            "required": ["prompt"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let prompt = args["prompt"].as_str().unwrap_or("");
        run_subagent_sync_with_model(ctx, prompt, false, args["model"].as_str())
    }
}

struct RunReadonlySubagentTool;

impl Tool for RunReadonlySubagentTool {
    fn name(&self) -> &str {
        "run_readonly_subagent"
    }
    fn description(&self) -> &str {
        "Run one bounded read-only child task when delegation improves research, exploration, review, or verification."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string" },
                "model": { "type": "string", "description": "Coordinator-selected exact model ID. Required by policy when multi-model collaboration is enabled; otherwise optional." }
            },
            "required": ["prompt"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        run_subagent_sync_with_model(ctx, args["prompt"].as_str().unwrap_or(""), true, args["model"].as_str())
    }
}

struct RunParallelSubagentsTool;

impl Tool for RunParallelSubagentsTool {
    fn name(&self) -> &str {
        "run_parallel_subagents"
    }
    fn description(&self) -> &str {
        "Run independent bounded read-only tasks concurrently when parallel delegation fits the task."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "prompt": { "type": "string" },
                            "model": { "type": "string", "description": "Coordinator-selected exact model ID for this task. Required by policy when multi-model collaboration is enabled; otherwise optional." }
                        },
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
