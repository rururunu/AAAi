use std::collections::HashMap;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::async_runtime;
use tokio::sync::mpsc;

use crate::core::ai::provider::{AIProvider, ProviderError};
use crate::core::chat::agent::AgentRunner;
use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::chat::error::ChatError;
use crate::core::event::{BusEvent, EventBus};
use crate::core::runtime::{ChatRequest, MessageStatus, StreamEvent};
use crate::core::tools::context::{AskStore, PathPermissionStore, TaskItem, ToolContext};
use crate::runtime::ToolManager;

struct ActiveTask {
    cancelled: Arc<AtomicBool>,
    content: Arc<Mutex<String>>,
    reasoning: Arc<Mutex<String>>,
}

pub struct StreamManager {
    active_tasks: Arc<Mutex<HashMap<String, ActiveTask>>>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn spawn(
        &self,
        provider: Arc<dyn AIProvider>,
        tools: Arc<ToolManager>,
        event_bus: Arc<dyn EventBus>,
        conversation: Arc<ConversationManager>,
        ask_store: Arc<AskStore>,
        path_permission_store: Arc<PathPermissionStore>,
        tasks: Arc<Mutex<Vec<TaskItem>>>,
        app_handle: Option<tauri::AppHandle>,
        request: ChatRequest,
        assistant_message_id: String,
        session_id: String,
    ) {
        let cancelled = Arc::new(AtomicBool::new(false));
        let content_ref = Arc::new(Mutex::new(String::new()));
        let reasoning_ref = Arc::new(Mutex::new(String::new()));
        if let Ok(mut active) = self.active_tasks.lock() {
            active.insert(
                assistant_message_id.clone(),
                ActiveTask {
                    cancelled: cancelled.clone(),
                    content: Arc::clone(&content_ref),
                    reasoning: Arc::clone(&reasoning_ref),
                },
            );
        }

        let active_tasks = Arc::clone(&self.active_tasks);
        let workspace_root = request
            .context
            .workspace
            .as_ref()
            .map(|workspace| std::path::PathBuf::from(&workspace.root))
            .unwrap_or_else(public_workspace_root);

        async_runtime::spawn(async move {
            let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
            let tool_ctx = ToolContext {
                workspace_root,
                request_context: request.context.clone(),
                session_id: session_id.clone(),
                assistant_message_id: assistant_message_id.clone(),
                conversation: Arc::clone(&conversation),
                event_bus: Arc::clone(&event_bus),
                tasks,
                ask_store,
                path_permission_store,
                registry: Some(tools.registry()),
                provider: Some(provider.clone()),
                subagent_depth: 0,
                max_subagent_depth: 3,
                app_handle,
            };

            let runner = AgentRunner::new(provider.clone(), tools);
            let agent_task = async_runtime::spawn({
                let request = request.clone();
                let tx = tx.clone();
                let cancelled = cancelled.clone();
                async move { runner.run(request, tool_ctx, tx, cancelled).await }
            });
            // 关键修复：显式丢弃这份 tx。否则外层作用域一直持有一个未使用的
            // Sender，即使 agent_task 内部的 tx 克隆已经在任务完成后被丢弃，
            // channel 的发送端计数也不会归零，下面的 `rx.recv().await` 会永远
            // 阻塞、永远等不到 None，导致成功完成的对话永远不会跳出循环去发
            // ChatFinished —— 这正是"AI 回复完按钮还显示暂停"的根因。
            drop(tx);

            let mut content = String::new();
            let mut reasoning = String::new();
            let mut streaming_started = false;
            let mut finish_reason = None;

            while let Some(event) = rx.recv().await {
                if cancelled.load(Ordering::Relaxed) {
                    break;
                }

                match event {
                    StreamEvent::Start => {}
                    StreamEvent::Delta(delta) => {
                        if !streaming_started {
                            streaming_started = true;
                            conversation.update_message(
                                &session_id,
                                &assistant_message_id,
                                MessageStatus::Streaming,
                                None,
                                None,
                            );
                        }
                        content.push_str(&delta);
                        if let Ok(mut guard) = content_ref.lock() {
                            guard.push_str(&delta);
                        }
                        event_bus.emit(BusEvent::ChatDelta {
                            session_id: session_id.clone(),
                            message_id: assistant_message_id.clone(),
                            delta,
                        });
                    }
                    StreamEvent::Reasoning(chunk) => {
                        reasoning.push_str(&chunk);
                        if let Ok(mut guard) = reasoning_ref.lock() {
                            guard.push_str(&chunk);
                        }
                        event_bus.emit(BusEvent::ChatReasoning {
                            session_id: session_id.clone(),
                            message_id: assistant_message_id.clone(),
                            content: chunk,
                        });
                    }
                    StreamEvent::ToolCall(_) => {}
                    StreamEvent::TurnComplete {
                        content: turn_content,
                        reasoning: turn_reasoning,
                        tool_calls: _,
                        finish_reason: turn_finish,
                    } => {
                        if !turn_content.is_empty() {
                            content = turn_content;
                            if let Ok(mut guard) = content_ref.lock() {
                                *guard = content.clone();
                            }
                        }
                        if let Some(value) = turn_reasoning {
                            reasoning = value;
                            if let Ok(mut guard) = reasoning_ref.lock() {
                                *guard = reasoning.clone();
                            }
                        }
                        finish_reason = turn_finish;
                    }
                    StreamEvent::Finish => break,
                    StreamEvent::Error(message) => {
                        finish_with_error(
                            &event_bus,
                            &conversation,
                            &session_id,
                            &assistant_message_id,
                            content,
                            reasoning,
                            message,
                        );
                        let _ = crate::core::checkpoint::shared_checkpoint_store()
                            .finish_turn(&session_id);
                        active_tasks
                            .lock()
                            .ok()
                            .and_then(|mut active| active.remove(&assistant_message_id));
                        let _ = agent_task.await;
                        return;
                    }
                }
            }

            active_tasks
                .lock()
                .ok()
                .and_then(|mut active| active.remove(&assistant_message_id));

            let result = if cancelled.load(Ordering::Relaxed) {
                Err(ProviderError::cancelled())
            } else {
                match agent_task.await {
                    Ok(result) => result,
                    Err(error) => Err(ProviderError::message(format!(
                        "agent task failed: {error}"
                    ))),
                }
            };

            match result {
                Ok(()) => finish_success(
                    &event_bus,
                    &conversation,
                    &session_id,
                    &assistant_message_id,
                    content,
                    reasoning,
                    finish_reason,
                ),
                Err(ProviderError::Cancelled) => {}
                Err(error) => finish_with_error(
                    &event_bus,
                    &conversation,
                    &session_id,
                    &assistant_message_id,
                    content,
                    reasoning,
                    error.to_string(),
                ),
            }
            let _ = crate::core::checkpoint::shared_checkpoint_store().finish_turn(&session_id);
        });
    }

