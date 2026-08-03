use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::future::join_all;
use tokio::sync::mpsc;

use crate::core::ai::provider::{AIProvider, ProviderError};
use crate::core::chat::limits::{
    estimate_tokens, truncate_tool_output, DEFAULT_MAX_STEPS, DEFAULT_MAX_TURN_TOKENS,
    MAX_CONSECUTIVE_TOOL_FAILURES, TOOL_OUTPUT_MAX_CHARS,
};
use crate::core::runtime::{
    ChatMessage, ChatRequest, MessageStatus, Role, StreamEvent, ToolActivity, ToolCallPayload,
};
use crate::core::tools::context::ToolContext;
use crate::core::tools::display::build_activity_view;
use crate::core::tools::error::ToolError;
use crate::core::tools::registry::ToolRegistry;
use crate::runtime::ToolManager;

/// How many times an unverified completion claim is challenged and sent back
/// before the final answer is replaced with an explicit unverified result.
const MAX_COMPLETION_RETRIES: u32 = 1;

/// Injected after the model's final answer claims completion without any
/// successful modifying tool. The model must either actually execute the work
/// or explicitly admit nothing was changed — claiming done is not accepted.
const COMPLETION_CHALLENGE: &str = concat!(
    "[System] Completion claim rejected: no modifying tool succeeded this turn. ",
    "Do not restate prior reasoning. Either call the required tools now, ",
    "or clearly say what was not changed and what is blocking.",
);

