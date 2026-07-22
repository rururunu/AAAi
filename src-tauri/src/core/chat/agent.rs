use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::future::join_all;
use tokio::sync::mpsc;

use crate::core::ai::provider::{AIProvider, ProviderError};
use crate::core::chat::limits::{
    estimate_tokens, truncate_tool_output, DEFAULT_MAX_STEPS, DEFAULT_MAX_TURN_TOKENS,
    TOOL_OUTPUT_MAX_CHARS,
};
use crate::core::runtime::{
    ChatMessage, ChatRequest, MessageStatus, Role, StreamEvent, ToolActivity, ToolCallPayload,
};
use crate::core::tools::context::ToolContext;
use crate::core::tools::display::build_activity_view;
use crate::core::tools::error::ToolError;
use crate::core::tools::registry::ToolRegistry;
use crate::runtime::ToolManager;

pub struct AgentRunner {
    provider: Arc<dyn AIProvider>,
    tools: Arc<ToolManager>,
    max_steps: u32,
    max_turn_tokens: usize,
    tool_output_max_chars: usize,
}

impl AgentRunner {
    pub fn new(provider: Arc<dyn AIProvider>, tools: Arc<ToolManager>) -> Self {
        Self {
            provider,
            tools,
            max_steps: DEFAULT_MAX_STEPS,
            max_turn_tokens: DEFAULT_MAX_TURN_TOKENS,
            tool_output_max_chars: TOOL_OUTPUT_MAX_CHARS,
        }
    }

    pub fn with_max_turn_tokens(mut self, max_turn_tokens: usize) -> Self {
        self.max_turn_tokens = max_turn_tokens;
        self
    }

    #[cfg(test)]
    pub fn with_limits(
        provider: Arc<dyn AIProvider>,
        tools: Arc<ToolManager>,
        max_steps: u32,
        max_turn_tokens: usize,
        tool_output_max_chars: usize,
    ) -> Self {
        Self {
            provider,
            tools,
            max_steps,
            max_turn_tokens,
            tool_output_max_chars,
        }
    }

    pub async fn run(
        &self,
        mut request: ChatRequest,
        tool_ctx: ToolContext,
        tx: mpsc::Sender<StreamEvent>,
        cancelled: Arc<AtomicBool>,
        soft_queue: Arc<Mutex<VecDeque<String>>>,
    ) -> Result<(), ProviderError> {
        request.tools = self.tools.schemas_arc();
        let mut steps = 0u32;
        let mut user_msg_index = request.messages.iter().rposition(|msg| msg.role == Role::User);
        let mut used_tokens = estimate_request_tokens(&request);

        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(ProviderError::cancelled());
            }
            drain_soft_injects(&soft_queue, &mut request, &tx, &mut user_msg_index).await;
            if self.max_steps > 0 && steps >= self.max_steps {
                let _ = tx
                    .send(StreamEvent::TurnComplete {
                        content: format!(
                            "已停止：本轮达到最大工具步数上限（{}）。",
                            self.max_steps
                        ),
                        reasoning: None,
                        tool_calls: vec![],
                        finish_reason: Some("max_steps".to_string()),
                    })
                    .await;
                break;
            }
            if self.max_turn_tokens > 0 && used_tokens >= self.max_turn_tokens {
                let mut compacted = false;
                if let Some(user_idx) = user_msg_index {
                    if user_idx > 0 {
                        let prior = &request.messages[..user_idx];
                        let current_turn = request.messages[user_idx..].to_vec();
                        let summarizer = crate::core::chat::compact::ProviderSummarizer::new(Arc::clone(&self.provider));
                        if let Some(outcome) = crate::core::chat::compact::compact_prior(prior, &request.session_id, Some(&summarizer)).await {
                            let mut new_messages = outcome.messages;
                            let new_user_idx = new_messages.len();
                            new_messages.extend(current_turn);
                            request.messages = new_messages;
                            user_msg_index = Some(new_user_idx);
                            used_tokens = estimate_request_tokens(&request);
                            compacted = true;
                        }
                    }
                }

                if !compacted {
                    let _ = tx
                        .send(StreamEvent::TurnComplete {
                            content: format!(
                                "已停止：本轮估算 token 用量达到上限（{}）。",
                                self.max_turn_tokens
                            ),
                            reasoning: None,
                            tool_calls: vec![],
                            finish_reason: Some("max_turn_tokens".to_string()),
                        })
                        .await;
                    break;
                }
            }

            let (turn_tx, mut turn_rx) = mpsc::channel::<StreamEvent>(64);
            let provider = Arc::clone(&self.provider);
            let turn_request = request.clone();
            let provider_task =
                tauri::async_runtime::spawn(
                    async move { provider.stream(turn_request, turn_tx).await },
                );

