mod files;

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::event::{BusEvent, EventBus};
use crate::core::tools::context::{AskQuestion, TaskItem, Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::memory::shared_memory_store;
use crate::core::tools::registry::ToolRegistry;
use crate::core::tools::shell_jobs::{run_foreground, ShellJobStore};

use files::*;

pub fn register_all(
    registry: &mut ToolRegistry,
    conversation: Arc<ConversationManager>,
    event_bus: Arc<dyn EventBus>,
) {
    let shell_jobs = ShellJobStore::new();
    let memory = shared_memory_store();
    let tasks: Arc<Mutex<Vec<TaskItem>>> = Arc::new(Mutex::new(Vec::new()));

    macro_rules! reg {
        ($tool:expr) => {
            registry.register(Arc::new($tool));
        };
    }

    reg!(ReadFileTool);
    reg!(ListFolderTool);
    reg!(FindFilesTool);
    reg!(SearchFilesTool);
    reg!(ListSymbolsTool);
    reg!(WriteFileTool);
    reg!(ReplaceInFileTool);
    reg!(ReplaceManyInFileTool);
    reg!(MovePathTool);
    reg!(EditNotebookCellTool);
    reg!(DeleteTextRangeTool);
    reg!(DeleteGoSymbolTool);

    registry.register(Arc::new(RunShellTool {
        jobs: Arc::clone(&shell_jobs),
    }));
    registry.register(Arc::new(ReadShellOutputTool {
        jobs: Arc::clone(&shell_jobs),
    }));
    registry.register(Arc::new(WaitForShellTool {
        jobs: Arc::clone(&shell_jobs),
    }));
    registry.register(Arc::new(StopShellTool { jobs: shell_jobs }));

    registry.register(Arc::new(UpdateTasksTool {
        tasks: Arc::clone(&tasks),
        event_bus: Arc::clone(&event_bus),
    }));
    registry.register(Arc::new(AskUserTool {
        event_bus: Arc::clone(&event_bus),
    }));

    registry.register(Arc::new(SaveMemoryTool {
        memory: Arc::clone(&memory),
    }));
    registry.register(Arc::new(SearchMemoryTool { memory }));
    registry.register(Arc::new(DeleteMemoryTool {
        memory: shared_memory_store(),
    }));

    registry.register(Arc::new(ListChatsTool));
    registry.register(Arc::new(ReadChatTool {
        conversation: Arc::clone(&conversation),
    }));
    registry.register(Arc::new(SearchPastChatsTool { conversation }));

    registry.register(Arc::new(CompletePlanStepTool {
        tasks: Arc::clone(&tasks),
        event_bus,
    }));
    registry.register(Arc::new(RunSlashCommandTool));
    registry.register(Arc::new(ConnectToolsTool));
    registry.register(Arc::new(ReconnectToolsTool));
    registry.register(Arc::new(InstallToolSourceTool));
    registry.register(Arc::new(LspHoverTool));
    registry.register(Arc::new(LspDefinitionTool));
    registry.register(Arc::new(LspDiagnosticsTool));
}

struct RunShellTool {
    jobs: Arc<ShellJobStore>,
}

impl Tool for RunShellTool {
    fn name(&self) -> &str {
        "run_shell"
    }
    fn description(&self) -> &str {
        "Run a PowerShell command in the project workspace directory. Prefer dedicated file tools for scoped file operations. When rtk is installed, use it to compact large command output (for example rtk grep, rtk git, rtk test, or rtk cargo), and fall back to the native command when RTK cannot express the required operation."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "run_in_background": { "type": "boolean" }
            },
            "required": ["command"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let command = args["command"].as_str().unwrap_or("");
        if args["run_in_background"].as_bool().unwrap_or(false) {
            return self
                .jobs
                .spawn_background(
                    command.to_string(),
                    Some(&ctx.workspace_root),
                    Arc::clone(&ctx.cancelled),
                );
        }
        run_foreground(command, Some(&ctx.workspace_root), &ctx.cancelled)
    }
}

struct ReadShellOutputTool {
    jobs: Arc<ShellJobStore>,
}

impl Tool for ReadShellOutputTool {
    fn name(&self) -> &str {
        "read_shell_output"
    }
    fn description(&self) -> &str {
        "Read output from a background shell job."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "job_id": { "type": "string" } },
            "required": ["job_id"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let job_id = args["job_id"].as_str().unwrap_or("");
        self.jobs.read_output(job_id)
    }
}

struct WaitForShellTool {
    jobs: Arc<ShellJobStore>,
}

impl Tool for WaitForShellTool {
    fn name(&self) -> &str {
        "wait_for_shell"
    }
    fn description(&self) -> &str {
        "Wait for a background shell job to finish."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "job_id": { "type": "string" } },
            "required": ["job_id"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        self.jobs
            .wait_job(args["job_id"].as_str().unwrap_or(""), ctx)
    }
}

struct StopShellTool {
    jobs: Arc<ShellJobStore>,
}

impl Tool for StopShellTool {
    fn name(&self) -> &str {
        "stop_shell"
    }
    fn description(&self) -> &str {
        "Stop a background shell job."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "job_id": { "type": "string" } },
            "required": ["job_id"]
        })
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        self.jobs.kill(args["job_id"].as_str().unwrap_or(""))
    }
}

