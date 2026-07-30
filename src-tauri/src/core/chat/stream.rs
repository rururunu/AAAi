use std::collections::{HashMap, VecDeque};
use std::env;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::json;
use tauri::async_runtime;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::core::ai::provider::{AIProvider, ProviderError};
use crate::core::chat::agent::AgentRunner;
use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::chat::error::ChatError;
use crate::core::chat::telemetry::TurnSpan;
use crate::core::event::{BusEvent, EventBus};
use crate::core::runtime::{ChatRequest, MessageStatus, StreamEvent};
use crate::core::tools::context::{AskStore, PathPermissionStore, TaskItem, ToolContext};
use crate::runtime::ToolManager;

struct ActiveTask {
    session_id: String,
    epoch: u64,
    cancelled: Arc<AtomicBool>,
    soft_queue: Arc<Mutex<VecDeque<String>>>,
    content: Arc<Mutex<String>>,
    reasoning: Arc<Mutex<String>>,
}

pub struct StreamManager {
    active_tasks: Arc<Mutex<HashMap<String, ActiveTask>>>,
    epoch_counter: Arc<AtomicU64>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            epoch_counter: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn active_assistant_for_session(&self, session_id: &str) -> Option<String> {
        let active = self.active_tasks.lock().ok()?;
        active
            .iter()
            .find(|(_, task)| task.session_id == session_id)
            .map(|(id, _)| id.clone())
    }