            let mut content = String::new();
            let mut reasoning = String::new();
            let mut tool_calls = Vec::new();
            let mut finish_reason = None;

            while let Some(event) = turn_rx.recv().await {
                if cancelled.load(Ordering::Relaxed) {
                    return Err(ProviderError::cancelled());
                }
                match event {
                    StreamEvent::Start => {}
                    StreamEvent::Delta(delta) => {
                        content.push_str(&delta);
                        let _ = tx.send(StreamEvent::Delta(delta)).await;
                    }
                    StreamEvent::Reasoning(chunk) => {
                        reasoning.push_str(&chunk);
                        let _ = tx.send(StreamEvent::Reasoning(chunk)).await;
                    }
                    StreamEvent::Status { kind } => {
                        let _ = tx.send(StreamEvent::Status { kind }).await;
                    }
                    StreamEvent::UserContentPatch { message_id, content } => {
                        let _ = tx
                            .send(StreamEvent::UserContentPatch { message_id, content })
                            .await;
                    }
                    StreamEvent::ToolCall(call) => {
                        merge_tool_call(&mut tool_calls, call);
                    }
                    StreamEvent::TurnComplete {
                        content: turn_content,
                        reasoning: turn_reasoning,
                        tool_calls: turn_tool_calls,
                        finish_reason: turn_finish,
                    } => {
                        content = turn_content;
                        if let Some(value) = turn_reasoning {
                            reasoning = value;
                        }
                        tool_calls = turn_tool_calls;
                        finish_reason = turn_finish;
                    }
                    StreamEvent::Finish => break,
                    StreamEvent::Error(message) => {
                        let _ = tx.send(StreamEvent::Error(message.clone())).await;
                        return Err(ProviderError::message(message));
                    }
                }
            }

            provider_task
                .await
                .map_err(|error| ProviderError::message(format!("provider task failed: {error}")))??;

            used_tokens += estimate_tokens(&content) + estimate_tokens(&reasoning);

            if tool_calls.is_empty() {
                let _ = tx
                    .send(StreamEvent::TurnComplete {
                        content,
                        reasoning: non_empty(reasoning),
                        tool_calls: vec![],
                        finish_reason,
                    })
                    .await;
                break;
            }

            let _ = tx
                .send(StreamEvent::Status {
                    kind: format!("tools:{}", tool_calls.len()),
                })
                .await;

            let assistant = ChatMessage {
                id: format!("msg-{}", now_millis()),
                session_id: request.session_id.clone(),
                role: Role::Assistant,
                content: content.clone(),
                reasoning: non_empty(reasoning.clone()),
                tool_activities: None,
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
                name: None,
                status: MessageStatus::Done,
                timestamp: now_millis(),
            };
            request.messages.push(assistant);

            let parallel = tool_calls.len() > 1
                && tool_calls
                    .iter()
                    .all(|call| self.tools.is_read_only(&call.name));

            let outcomes = if parallel {
                self.execute_tools_parallel(&tool_calls, &tool_ctx, &cancelled)
                    .await?
            } else {
                self.execute_tools_serial(&tool_calls, &tool_ctx, &cancelled)
                    .await?
            };

            let mut user_denied = false;
            for outcome in outcomes {
                used_tokens += estimate_tokens(&outcome.result);
                request.messages.push(ChatMessage {
                    id: format!("msg-{}", now_millis()),
                    session_id: request.session_id.clone(),
                    role: Role::Tool,
                    content: outcome.result,
                    reasoning: None,
                    tool_activities: None,
                    tool_calls: None,
                    tool_call_id: Some(outcome.call_id),
                    name: Some(outcome.tool_name),
                    status: MessageStatus::Done,
                    timestamp: now_millis(),
                });
                if outcome.user_denied {
                    user_denied = true;
                }
            }

            if user_denied {
                let _ = tx
                    .send(StreamEvent::TurnComplete {
                        content: "已停止：你拒绝了文件访问权限。".to_string(),
                        reasoning: None,
                        tool_calls: vec![],
                        finish_reason: Some("user_denied".to_string()),
                    })
                    .await;
                return Ok(());
            }