struct UpdateTasksTool {
    tasks: Arc<Mutex<Vec<TaskItem>>>,
    event_bus: Arc<dyn EventBus>,
}

impl Tool for UpdateTasksTool {
    fn name(&self) -> &str {
        "update_tasks"
    }
    fn description(&self) -> &str {
        "Replace the in-session task list."
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
                            "content": { "type": "string" },
                            "status": { "type": "string" },
                            "activeForm": { "type": "string" },
                            "level": { "type": "integer" }
                        },
                        "required": ["content", "status"]
                    }
                }
            },
            "required": ["tasks"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let parsed: Vec<TaskItem> = serde_json::from_value(args["tasks"].clone())?;
        {
            let mut guard = self.tasks.lock().map_err(|_| ToolError::new("task lock"))?;
            *guard = parsed.clone();
        }
        self.event_bus.emit(BusEvent::TaskListUpdated {
            session_id: ctx.root_session_id().to_string(),
            tasks: parsed,
        });
        Ok("updated".into())
    }
}

struct AskUserTool {
    event_bus: Arc<dyn EventBus>,
}

impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }
    fn description(&self) -> &str {
        "Ask the user multiple-choice questions and wait for answers."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "header": { "type": "string" },
                            "question": { "type": "string" },
                            "options": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": { "type": "string" },
                                        "description": { "type": "string" }
                                    },
                                    "required": ["label"]
                                }
                            },
                            "multiSelect": { "type": "boolean" }
                        },
                        "required": ["header", "question", "options"]
                    }
                }
            },
            "required": ["questions"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let questions: Vec<AskQuestion> = serde_json::from_value(args["questions"].clone())?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        ctx.ask_store.insert(request_id.clone(), tx);
        self.event_bus.emit(BusEvent::AskUser {
            session_id: ctx.root_session_id().to_string(),
            request_id: request_id.clone(),
            questions,
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
        loop {
            ctx.ensure_not_cancelled()?;
            match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(answer) => return Ok(answer),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                    if std::time::Instant::now() < deadline => {}
                Err(_) => return Err(ToolError::new("ask_user timed out or disconnected")),
            }
        }
    }
}

struct SaveMemoryTool {
    memory: Arc<crate::core::tools::memory::MemoryStore>,
}

impl Tool for SaveMemoryTool {
    fn name(&self) -> &str {
        "save_memory"
    }
    fn description(&self) -> &str {
        "Save one concise, durable, user-confirmed fact for future chats. Suitable for lasting preferences, identity/profile facts, recurring workflows, durable environment constraints, scoped project conventions, repeated corrections, and long-term goals. Include project scope when applicable. Do not save secrets, guesses, generated or copied content, one-off requests, transient state, facts already supplied by current environment context, or duplicates. Follow the system memory policy before calling."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["title", "content"]
        })
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let title = args["title"].as_str().unwrap_or("").to_string();
        let content = args["content"].as_str().unwrap_or("").to_string();
        let id = self.memory.save(title, content)?;
        Ok(format!("saved id={id}"))
    }
}

struct SearchMemoryTool {
    memory: Arc<crate::core::tools::memory::MemoryStore>,
}

impl Tool for SearchMemoryTool {
    fn name(&self) -> &str {
        "search_memory"
    }
    fn description(&self) -> &str {
        "Search cross-chat memory only when prior durable context could materially affect the answer, or to locate a duplicate, correction, or user-requested deletion. Use a concise semantic query for the missing fact, not the entire message. Results may be stale or conflicting and never override the current user message or verified state."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "query": { "type": "string" } },
            "required": ["query"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        self.memory.search(args["query"].as_str().unwrap_or(""))
    }
}