    pub fn soft_inject(&self, session_id: &str, content: String) -> Result<String, ChatError> {
        let content = content.trim().to_string();
        if content.is_empty() {
            return Err(ChatError::EmptyMessage);
        }
        let mut active = self
            .active_tasks
            .lock()
            .map_err(|error| ChatError::Internal(error.to_string()))?;
        let (message_id, task) = active
            .iter_mut()
            .find(|(_, task)| task.session_id == session_id)
            .map(|(id, task)| (id.clone(), task))
            .ok_or(ChatError::MessageNotFound)?;
        if let Ok(mut queue) = task.soft_queue.lock() {
            queue.push_back(content);
        }
        Ok(message_id)
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
        max_turn_tokens: usize,
        model: String,
    ) {
        let cancelled = Arc::new(AtomicBool::new(false));
        let soft_queue = Arc::new(Mutex::new(VecDeque::new()));
        let content_ref = Arc::new(Mutex::new(String::new()));
        let reasoning_ref = Arc::new(Mutex::new(String::new()));
        let epoch = self.epoch_counter.fetch_add(1, Ordering::Relaxed);
        let turn_id = Uuid::new_v4().to_string();

        if let Ok(mut active) = self.active_tasks.lock() {
            active.insert(
                assistant_message_id.clone(),
                ActiveTask {
                    session_id: session_id.clone(),
                    epoch,
                    cancelled: cancelled.clone(),
                    soft_queue: Arc::clone(&soft_queue),
                    content: Arc::clone(&content_ref),
                    reasoning: Arc::clone(&reasoning_ref),
                },
            );
        }

        let journal = conversation.journal().clone();
        journal.append(
            &session_id,
            &turn_id,
            &assistant_message_id,
            "assistant_created",
            json!({ "status": "pending" }),
        );

        let active_tasks = Arc::clone(&self.active_tasks);
        let workspace_root = request
            .context
            .workspace
            .as_ref()
            .map(|workspace| std::path::PathBuf::from(&workspace.root))
            .unwrap_or_else(public_workspace_root);
        let provider_id = provider.id().to_string();

        async_runtime::spawn(async move {
            let mut turn_span = TurnSpan::start(
                &session_id,
                &turn_id,
                &assistant_message_id,
                &provider_id,
                &model,
            );

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
                max_subagent_depth: 1,
                subagent_id: None,
                parent_activity_id: None,
                app_handle,
                cancelled: Arc::clone(&cancelled),
            };

            let runner = AgentRunner::new(provider.clone(), tools)
                .with_max_turn_tokens(max_turn_tokens);
            let agent_task = async_runtime::spawn({
                let request = request.clone();
                let tx = tx.clone();
                let cancelled = cancelled.clone();
                let soft_queue = Arc::clone(&soft_queue);
                async move { runner.run(request, tool_ctx, tx, cancelled, soft_queue).await }
            });
            drop(tx);

            let mut content = String::new();
            let mut reasoning = String::new();
            let mut streaming_started = false;
            let mut finish_reason = None;

            while let Some(event) = rx.recv().await {
                if !epoch_still_active(&active_tasks, &assistant_message_id, epoch) {
                    break;
                }
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
                            journal.append(
                                &session_id,
                                &turn_id,
                                &assistant_message_id,
                                "status",
                                json!({ "status": "streaming" }),
                            );
                        }
                        turn_span.mark_first_token();
                        content.push_str(&delta);
                        if let Ok(mut guard) = content_ref.lock() {
                            guard.push_str(&delta);
                        }
                        journal.record_delta(
                            &session_id,
                            &turn_id,
                            &assistant_message_id,
                            &delta,
                            false,
                        );
                        // Also project content periodically via update_message for crash UX.
                        if content.len() % 512 < delta.len() {
                            conversation.update_message(
                                &session_id,
                                &assistant_message_id,
                                MessageStatus::Streaming,
                                Some(content.clone()),
                                Some(non_empty_string(reasoning.clone())),
                            );
                        }
                        event_bus.emit(BusEvent::ChatDelta {
                            session_id: session_id.clone(),
                            message_id: assistant_message_id.clone(),
                            delta,
                        });
                    }
                    StreamEvent::Reasoning(chunk) => {
                        turn_span.mark_first_token();
                        reasoning.push_str(&chunk);
                        if let Ok(mut guard) = reasoning_ref.lock() {
                            guard.push_str(&chunk);
                        }
                        journal.record_delta(
                            &session_id,
                            &turn_id,
                            &assistant_message_id,
                            &chunk,
                            true,
                        );
                        event_bus.emit(BusEvent::ChatReasoning {
                            session_id: session_id.clone(),
                            message_id: assistant_message_id.clone(),
                            content: chunk,
                        });
                    }
                    StreamEvent::Status { kind } => {
                        if kind == "soft_injected" {
                            turn_span.soft_inject(0);
                            journal.append(
                                &session_id,
                                &turn_id,
                                &assistant_message_id,
                                "soft_inject",
                                json!({}),
                            );
                        }
                        if kind.starts_with("tools:") {
                            if let Ok(count) = kind.trim_start_matches("tools:").parse::<u32>() {
                                turn_span.add_tools(count);
                            }
                        }
                        event_bus.emit(BusEvent::ChatStatus {
                            session_id: session_id.clone(),
                            message_id: assistant_message_id.clone(),
                            kind,
                        });
                    }
                    StreamEvent::UserContentPatch { message_id, content } => {
                        let status = conversation
                            .find_message(&message_id)
                            .map(|(_, msg)| msg.status)
                            .unwrap_or(MessageStatus::Done);
                        conversation.update_message(
                            &session_id,
                            &message_id,
                            status,
                            Some(content.clone()),
                            None,
                        );
                        event_bus.emit(BusEvent::ChatUserContent {
                            session_id: session_id.clone(),
                            message_id,
                            content,
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
                        journal.flush_delta();
                        journal.append(
                            &session_id,
                            &turn_id,
                            &assistant_message_id,
                            "error",
                            json!({ "message": message }),
                        );
                        turn_span.finish_err(&message);
                        if epoch_still_active(&active_tasks, &assistant_message_id, epoch) {
                            finish_with_error(
                                &event_bus,
                                &conversation,
                                &session_id,
                                &assistant_message_id,
                                content,
                                reasoning,
                                message,
                            );
                        }
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

            journal.flush_delta();

            let should_finish = match active_tasks.lock() {
                Ok(mut active) => match active.get(&assistant_message_id) {
                    Some(task) if task.epoch == epoch => {
                        active.remove(&assistant_message_id);
                        true
                    }
                    _ => false,
                },
                Err(_) => false,
            };

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

            if !should_finish {
                let _ = crate::core::checkpoint::shared_checkpoint_store().finish_turn(&session_id);
                return;
            }

            match result {
                Ok(()) => {
                    journal.append(
                        &session_id,
                        &turn_id,
                        &assistant_message_id,
                        "finished",
                        json!({
                            "finish_reason": finish_reason.clone().unwrap_or_else(|| "stop".into()),
                            "content_len": content.len(),
                        }),
                    );
                    turn_span.finish_ok(finish_reason.as_deref());
                    finish_success(
                        &event_bus,
                        &conversation,
                        &session_id,
                        &assistant_message_id,
                        content,
                        reasoning,
                        finish_reason,
                    );
                }
                Err(ProviderError::Cancelled) => {
                    journal.append(
                        &session_id,
                        &turn_id,
                        &assistant_message_id,
                        "finished",
                        json!({ "finish_reason": "cancelled" }),
                    );
                    turn_span.finish_err("cancelled");
                }
                Err(error) => {
                    let message = error.to_string();
                    journal.append(
                        &session_id,
                        &turn_id,
                        &assistant_message_id,
                        "error",
                        json!({ "message": message }),
                    );
                    turn_span.finish_err(&message);
                    finish_with_error(
                        &event_bus,
                        &conversation,
                        &session_id,
                        &assistant_message_id,
                        content,
                        reasoning,
                        message,
                    );
                }
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

        // Bump global epoch so late events from this task are ignored if any race.
        self.epoch_counter.fetch_add(1, Ordering::Relaxed);
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

        conversation.journal().flush_delta();
        conversation.journal().append(
            &task.session_id,
            "cancel",
            message_id,
            "finished",
            json!({ "finish_reason": "cancelled" }),
        );

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

fn epoch_still_active(
    active_tasks: &Arc<Mutex<HashMap<String, ActiveTask>>>,
    message_id: &str,
    epoch: u64,
) -> bool {
    active_tasks
        .lock()
        .ok()
        .and_then(|active| active.get(message_id).map(|task| task.epoch == epoch))
        .unwrap_or(false)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_mismatch_is_inactive() {
        let tasks = Arc::new(Mutex::new(HashMap::new()));
        tasks.lock().unwrap().insert(
            "m1".into(),
            ActiveTask {
                session_id: "s1".into(),
                epoch: 2,
                cancelled: Arc::new(AtomicBool::new(false)),
                soft_queue: Arc::new(Mutex::new(VecDeque::new())),
                content: Arc::new(Mutex::new(String::new())),
                reasoning: Arc::new(Mutex::new(String::new())),
            },
        );
        assert!(!epoch_still_active(&tasks, "m1", 1));
        assert!(epoch_still_active(&tasks, "m1", 2));
    }
}