            // Soft-inject at tool boundary before the next provider call.
            drain_soft_injects(&soft_queue, &mut request, &tx, &mut user_msg_index).await;
            steps += 1;
        }

        Ok(())
    }

    async fn execute_tools_serial(
        &self,
        tool_calls: &[ToolCallPayload],
        tool_ctx: &ToolContext,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<Vec<ToolOutcome>, ProviderError> {
        let mut outcomes = Vec::with_capacity(tool_calls.len());
        for call in tool_calls {
            if cancelled.load(Ordering::Relaxed) {
                return Err(ProviderError::cancelled());
            }
            outcomes.push(self.execute_one_tool(call, tool_ctx).await);
        }
        Ok(outcomes)
    }

    async fn execute_tools_parallel(
        &self,
        tool_calls: &[ToolCallPayload],
        tool_ctx: &ToolContext,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<Vec<ToolOutcome>, ProviderError> {
        if cancelled.load(Ordering::Relaxed) {
            return Err(ProviderError::cancelled());
        }

        // Emit "running" UI events on the agent task before dispatching workers.
        let prepared: Vec<_> = tool_calls
            .iter()
            .map(|call| self.begin_tool_activity(call, tool_ctx))
            .collect();

        let jobs = prepared.into_iter().map(|started| {
            let tools = Arc::clone(&self.tools);
            let execution_context = tool_ctx.clone();
            let tool_name = started.tool_name.clone();
            let tool_args = started.args.clone();
            let max_chars = self.tool_output_max_chars;
            async move {
                let execution = tools
                    .dispatch_async(&execution_context, &tool_name, tool_args)
                    .await;
                (started, execution, max_chars)
            }
        });

        let finished = join_all(jobs).await;
        let mut outcomes = Vec::with_capacity(finished.len());
        for (started, execution, max_chars) in finished {
            outcomes.push(self.finish_tool_activity(started, execution, tool_ctx, max_chars));
        }
        Ok(outcomes)
    }

    async fn execute_one_tool(&self, call: &ToolCallPayload, tool_ctx: &ToolContext) -> ToolOutcome {
        let started = self.begin_tool_activity(call, tool_ctx);
        let tools = Arc::clone(&self.tools);
        let execution_context = tool_ctx.clone();
        let tool_name = started.tool_name.clone();
        let tool_args = started.args.clone();
        let execution = tools
            .dispatch_async(&execution_context, &tool_name, tool_args)
            .await;
        self.finish_tool_activity(started, execution, tool_ctx, self.tool_output_max_chars)
    }

    fn begin_tool_activity(&self, call: &ToolCallPayload, tool_ctx: &ToolContext) -> StartedTool {
        let args: serde_json::Value =
            serde_json::from_str(&call.arguments).unwrap_or_else(|_| serde_json::json!({}));
        let activity_id = format!("tool-{}-{}", call.id, now_millis());
        let preview = build_activity_view(&call.name, &args, None);
        let preview_detail = preview.detail.clone();
        let display_sid = display_session_id(&tool_ctx.session_id);
        tool_ctx.conversation.upsert_tool_activity(
            &display_sid,
            &tool_ctx.assistant_message_id,
            ToolActivity {
                id: activity_id.clone(),
                tool_name: call.name.clone(),
                title: preview.title.clone(),
                kind: preview.kind.clone(),
                detail: preview.detail.clone(),
                arguments: Some(args.clone()),
                result: None,
                success: true,
                status: "running".to_string(),
            },
        );
        tool_ctx
            .event_bus
            .emit(crate::core::event::BusEvent::ToolStarted {
                session_id: display_sid,
                message_id: tool_ctx.assistant_message_id.clone(),
                activity_id: activity_id.clone(),
                tool_name: call.name.clone(),
                title: preview.title.clone(),
                kind: preview.kind.clone(),
                detail: preview.detail,
                arguments: args.clone(),
            });
        StartedTool {
            call_id: call.id.clone(),
            tool_name: call.name.clone(),
            activity_id,
            args,
            preview_detail,
        }
    }

    fn finish_tool_activity(
        &self,
        started: StartedTool,
        execution: Result<String, ToolError>,
        tool_ctx: &ToolContext,
        max_chars: usize,
    ) -> ToolOutcome {
        let user_denied = execution.as_ref().err().is_some_and(ToolError::is_terminal);
        let (raw_result, success) = match execution {
            Ok(value) => (value, true),
            Err(error) => (format!("tool error: {error}"), false),
        };
        let result = truncate_tool_output(&raw_result, max_chars);
        let finished = build_activity_view(&started.tool_name, &started.args, Some(&result));
        let detail = finished.detail.or(started.preview_detail);
        let display_sid = display_session_id(&tool_ctx.session_id);
        tool_ctx.conversation.upsert_tool_activity(
            &display_sid,
            &tool_ctx.assistant_message_id,
            ToolActivity {
                id: started.activity_id.clone(),
                tool_name: started.tool_name.clone(),
                title: finished.title.clone(),
                kind: finished.kind.clone(),
                detail: detail.clone(),
                arguments: Some(started.args.clone()),
                result: Some(result.clone()),
                success,
                status: if success { "done" } else { "error" }.to_string(),
            },
        );
        tool_ctx
            .event_bus
            .emit(crate::core::event::BusEvent::ToolFinished {
                session_id: display_sid,
                message_id: tool_ctx.assistant_message_id.clone(),
                activity_id: started.activity_id,
                tool_name: started.tool_name.clone(),
                title: finished.title,
                kind: finished.kind,
                detail,
                arguments: started.args,
                result: result.clone(),
                success,
            });
        ToolOutcome {
            call_id: started.call_id,
            tool_name: started.tool_name,
            result,
            user_denied,
        }
    }

    pub async fn run_subagent(
        provider: Arc<dyn AIProvider>,
        registry: Arc<ToolRegistry>,
        tool_ctx: ToolContext,
        prompt: String,
        read_only: bool,
    ) -> Result<String, ToolError> {
        let active_tools = if read_only {
            Arc::new(ToolManager::new(registry.filter_read_only()))
        } else {
            Arc::new(ToolManager::from_registry(registry))
        };
        let runner = AgentRunner::new(provider, active_tools);
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
        let cancelled = Arc::new(AtomicBool::new(false));
        let soft_queue = Arc::new(Mutex::new(VecDeque::new()));
        let request = ChatRequest {
            request_id: format!("sub-{}", now_millis()),
            session_id: tool_ctx.session_id.clone(),
            messages: vec![ChatMessage {
                id: format!("msg-{}", now_millis()),
                session_id: tool_ctx.session_id.clone(),
                role: Role::User,
                content: prompt,
                reasoning: None,
                tool_activities: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                status: MessageStatus::Done,
                timestamp: now_millis(),
            }],
            context: Default::default(),
            provider: None,
            stream: true,
            tools: std::sync::Arc::from([]),
            temperature: None,
            max_tokens: None,
        };

        // Spawn a background task to receive from rx concurrently to avoid channel deadlocks.
        let answer = Arc::new(tokio::sync::Mutex::new(String::new()));
        let answer_clone = Arc::clone(&answer);
        let rx_task = tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::TurnComplete { content, .. } => {
                        let mut lock = answer_clone.lock().await;
                        *lock = content;
                    }
                    StreamEvent::Delta(delta) => {
                        let mut lock = answer_clone.lock().await;
                        lock.push_str(&delta);
                    }
                    _ => {}
                }
            }
        });

        Box::pin(runner.run(request, tool_ctx, tx, cancelled, soft_queue))
            .await
            .map_err(|error| ToolError::new(error.to_string()))?;

        // Wait for the receiver task to finish draining
        let _ = rx_task.await;

        let final_answer = answer.lock().await.clone();
        Ok(final_answer)
    }
}