struct DeleteMemoryTool {
    memory: Arc<crate::core::tools::memory::MemoryStore>,
}

impl Tool for DeleteMemoryTool {
    fn name(&self) -> &str {
        "delete_memory"
    }
    fn description(&self) -> &str {
        "Delete a saved memory by id when the user asks to forget it or an explicit correction makes it obsolete. Obtain the exact id from memory search first."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        })
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        self.memory.delete(args["id"].as_str().unwrap_or(""))
    }
}

struct ListChatsTool;

impl Tool for ListChatsTool {
    fn name(&self) -> &str {
        "list_chats"
    }
    fn description(&self) -> &str {
        "List chat session ids."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, _args: Value) -> Result<String, ToolError> {
        let sessions = ctx.conversation.inner();
        let guard = sessions.lock().map_err(|_| ToolError::new("lock"))?;
        Ok(guard.keys().cloned().collect::<Vec<_>>().join("\n"))
    }
}

struct ReadChatTool {
    conversation: Arc<ConversationManager>,
}

impl Tool for ReadChatTool {
    fn name(&self) -> &str {
        "read_chat"
    }
    fn description(&self) -> &str {
        "Read messages from a chat session."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "session_id": { "type": "string" } },
            "required": ["session_id"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let session_id = args["session_id"].as_str().unwrap_or("default");
        let messages = self.conversation.messages(session_id);
        Ok(messages
            .iter()
            .map(|m| format!("{:?}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

struct SearchPastChatsTool {
    conversation: Arc<ConversationManager>,
}

impl Tool for SearchPastChatsTool {
    fn name(&self) -> &str {
        "search_past_chats"
    }
    fn description(&self) -> &str {
        "Search text across all chat sessions."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "query": { "type": "string" } },
            "required": ["query"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let query = args["query"].as_str().unwrap_or("").to_lowercase();
        let sessions = self.conversation.inner();
        let guard = sessions.lock().map_err(|_| ToolError::new("lock"))?;
        let mut hits = Vec::new();
        for (session_id, messages) in guard.iter() {
            for message in messages {
                if message.content.to_lowercase().contains(&query) {
                    hits.push(format!(
                        "{session_id} {:?}: {}",
                        message.role, message.content
                    ));
                }
            }
        }
        Ok(hits.join("\n"))
    }
}

struct CompletePlanStepTool {
    tasks: Arc<Mutex<Vec<TaskItem>>>,
    event_bus: Arc<dyn EventBus>,
}

impl Tool for CompletePlanStepTool {
    fn name(&self) -> &str {
        "complete_plan_step"
    }
    fn description(&self) -> &str {
        "Mark a plan step complete with evidence."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "step": { "type": "string" },
                "evidence": { "type": "string" }
            },
            "required": ["step", "evidence"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let step = args["step"].as_str().unwrap_or("");
        let evidence = args["evidence"].as_str().unwrap_or("");
        if evidence.trim().is_empty() {
            return Err(ToolError::new("evidence is required"));
        }
        let mut guard = self.tasks.lock().map_err(|_| ToolError::new("task lock"))?;
        for task in guard.iter_mut() {
            if task.content == step {
                task.status = "completed".into();
            }
        }
        self.event_bus.emit(BusEvent::TaskListUpdated {
            session_id: ctx.root_session_id().to_string(),
            tasks: guard.clone(),
        });
        Ok(format!("completed step with evidence: {evidence}"))
    }
}

struct RunSlashCommandTool;

impl Tool for RunSlashCommandTool {
    fn name(&self) -> &str {
        "run_slash_command"
    }
    fn description(&self) -> &str {
        "Run a slash command. Known commands: history, model, plan, settings, work, exit, compact, clear, context. Emits a UI event for frontend-handled commands."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Command name without leading slash" },
                "args": { "type": "string" }
            },
            "required": ["command"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let raw = args["command"].as_str().unwrap_or("").trim();
        let command = raw.trim_start_matches('/').to_ascii_lowercase();
        let extra = args["args"].as_str().unwrap_or("").trim().to_string();
        if command.is_empty() {
            return Err(ToolError::new("command is required"));
        }

        let known = [
            "history", "model", "plan", "settings", "work", "exit", "compact", "clear", "context",
        ];
        if !known.contains(&command.as_str()) {
            return Err(ToolError::new(format!(
                "unknown slash command `/{command}`. Known: {}",
                known.map(|c| format!("/{c}")).join(", ")
            )));
        }

        match command.as_str() {
            "context" => serde_json::to_string_pretty(&ctx.request_context)
                .map_err(|error| ToolError::new(error.to_string())),
            "compact" => Ok(
                "Slash /compact acknowledged — context compaction runs automatically when history exceeds the size threshold."
                    .into(),
            ),
            "clear" => {
                ctx.event_bus.emit(BusEvent::SlashCommand {
                    session_id: ctx.root_session_id().to_string(),
                    command: command.clone(),
                    args: extra,
                });
                Ok("Slash /clear requested — frontend should clear the visible conversation.".into())
            }
            other => {
                ctx.event_bus.emit(BusEvent::SlashCommand {
                    session_id: ctx.root_session_id().to_string(),
                    command: other.to_string(),
                    args: extra.clone(),
                });
                Ok(format!(
                    "Slash /{other}{} dispatched to UI",
                    if extra.is_empty() {
                        String::new()
                    } else {
                        format!(" {extra}")
                    }
                ))
            }
        }
    }
}

struct ConnectToolsTool;

impl Tool for ConnectToolsTool {
    fn name(&self) -> &str {
        "connect_tools"
    }
    fn description(&self) -> &str {
        "Connect a configured MCP server by id and register its tools as mcp__{id}__{tool}."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "source": { "type": "string", "description": "MCP server id from Settings" } },
            "required": ["source"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let source = args["source"].as_str().unwrap_or("");
        let registry = ctx
            .registry
            .as_ref()
            .ok_or_else(|| ToolError::new("tool registry unavailable"))?;
        let count = crate::core::mcp::shared_mcp_manager().connect_by_id(source, registry)?;
        Ok(format!("connected MCP `{source}` with {count} tools"))
    }
}

struct ReconnectToolsTool;

impl Tool for ReconnectToolsTool {
    fn name(&self) -> &str {
        "reconnect_tools"
    }
    fn description(&self) -> &str {
        "Disconnect and reconnect a configured MCP server by id, refreshing its mcp__{id}__* tools."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "source": { "type": "string", "description": "MCP server id from Settings" } },
            "required": ["source"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let source = args["source"].as_str().unwrap_or("");
        let registry = ctx
            .registry
            .as_ref()
            .ok_or_else(|| ToolError::new("tool registry unavailable"))?;
        let count = crate::core::mcp::shared_mcp_manager().reconnect_by_id(source, registry)?;
        Ok(format!("reconnected MCP `{source}` with {count} tools"))
    }
}

struct InstallToolSourceTool;

impl Tool for InstallToolSourceTool {
    fn name(&self) -> &str {
        "install_tool_source"
    }
    fn description(&self) -> &str {
        "Install an MCP or tool source package."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "url": { "type": "string" } },
            "required": ["url"]
        })
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        Ok(format!(
            "install queued for {}",
            args["url"].as_str().unwrap_or("")
        ))
    }
}

struct LspHoverTool;

impl Tool for LspHoverTool {
    fn name(&self) -> &str {
        "lsp_hover"
    }
    fn description(&self) -> &str {
        "LSP hover information for a symbol. Requires LSP enabled in Settings."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "line": { "type": "integer" },
                "character": { "type": "integer" }
            },
            "required": ["path", "line", "character"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        crate::core::lsp::shared_lsp_manager().hover(
            &ctx.workspace_root,
            args["path"].as_str().unwrap_or(""),
            args["line"].as_u64().unwrap_or(0),
            args["character"].as_u64().unwrap_or(0),
        )
    }
}

struct LspDefinitionTool;

impl Tool for LspDefinitionTool {
    fn name(&self) -> &str {
        "lsp_definition"
    }
    fn description(&self) -> &str {
        "LSP go-to-definition. Requires LSP enabled in Settings."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "line": { "type": "integer" },
                "character": { "type": "integer" }
            },
            "required": ["path", "line", "character"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        crate::core::lsp::shared_lsp_manager().definition(
            &ctx.workspace_root,
            args["path"].as_str().unwrap_or(""),
            args["line"].as_u64().unwrap_or(0),
            args["character"].as_u64().unwrap_or(0),
        )
    }
}

struct LspDiagnosticsTool;

impl Tool for LspDiagnosticsTool {
    fn name(&self) -> &str {
        "lsp_diagnostics"
    }
    fn description(&self) -> &str {
        "Pull LSP diagnostics for a file. Requires LSP enabled in Settings."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        crate::core::lsp::shared_lsp_manager()
            .diagnostics(&ctx.workspace_root, args["path"].as_str().unwrap_or(""))
    }
}