const VERIFICATION_CHALLENGE: &str = concat!(
    "[System] Changes ran but were not verified. ",
    "Do not restate prior reasoning. Call a read/test/build check now, ",
    "then report the verified result or the failure.",
);

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
        request.tools = self.tools.schemas_for_request(&request, tool_ctx.root_session_id());
        let mut steps = 0u32;
        let mut consecutive_tool_failures = 0u32;
        let mut repeated_tool_errors: HashMap<String, String> = HashMap::new();
        let mut mutation_succeeded = false;
        let mut verification_succeeded = false;
        let mut user_msg_index = request
            .messages
            .iter()
            .rposition(|msg| msg.role == Role::User);
        let mut used_tokens = estimate_request_tokens(&request);
        let mut empty_completion_retries = 0u32;
        let mut verification_retries = 0u32;

        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(ProviderError::cancelled());
            }
            drain_soft_injects(&soft_queue, &mut request, &tx, &mut user_msg_index).await;
            if self.max_steps > 0 && steps >= self.max_steps {
                let _ = tx
                    .send(StreamEvent::TurnComplete {
                        content: format!(
                            "已停止：本轮达到最大工具步数上限（{}）。可发送「继续」让我接着做未完成的部分。",
                            self.max_steps
                        ),
                        reasoning: None,
                        tool_calls: vec![],
                        finish_reason: Some("max_steps".to_string()),
                    })
                    .await;
                break;
            }
            // Codex-style: near the context window, auto-compact and continue.
            // Never hard-stop the turn solely because tokens crossed a budget.
            let compact_at = mid_turn_compact_threshold(self.max_turn_tokens);
            if compact_at > 0 && used_tokens >= compact_at {
                if let Some(user_idx) = user_msg_index {
                    if user_idx > 0 {
                        let prior = &request.messages[..user_idx];
                        let current_turn = request.messages[user_idx..].to_vec();
                        let summarizer = crate::core::chat::compact::ProviderSummarizer::new(
                            Arc::clone(&self.provider),
                        );
                        if let Some(outcome) = crate::core::chat::compact::compact_prior(
                            prior,
                            &request.session_id,
                            Some(&summarizer),
                        )
                        .await
                        {
                            let mut new_messages = outcome.messages;
                            let new_user_idx = new_messages.len();
                            new_messages.extend(current_turn);
                            request.messages = new_messages;
                            user_msg_index = Some(new_user_idx);
                            used_tokens = estimate_request_tokens(&request);
                            let _ = tx
                                .send(StreamEvent::Status {
                                    kind: "context_compacted".to_string(),
                                })
                                .await;
                        }
                    }
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
                    StreamEvent::UserContentPatch {
                        message_id,
                        content,
                    } => {
                        let _ = tx
                            .send(StreamEvent::UserContentPatch {
                                message_id,
                                content,
                            })
                            .await;
                    }
                    StreamEvent::ToolCall(call) => {
                        merge_tool_call(&mut tool_calls, call);
                    }
                    StreamEvent::Usage(usage) => {
                        let _ = tx.send(StreamEvent::Usage(usage)).await;
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

            provider_task.await.map_err(|error| {
                ProviderError::message(format!("provider task failed: {error}"))
            })??;

            used_tokens += estimate_tokens(&content) + estimate_tokens(&reasoning);

            if tool_calls.is_empty() {
                // Honest-completion enforcement: a task-like final answer that
                // claims completion without any successful modifying tool is
                // challenged and sent back so the model either actually executes
                // the work or explicitly admits nothing was changed.
                if !mutation_succeeded
                    && !crate::runtime::tool::is_question_only_request(&request)
                    && has_completion_claim(&content)
                    && empty_completion_retries < MAX_COMPLETION_RETRIES
                {
                    empty_completion_retries += 1;

                    let assistant = ChatMessage {
                        id: format!("msg-{}", now_millis()),
                        session_id: request.session_id.clone(),
                        role: Role::Assistant,
                        content: content.clone(),
                        reasoning: non_empty(reasoning.clone()),
                        tool_activities: None,
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                        status: MessageStatus::Done,
                        timestamp: now_millis(),
                        estimated_tokens: None,
                    };
                    request.messages.push(assistant);

                    let user_feedback = ChatMessage {
                        id: format!("msg-{}", now_millis()),
                        session_id: request.session_id.clone(),
                        role: Role::User,
                        content: COMPLETION_CHALLENGE.to_string(),
                        reasoning: None,
                        tool_activities: None,
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                        status: MessageStatus::Done,
                        timestamp: now_millis(),
                        estimated_tokens: None,
                    };
                    if user_msg_index.is_none() {
                        user_msg_index = Some(request.messages.len());
                    }
                    request.messages.push(user_feedback);

                    let _ = tx
                        .send(StreamEvent::Status {
                            kind: "reject_empty_completion".to_string(),
                        })
                        .await;

                    steps += 1;
                    continue;
                }

                if mutation_succeeded
                    && !verification_succeeded
                    && has_completion_claim(&content)
                    && verification_retries < MAX_COMPLETION_RETRIES
                {
                    verification_retries += 1;
                    push_completion_feedback(
                        &mut request,
                        &mut user_msg_index,
                        content.clone(),
                        reasoning.clone(),
                        VERIFICATION_CHALLENGE,
                    );
                    let _ = tx
                        .send(StreamEvent::Status {
                            kind: "verify_completion".to_string(),
                        })
                        .await;
                    steps += 1;
                    continue;
                }

                let completion_rejected = reject_unverified_completion(
                    &mut content,
                    &request,
                    mutation_succeeded,
                    verification_succeeded,
                );
                let _ = tx
                    .send(StreamEvent::TurnComplete {
                        content,
                        reasoning: non_empty(reasoning),
                        tool_calls: vec![],
                        finish_reason: if completion_rejected {
                            Some("unverified_completion".to_string())
                        } else {
                            finish_reason
                        },
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
                estimated_tokens: None,
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
            for outcome in &outcomes {
                if outcome.success {
                    if mutation_succeeded && provides_verification_evidence(&self.tools, outcome) {
                        verification_succeeded = true;
                    } else if provides_completion_evidence(&self.tools, outcome) {
                        mutation_succeeded = true;
                        verification_succeeded = false;
                    }
                }
                used_tokens += estimate_tokens(&outcome.result);
                request.messages.push(ChatMessage {
                    id: format!("msg-{}", now_millis()),
                    session_id: request.session_id.clone(),
                    role: Role::Tool,
                    content: outcome.result.clone(),
                    reasoning: None,
                    tool_activities: None,
                    tool_calls: None,
                    tool_call_id: Some(outcome.call_id.clone()),
                    name: Some(outcome.tool_name.clone()),
                    status: MessageStatus::Done,
                    timestamp: now_millis(),
                    estimated_tokens: None,
                });
                if outcome.user_denied {
                    user_denied = true;
                }
            }

            // 失败熔断与同错误防重复：连续失败超过阈值，或同一工具以相同参数
            // 反复返回同一错误，立即停止本轮，避免无效循环。
            let mut stop_reason = None;
            for outcome in &outcomes {
                if outcome.user_denied {
                    continue;
                }
                if !outcome.success {
                    consecutive_tool_failures += 1;
                    if consecutive_tool_failures >= MAX_CONSECUTIVE_TOOL_FAILURES {
                        stop_reason = Some(format!(
                            "工具连续失败 {} 次，已触发熔断",
                            MAX_CONSECUTIVE_TOOL_FAILURES
                        ));
                        break;
                    }
                    let key = format!("{}|{}", outcome.tool_name, outcome.arguments);
                    match repeated_tool_errors.get(&key) {
                        Some(previous) if previous == &outcome.result => {
                            stop_reason = Some(format!(
                                "工具 `{}` 以相同参数反复返回同一错误，已停止重试",
                                outcome.tool_name
                            ));
                            break;
                        }
                        _ => {
                            repeated_tool_errors.insert(key, outcome.result.clone());
                        }
                    }
                } else {
                    consecutive_tool_failures = 0;
                }
            }

            if let Some(reason) = stop_reason {
                let _ = tx
                    .send(StreamEvent::TurnComplete {
                        content: format!("已停止：{reason}。"),
                        reasoning: None,
                        tool_calls: vec![],
                        finish_reason: Some("tool_failure_breaker".to_string()),
                    })
                    .await;
                return Ok(());
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
            outcomes.push(self.execute_one_tool(call, tool_ctx).await?);
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
            let mut execution_context = tool_ctx.clone();
            execution_context.parent_activity_id = Some(started.activity_id.clone());
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
        if cancelled.load(Ordering::Relaxed) {
            return Err(ProviderError::cancelled());
        }
        let mut outcomes = Vec::with_capacity(finished.len());
        for (started, execution, max_chars) in finished {
            outcomes.push(self.finish_tool_activity(started, execution, tool_ctx, max_chars));
        }
        Ok(outcomes)
    }

    async fn execute_one_tool(
        &self,
        call: &ToolCallPayload,
        tool_ctx: &ToolContext,
    ) -> Result<ToolOutcome, ProviderError> {
        let started = self.begin_tool_activity(call, tool_ctx);
        let tools = Arc::clone(&self.tools);
        let mut execution_context = tool_ctx.clone();
        execution_context.parent_activity_id = Some(started.activity_id.clone());
        let tool_name = started.tool_name.clone();
        let tool_args = started.args.clone();
        let execution = tools
            .dispatch_async(&execution_context, &tool_name, tool_args)
            .await;
        if tool_ctx.is_cancelled()
            || execution
                .as_ref()
                .err()
                .is_some_and(ToolError::is_cancelled)
        {
            return Err(ProviderError::cancelled());
        }
        Ok(self.finish_tool_activity(started, execution, tool_ctx, self.tool_output_max_chars))
    }

    fn begin_tool_activity(&self, call: &ToolCallPayload, tool_ctx: &ToolContext) -> StartedTool {
        let args: serde_json::Value =
            serde_json::from_str(&call.arguments).unwrap_or_else(|_| serde_json::json!({}));
        let activity_id = format!("tool-{}-{}", call.id, now_millis());
        let activity_view = build_activity_view(&call.name, &args, None);
        let preview_detail = activity_view.detail.clone();
        let tool_preview = self.tools.preview(tool_ctx, &call.name, &args);
        let display_sid = tool_ctx.root_session_id().to_string();
        tool_ctx.conversation.upsert_tool_activity(
            &display_sid,
            &tool_ctx.assistant_message_id,
            ToolActivity {
                id: activity_id.clone(),
                subagent_id: tool_ctx.subagent_id.clone(),
                parent_activity_id: tool_ctx.parent_activity_id.clone(),
                tool_name: call.name.clone(),
                title: activity_view.title.clone(),
                kind: activity_view.kind.clone(),
                detail: activity_view.detail.clone(),
                arguments: Some(args.clone()),
                result: None,
                preview: tool_preview.clone(),
                success: true,
                status: "running".to_string(),
            },
        );
        tool_ctx
            .event_bus
            .emit(crate::core::event::BusEvent::ToolStarted {
                session_id: display_sid,
                subagent_id: tool_ctx.subagent_id.clone(),
                parent_activity_id: tool_ctx.parent_activity_id.clone(),
                message_id: tool_ctx.assistant_message_id.clone(),
                activity_id: activity_id.clone(),
                tool_name: call.name.clone(),
                title: activity_view.title.clone(),
                kind: activity_view.kind.clone(),
                detail: activity_view.detail,
                arguments: args.clone(),
                preview: tool_preview.clone(),
            });
        StartedTool {
            call_id: call.id.clone(),
            tool_name: call.name.clone(),
            activity_id,
            args,
            preview_detail,
            tool_preview,
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
        let display_sid = tool_ctx.root_session_id().to_string();
        tool_ctx.conversation.upsert_tool_activity(
            &display_sid,
            &tool_ctx.assistant_message_id,
            ToolActivity {
                id: started.activity_id.clone(),
                subagent_id: tool_ctx.subagent_id.clone(),
                parent_activity_id: tool_ctx.parent_activity_id.clone(),
                tool_name: started.tool_name.clone(),
                title: finished.title.clone(),
                kind: finished.kind.clone(),
                detail: detail.clone(),
                arguments: Some(started.args.clone()),
                result: Some(result.clone()),
                preview: started.tool_preview.clone(),
                success,
                status: if success { "done" } else { "error" }.to_string(),
            },
        );
        tool_ctx
            .event_bus
            .emit(crate::core::event::BusEvent::ToolFinished {
                session_id: display_sid,
                subagent_id: tool_ctx.subagent_id.clone(),
                parent_activity_id: tool_ctx.parent_activity_id.clone(),
                message_id: tool_ctx.assistant_message_id.clone(),
                activity_id: started.activity_id,
                tool_name: started.tool_name.clone(),
                title: finished.title,
                kind: finished.kind,
                detail,
                arguments: started.args.clone(),
                preview: started.tool_preview,
                result: result.clone(),
                success,
            });
        ToolOutcome {
            call_id: started.call_id,
            tool_name: started.tool_name,
            arguments: serde_json::to_string(&started.args).unwrap_or_default(),
            result,
            success,
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
        let active_tools = Arc::new(ToolManager::new(registry.filter_for_subagent(read_only)));
        let runner = AgentRunner::new(provider, active_tools);
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
        let cancelled = Arc::clone(&tool_ctx.cancelled);
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
                estimated_tokens: None,
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
        let progress_bus = Arc::clone(&tool_ctx.event_bus);
        let progress_subagent_id = tool_ctx.subagent_id.clone();
        let rx_task = tauri::async_runtime::spawn(async move {
            let mut response_reported = false;
            let mut reasoning_reported = false;
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::TurnComplete { content, .. } => {
                        let mut lock = answer_clone.lock().await;
                        *lock = content;
                    }
                    StreamEvent::Delta(delta) => {
                        if !response_reported {
                            if let Some(subagent_id) = &progress_subagent_id {
                                progress_bus.emit(crate::core::event::BusEvent::SubagentProgress {
                                    subagent_id: subagent_id.clone(),
                                    kind: "responding".to_string(),
                                    content: "Generating response".to_string(),
                                    timestamp_ms: now_millis(),
                                });
                            }
                            response_reported = true;
                        }
                        let mut lock = answer_clone.lock().await;
                        lock.push_str(&delta);
                    }
                    StreamEvent::Reasoning(_) => {
                        if !reasoning_reported {
                            if let Some(subagent_id) = &progress_subagent_id {
                                progress_bus.emit(crate::core::event::BusEvent::SubagentProgress {
                                    subagent_id: subagent_id.clone(),
                                    kind: "reasoning".to_string(),
                                    content: "Reasoning".to_string(),
                                    timestamp_ms: now_millis(),
                                });
                            }
                            reasoning_reported = true;
                        }
                    }
                    StreamEvent::Usage(usage) => {
                        progress_bus.emit(crate::core::event::BusEvent::TokenUsage {
                            model: "subagent".to_string(),
                            usage,
                        });
                    }
                    StreamEvent::Status { kind } => {
                        if let Some(subagent_id) = &progress_subagent_id {
                            progress_bus.emit(crate::core::event::BusEvent::SubagentProgress {
                                subagent_id: subagent_id.clone(),
                                kind: "status".to_string(),
                                content: kind,
                                timestamp_ms: now_millis(),
                            });
                        }
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
    tool_preview: Option<crate::core::tools::preview::ToolPreview>,
}

struct ToolOutcome {
    call_id: String,
    tool_name: String,
    /// Serialized arguments, used to detect repeated identical calls.
    arguments: String,
    result: String,
    success: bool,
    user_denied: bool,
}

/// Token count at which mid-turn auto-compact is attempted (Codex-style ~80–90%).
fn mid_turn_compact_threshold(context_window: usize) -> usize {
    if context_window == 0 {
        return 0;
    }
    ((context_window as f32) * crate::core::chat::compact::COMPACT_TRIGGER_RATIO).ceil() as usize
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

fn push_completion_feedback(
    request: &mut ChatRequest,
    user_msg_index: &mut Option<usize>,
    content: String,
    reasoning: String,
    feedback: &str,
) {
    request.messages.push(ChatMessage {
        id: format!("msg-{}", now_millis()),
        session_id: request.session_id.clone(),
        role: Role::Assistant,
        content,
        reasoning: non_empty(reasoning),
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: now_millis(),
        estimated_tokens: None,
    });
    if user_msg_index.is_none() {
        *user_msg_index = Some(request.messages.len());
    }
    request.messages.push(ChatMessage {
        id: format!("msg-{}", now_millis()),
        session_id: request.session_id.clone(),
        role: Role::User,
        content: feedback.to_string(),
        reasoning: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: now_millis(),
        estimated_tokens: None,
    });
}

/// Successful non-read-only tools normally prove that work happened, but
/// orchestration-only tools must not let a model turn task bookkeeping into
/// evidence that the requested change was made.
fn provides_completion_evidence(tools: &ToolManager, outcome: &ToolOutcome) -> bool {
    if tools.is_read_only(&outcome.tool_name) {
        return false;
    }
    !matches!(
        outcome.tool_name.as_str(),
        "update_tasks" | "ask_user" | "complete_plan_step" | "connect_tools"
    )
}

fn provides_verification_evidence(tools: &ToolManager, outcome: &ToolOutcome) -> bool {
    if tools.is_read_only(&outcome.tool_name) {
        return !matches!(
            outcome.tool_name.as_str(),
            "search_memory" | "list_chats" | "read_chat" | "search_past_chats"
        );
    }
    if outcome.tool_name != "run_shell" {
        return false;
    }
    let command = outcome.arguments.to_ascii_lowercase();
    const CHECK_MARKERS: &[&str] = &[
        " test", "test ", "cargo test", "pytest", "unittest", "pnpm build", "npm run build",
        "npm test", "cargo check", "tsc", "vue-tsc", "lint", "check", "verify",
    ];
    CHECK_MARKERS.iter().any(|marker| command.contains(marker))
}

/// A change request cannot finish with a completion claim unless a modifying
/// tool succeeded in this turn. Replace the claim instead of displaying it with
/// a caveat, because the original text is still misleading and looks complete.
fn reject_unverified_completion(
    content: &mut String,
    request: &ChatRequest,
    mutation_succeeded: bool,
    verification_succeeded: bool,
) -> bool {
    if mutation_succeeded && verification_succeeded {
        return false;
    }
    if crate::runtime::tool::is_question_only_request(request) {
        return false;
    }
    if !has_completion_claim(content) {
        return false;
    }
    *content = if content
        .chars()
        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
    {
        if mutation_succeeded {
            "未验证完成：虽然执行了修改，但没有成功检查修改后的结果，因此不能确认任务真的完成。请运行读取检查、测试或构建验证。".to_string()
        } else {
            "未完成：本轮没有任何修改类工具成功执行，因此无法确认发生了实际改动。请重新执行所需操作，或明确说明当前阻塞项。".to_string()
        }
    } else {
        if mutation_succeeded {
            "Completion not verified: a modification ran, but its result was not successfully checked. Run a read-back, test, build, or equivalent verification before claiming completion.".to_string()
        } else {
            "Not completed: no modifying tool succeeded in this turn, so no actual change can be verified. Run the required operation or state the current blocker explicitly.".to_string()
        }
    };
    true
}

fn has_completion_claim(content: &str) -> bool {
    // Keep this strict: weak phrases like "搞定/修改了/done" used to false-trigger
    // an extra model round (and another full DeepSeek think cycle).
    const CLAIMS: &[&str] = &[
        "已完成",
        "全部完成",
        "完成修改",
        "修改完成",
        "修复完成",
        "更新完成",
        "创建完成",
        "写入完成",
        "任务完成",
        "全部搞定",
        "大功告成",
        "successfully completed",
        "task completed",
        "all done",
        "all set",
        "has been fixed",
        "have fixed",
        "has been completed",
        "have completed",
        "implementation is complete",
        "changes are complete",
    ];
    let lower = content.to_ascii_lowercase();
    CLAIMS.iter().any(|claim| lower.contains(&claim.to_ascii_lowercase()))
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
            content: format!("[Follow-up instruction while you were working]\n{content}"),
            reasoning: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: now_millis(),
            estimated_tokens: None,
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