struct StartedTool {
    call_id: String,
    tool_name: String,
    activity_id: String,
    args: serde_json::Value,
    preview_detail: Option<String>,
}

struct ToolOutcome {
    call_id: String,
    tool_name: String,
    result: String,
    user_denied: bool,
}

fn estimate_request_tokens(request: &ChatRequest) -> usize {
    let message_tokens: usize = request
        .messages
        .iter()
        .map(|message| {
            estimate_tokens(&message.content)
                + estimate_tokens(message.reasoning.as_deref().unwrap_or(""))
                + 4
        })
        .sum();
    let tool_tokens: usize = request
        .tools
        .iter()
        .map(|tool| estimate_tokens(&tool.to_string()))
        .sum();
    message_tokens + tool_tokens
}

fn merge_tool_call(calls: &mut Vec<ToolCallPayload>, incoming: ToolCallPayload) {
    if !incoming.id.is_empty() {
        if let Some(existing) = calls.iter_mut().find(|call| call.id == incoming.id) {
            if !incoming.name.is_empty() {
                existing.name = incoming.name;
            }
            existing.arguments.push_str(&incoming.arguments);
            return;
        }
    }
    calls.push(incoming);
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

async fn drain_soft_injects(
    soft_queue: &Arc<Mutex<VecDeque<String>>>,
    request: &mut ChatRequest,
    tx: &mpsc::Sender<StreamEvent>,
    user_msg_index: &mut Option<usize>,
) {
    let injected: Vec<String> = {
        let Ok(mut queue) = soft_queue.lock() else {
            return;
        };
        queue.drain(..).collect()
    };
    if injected.is_empty() {
        return;
    }

    for content in injected {
        let message = ChatMessage {
            id: format!("msg-{}", now_millis()),
            session_id: request.session_id.clone(),
            role: Role::User,
            content: format!(
                "[Follow-up instruction while you were working]\n{content}"
            ),
            reasoning: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: now_millis(),
        };
        if user_msg_index.is_none() {
            *user_msg_index = Some(request.messages.len());
        }
        request.messages.push(message);
    }

    let _ = tx
        .send(StreamEvent::Status {
            kind: "soft_injected".to_string(),
        })
        .await;
}

fn display_session_id(session_id: &str) -> String {
    if let Some(pos) = session_id.find("-sub") {
        session_id[..pos].to_string()
    } else {
        session_id.to_string()
    }
}