    pub fn cancel(
        &self,
        conversation: &ConversationManager,
        event_bus: &dyn EventBus,
        message_id: &str,
    ) -> Result<(), ChatError> {
        let active = self
            .active_tasks
            .lock()
            .map_err(|error| ChatError::Internal(error.to_string()))?
            .remove(message_id);

        let Some(task) = active else {
            // No live task (app restarted mid-run). Still finalize the stored message
            // so the UI is not stuck in "executing" forever.
            if let Some((session_id, message)) =
                conversation.settle_interrupted_message(message_id)
            {
                event_bus.emit(BusEvent::ChatFinished {
                    session_id,
                    message_id: message_id.to_string(),
                    content: message.content,
                    reasoning: message.reasoning,
                    finish_reason: Some("cancelled".to_string()),
                });
                return Ok(());
            }
            return Err(ChatError::MessageNotFound);
        };

        task.cancelled.store(true, Ordering::Relaxed);

        let content = task
            .content
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default();
        let reasoning = task
            .reasoning
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default();

        let (session_id, message) = conversation.find_message(message_id)?;
        if let Some(message) = conversation.update_message(
            &session_id,
            message_id,
            MessageStatus::Cancelled,
            Some(if content.is_empty() {
                message.content
            } else {
                content
            }),
            Some(non_empty_string(reasoning)),
        ) {
            event_bus.emit(BusEvent::ChatFinished {
                session_id: session_id.clone(),
                message_id: message_id.to_string(),
                content: message.content,
                reasoning: message.reasoning,
                finish_reason: Some("cancelled".to_string()),
            });
        }
        let _ = crate::core::checkpoint::shared_checkpoint_store().finish_turn(&session_id);

        Ok(())
    }
}

fn finish_success(
    event_bus: &Arc<dyn EventBus>,
    conversation: &ConversationManager,
    session_id: &str,
    message_id: &str,
    content: String,
    reasoning: String,
    finish_reason: Option<String>,
) {
    let current_user = conversation
        .messages(session_id)
        .into_iter()
        .rev()
        .find(|message| message.role == crate::core::runtime::Role::User)
        .map(|message| super::selection::visible_user_text(&message.content).to_string());
    let reasoning = non_empty_string(reasoning);
    conversation.update_message(
        session_id,
        message_id,
        MessageStatus::Done,
        Some(content.clone()),
        Some(reasoning.clone()),
    );

    event_bus.emit(BusEvent::ChatFinished {
        session_id: session_id.to_string(),
        message_id: message_id.to_string(),
        content: content.clone(),
        reasoning,
        finish_reason: finish_reason.or(Some("stop".to_string())),
    });
    if let Some(user) = current_user {
        tauri::async_runtime::spawn_blocking(move || {
            crate::core::tools::memory::shared_memory_store().remember_exchange(user, content);
        });
    }
}

fn finish_with_error(
    event_bus: &Arc<dyn EventBus>,
    conversation: &ConversationManager,
    session_id: &str,
    message_id: &str,
    content: String,
    reasoning: String,
    error: String,
) {
    let reasoning = non_empty_string(reasoning);
    conversation.update_message(
        session_id,
        message_id,
        MessageStatus::Error,
        Some(error.clone()),
        Some(reasoning),
    );

    event_bus.emit(BusEvent::ChatError {
        session_id: session_id.to_string(),
        message_id: message_id.to_string(),
        message: error,
    });

    let _ = content;
}

fn non_empty_string(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn public_workspace_root() -> std::path::PathBuf {
    let root = env::temp_dir().join("peek-public");
    let _ = std::fs::create_dir_all(&root);
    root
}
