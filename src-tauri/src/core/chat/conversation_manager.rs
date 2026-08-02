use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::chat::error::ChatError;
use crate::core::chat::limits::estimate_message_tokens;
use crate::core::runtime::{ChatMessage, MessageStatus, Role, ToolActivity};
use crate::models::chat::ChatSessionSummary;

fn block_on_compat<F>(future: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(|| handle.block_on(future))
        }
        Ok(_) => std::thread::spawn(move || tauri::async_runtime::block_on(future))
            .join()
            .expect("async database init thread panicked"),
        Err(_) => tauri::async_runtime::block_on(future),
    }
}

pub struct ConversationManager {
    sessions: Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>,
    session_workspaces: Arc<Mutex<HashMap<String, String>>>,
    db_pool: sqlx::SqlitePool,
    journal: super::journal::SessionJournal,
}

impl ConversationManager {
    pub fn new(db_path: std::path::PathBuf) -> Self {
        let db_pool = block_on_compat({
            let db_path = db_path.clone();
            async move {
                super::db::init_db(&db_path)
                    .await
                    .expect("Failed to initialize SQLite database")
            }
        });

        let journal = super::journal::SessionJournal::new(db_pool.clone());

        // Load all existing messages into sessions map
        let all_messages = block_on_compat({
            let db_pool = db_pool.clone();
            async move {
                super::db::load_all_messages(&db_pool)
                    .await
                    .expect("Failed to load messages from SQLite")
            }
        });

        let mut sessions = HashMap::new();
        for msg in all_messages {
            sessions
                .entry(msg.session_id.clone())
                .or_insert_with(Vec::new)
                .push(msg);
        }

        // Rebuild partial streaming content from journal before settling orphans.
        {
            let pool = db_pool.clone();
            let mut flat: Vec<ChatMessage> = sessions.values().flatten().cloned().collect();
            let flat = block_on_compat({
                let pool = pool.clone();
                async move {
                    let _ = super::journal::hydrate_orphaned_from_journal(&pool, &mut flat).await;
                    flat
                }
            });
            // Write hydrated content back into the session map.
            let by_id: HashMap<String, ChatMessage> =
                flat.into_iter().map(|m| (m.id.clone(), m)).collect();
            for messages in sessions.values_mut() {
                for message in messages.iter_mut() {
                    if let Some(hydrated) = by_id.get(&message.id) {
                        if matches!(
                            message.status,
                            MessageStatus::Pending | MessageStatus::Streaming
                        ) {
                            message.content = hydrated.content.clone();
                            message.reasoning = hydrated.reasoning.clone();
                        }
                    }
                }
            }
        }

        // Crash / force-quit can leave pending/streaming messages and running tools.
        // Nothing is still executing after process start, so finalize them now.
        let dirty = settle_orphaned_in_sessions(&mut sessions);
        let pool_for_settle = db_pool.clone();
        let journal_for_settle = journal.clone();
        if !dirty.is_empty() {
            tauri::async_runtime::spawn(async move {
                for message in dirty {
                    if let Err(e) = super::db::save_message(&pool_for_settle, &message).await {
                        eprintln!("Failed to settle interrupted message {}: {}", message.id, e);
                    } else {
                        journal_for_settle.discard_message(&message.id);
                    }
                }
            });
        }

        let session_workspaces = block_on_compat({
            let db_pool = db_pool.clone();
            async move {
                super::db::load_session_workspaces(&db_pool)
                    .await
                    .expect("Failed to load chat session workspaces")
            }
        });

        Self {
            sessions: Arc::new(Mutex::new(sessions)),
            session_workspaces: Arc::new(Mutex::new(session_workspaces)),
            db_pool,
            journal,
        }
    }

    pub fn journal(&self) -> &super::journal::SessionJournal {
        &self.journal
    }

    pub fn db_pool(&self) -> sqlx::SqlitePool {
        self.db_pool.clone()
    }

    pub fn inner(&self) -> Arc<Mutex<HashMap<String, Vec<ChatMessage>>>> {
        Arc::clone(&self.sessions)
    }

    pub fn append(&self, session_id: &str, mut message: ChatMessage) {
        refresh_message_token_cache(&mut message);
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions
                .entry(session_id.to_string())
                .or_default()
                .push(message.clone());
        }

        // Save to database asynchronously
        let pool = self.db_pool.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = super::db::save_message(&pool, &message).await {
                eprintln!("Failed to save message to SQLite: {}", e);
            }
        });
    }

    pub fn messages(&self, session_id: &str) -> Vec<ChatMessage> {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(session_id).cloned())
            .unwrap_or_default()
    }

    pub fn bind_workspace(&self, session_id: &str, workspace_id: &str) {
        if let Ok(mut workspaces) = self.session_workspaces.lock() {
            workspaces
                .entry(session_id.to_string())
                .or_insert_with(|| workspace_id.to_string());
        }
        let pool = self.db_pool.clone();
        let session_id = session_id.to_string();
        let workspace_id = workspace_id.to_string();
        tauri::async_runtime::spawn(async move {
            if let Err(error) =
                super::db::bind_session_workspace(&pool, &session_id, &workspace_id).await
            {
                eprintln!("Failed to bind chat session workspace: {error}");
            }
        });
    }

    pub fn workspace_for_session(&self, session_id: &str) -> Option<String> {
        self.session_workspaces
            .lock()
            .ok()
            .and_then(|workspaces| workspaces.get(session_id).cloned())
    }

    pub fn history(&self, session_id: &str) -> Result<Vec<ChatMessage>, ChatError> {
        Ok(self.messages(session_id))
    }

    pub fn find_message(&self, message_id: &str) -> Result<(String, ChatMessage), ChatError> {
        let sessions = self.sessions.lock().map_err(lock_error)?;

        for (session_id, messages) in sessions.iter() {
            if let Some(message) = messages.iter().find(|item| item.id == message_id) {
                return Ok((session_id.clone(), message.clone()));
            }
        }

        Err(ChatError::MessageNotFound)
    }

    pub fn list_sessions(&self) -> Vec<ChatSessionSummary> {
        let sessions = match self.sessions.lock() {
            Ok(guard) => guard,
            Err(_) => return Vec::new(),
        };
        let session_workspaces = self
            .session_workspaces
            .lock()
            .map(|workspaces| workspaces.clone())
            .unwrap_or_default();

        let mut summaries = sessions
            .iter()
            .filter_map(|(session_id, messages)| {
                if messages.is_empty() {
                    return None;
                }
                let preview = session_preview(messages);
                let updated_at = messages
                    .iter()
                    .map(|message| message.timestamp)
                    .max()
                    .unwrap_or(0);
                let turn_count = messages
                    .iter()
                    .filter(|message| message.role == Role::User)
                    .count();
                let estimated_tokens = messages
                    .iter()
                    .map(|message| {
                        message
                            .estimated_tokens
                            .unwrap_or_else(|| estimate_message_tokens(message))
                    })
                    .sum();
                Some(ChatSessionSummary {
                    session_id: session_id.clone(),
                    workspace_id: session_workspaces.get(session_id).cloned(),
                    preview,
                    message_count: messages.len(),
                    turn_count,
                    estimated_tokens,
                    updated_at,
                })
            })
            .collect::<Vec<_>>();

        summaries.sort_by_key(|summary| std::cmp::Reverse(summary.updated_at));
        summaries
    }

    pub fn update_message(
        &self,
        session_id: &str,
        message_id: &str,
        status: MessageStatus,
        content: Option<String>,
        reasoning: Option<Option<String>>,
    ) -> Option<ChatMessage> {
        let mut sessions = self.sessions.lock().ok()?;
        let messages = sessions.get_mut(session_id)?;
        let message = messages.iter_mut().find(|item| item.id == message_id)?;

        let mut updated = message.clone().with_status(status);
        if let Some(content) = content {
            updated = updated.with_content(content);
        }
        if let Some(reasoning) = reasoning {
            updated.reasoning = reasoning;
        }
        refresh_message_token_cache(&mut updated);
        *message = updated.clone();

        // Save updated message to SQLite asynchronously
        let is_terminal = !matches!(
            &updated.status,
            MessageStatus::Pending | MessageStatus::Streaming
        );
        let pool = self.db_pool.clone();
        let journal = self.journal.clone();
        let msg_to_save = updated.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = super::db::save_message(&pool, &msg_to_save).await {
                eprintln!("Failed to save updated message to SQLite: {}", e);
            } else if is_terminal {
                journal.discard_message(&msg_to_save.id);
            }
        });

        Some(updated)
    }

    pub fn upsert_tool_activity(
        &self,
        session_id: &str,
        message_id: &str,
        activity: ToolActivity,
    ) -> Option<ChatMessage> {
        let should_persist = activity.status != "running";
        let mut sessions = self.sessions.lock().ok()?;
        let message = sessions
            .get_mut(session_id)?
            .iter_mut()
            .find(|item| item.id == message_id)?;
        let activities = message.tool_activities.get_or_insert_with(Vec::new);
        if let Some(existing) = activities.iter_mut().find(|item| item.id == activity.id) {
            *existing = activity;
        } else {
            activities.push(activity);
        }
        if !should_persist {
            message.estimated_tokens = None;
            return Some(message.clone());
        }
        refresh_message_token_cache(message);
        let updated = message.clone();
        let pool = self.db_pool.clone();
        let msg_to_save = updated.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = super::db::save_message(&pool, &msg_to_save).await {
                eprintln!("Failed to save tool activity to SQLite: {}", e);
            }
        });
        Some(updated)
    }

    /// Remove `user_message_id` and every message after it in the session.
    pub async fn truncate_from_message(
        &self,
        session_id: &str,
        user_message_id: &str,
    ) -> Result<(), ChatError> {
        let removed_ids = {
            let mut sessions = self.sessions.lock().map_err(lock_error)?;
            let messages = sessions
                .get_mut(session_id)
                .ok_or(ChatError::MessageNotFound)?;
            let Some(index) = messages.iter().position(|m| m.id == user_message_id) else {
                return Err(ChatError::MessageNotFound);
            };
            let removed: Vec<String> = messages[index..].iter().map(|m| m.id.clone()).collect();
            messages.truncate(index);
            removed
        };

        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|error| ChatError::Internal(error.to_string()))?;
        for id in &removed_ids {
            sqlx::query("DELETE FROM chat_journal_events WHERE message_id = ?;")
                .bind(id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| ChatError::Internal(error.to_string()))?;
            sqlx::query("DELETE FROM chat_messages WHERE id = ?;")
                .bind(id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| ChatError::Internal(error.to_string()))?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| ChatError::Internal(error.to_string()))?;
        for id in removed_ids {
            self.journal.discard_message(&id);
        }
        Ok(())
    }

    pub fn delete_session(&self, session_id: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(session_id);
        }
        if let Ok(mut workspaces) = self.session_workspaces.lock() {
            workspaces.remove(session_id);
        }

        let pool = self.db_pool.clone();
        let sid = session_id.to_string();
        self.journal.discard_session(session_id);
        tauri::async_runtime::spawn(async move {
            let result = async {
                let mut transaction = pool.begin().await?;
                sqlx::query("DELETE FROM chat_journal_events WHERE session_id = ?;")
                    .bind(&sid)
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("DELETE FROM chat_messages WHERE session_id = ?;")
                    .bind(&sid)
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("DELETE FROM chat_sessions WHERE session_id = ?;")
                    .bind(&sid)
                    .execute(&mut *transaction)
                    .await?;
                transaction.commit().await?;
                // No-op for legacy databases; new databases use incremental
                // auto-vacuum so deletion can return a bounded batch.
                sqlx::query("PRAGMA incremental_vacuum(1024)")
                    .execute(&pool)
                    .await?;
                Ok::<(), sqlx::Error>(())
            }
            .await;
            if let Err(error) = result {
                eprintln!("Failed to delete session {sid} from SQLite: {error}");
            }
        });
    }

    pub fn clear_all_sessions(&self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.clear();
        }
        if let Ok(mut workspaces) = self.session_workspaces.lock() {
            workspaces.clear();
        }

        let pool = self.db_pool.clone();
        self.journal.discard_all();
        tauri::async_runtime::spawn(async move {
            let result = async {
                let mut transaction = pool.begin().await?;
                sqlx::query("DELETE FROM chat_journal_events")
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("DELETE FROM chat_messages")
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("DELETE FROM chat_sessions")
                    .execute(&mut *transaction)
                    .await?;
                transaction.commit().await?;
                sqlx::query("PRAGMA incremental_vacuum(1024)")
                    .execute(&pool)
                    .await?;
                Ok::<(), sqlx::Error>(())
            }
            .await;
            if let Err(error) = result {
                eprintln!("Failed to clear chat history in SQLite: {error}");
            }
        });
    }

    /// Finalize a message that is no longer backed by an active stream task
    /// (e.g. app was killed mid-run, then user hits pause on the restored chat).
    pub fn settle_interrupted_message(&self, message_id: &str) -> Option<(String, ChatMessage)> {
        let mut sessions = self.sessions.lock().ok()?;
        for (session_id, messages) in sessions.iter_mut() {
            let Some(message) = messages.iter_mut().find(|item| item.id == message_id) else {
                continue;
            };
            if !settle_message_in_place(message) {
                return None;
            }
            let updated = message.clone();
            let pool = self.db_pool.clone();
            let msg_to_save = updated.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = super::db::save_message(&pool, &msg_to_save).await {
                    eprintln!("Failed to save settled message {}: {}", msg_to_save.id, e);
                }
            });
            return Some((session_id.clone(), updated));
        }
        None
    }
}

fn settle_orphaned_in_sessions(
    sessions: &mut HashMap<String, Vec<ChatMessage>>,
) -> Vec<ChatMessage> {
    let mut dirty = Vec::new();
    for messages in sessions.values_mut() {
        for message in messages.iter_mut() {
            if settle_message_in_place(message) {
                dirty.push(message.clone());
            }
        }
    }
    dirty
}

fn settle_message_in_place(message: &mut ChatMessage) -> bool {
    let mut changed = false;
    if matches!(
        message.status,
        MessageStatus::Pending | MessageStatus::Streaming
    ) {
        message.status = MessageStatus::Cancelled;
        changed = true;
    }
    if let Some(activities) = message.tool_activities.as_mut() {
        for activity in activities.iter_mut() {
            if activity.status == "running" {
                activity.status = "error".into();
                activity.success = false;
                if activity
                    .result
                    .as_ref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    activity.result = Some("interrupted".into());
                }
                changed = true;
            }
        }
    }
    if changed {
        refresh_message_token_cache(message);
    }
    changed
}

fn session_preview(messages: &[ChatMessage]) -> String {
    for message in messages {
        if matches!(message.role, Role::User) {
            let trimmed = super::selection::visible_user_text(&message.content);
            if !trimmed.is_empty() {
                return truncate_preview(&trimmed);
            }
        }
    }
    for message in messages {
        if matches!(message.role, Role::Assistant) {
            let trimmed = message.content.trim();
            if !trimmed.is_empty() {
                return truncate_preview(trimmed);
            }
        }
    }
    "（空会话）".into()
}

fn truncate_preview(value: &str) -> String {
    const MAX: usize = 72;
    let normalized = value.replace('\n', " ").trim().to_string();
    if normalized.chars().count() <= MAX {
        return normalized;
    }
    let truncated: String = normalized.chars().take(MAX).collect();
    format!("{truncated}…")
}

pub fn create_message(
    session_id: &str,
    role: Role,
    content: String,
    status: MessageStatus,
) -> ChatMessage {
    ChatMessage {
        id: format!("msg-{}", uuid::Uuid::new_v4()),
        session_id: session_id.to_string(),
        role,
        content,
        reasoning: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status,
        timestamp: now_millis(),
        estimated_tokens: None,
    }
}

fn refresh_message_token_cache(message: &mut ChatMessage) {
    if matches!(
        message.status,
        MessageStatus::Pending | MessageStatus::Streaming
    ) {
        message.estimated_tokens = None;
    } else {
        message.estimated_tokens = Some(estimate_message_tokens(message));
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn lock_error<T: std::fmt::Display>(error: T) -> ChatError {
    ChatError::Internal(error.to_string())
}

#[cfg(test)]
mod rewind_tests {
    use super::{create_message, ConversationManager};
    use crate::core::chat::db;
    use crate::core::runtime::{MessageStatus, Role};

    #[tokio::test]
    async fn truncate_is_persisted_before_returning() {
        let db_path = std::env::temp_dir().join(format!(
            "aaai-rewind-conversation-{}.db",
            uuid::Uuid::new_v4()
        ));
        let manager = ConversationManager::new(db_path.clone());
        let session_id = "session";
        let first = create_message(session_id, Role::User, "keep".into(), MessageStatus::Done);
        let rewind_from =
            create_message(session_id, Role::User, "rewind".into(), MessageStatus::Done);
        let assistant = create_message(
            session_id,
            Role::Assistant,
            "answer".into(),
            MessageStatus::Done,
        );

        manager.sessions.lock().unwrap().insert(
            session_id.into(),
            vec![first.clone(), rewind_from.clone(), assistant.clone()],
        );
        for message in [&first, &rewind_from, &assistant] {
            db::save_message(&manager.db_pool, message).await.unwrap();
        }
        for message in [&rewind_from, &assistant] {
            sqlx::query(
                "INSERT INTO chat_journal_events
                 (session_id, turn_id, message_id, kind, payload_json, created_at)
                 VALUES (?, 'turn', ?, 'delta', '{}', 0)",
            )
            .bind(session_id)
            .bind(&message.id)
            .execute(&manager.db_pool)
            .await
            .unwrap();
        }

        manager
            .truncate_from_message(session_id, &rewind_from.id)
            .await
            .unwrap();
        assert_eq!(manager.messages(session_id), vec![first.clone()]);
        let journal_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM chat_journal_events WHERE session_id = ?")
                .bind(session_id)
                .fetch_one(&manager.db_pool)
                .await
                .unwrap();
        assert_eq!(journal_count, 0);
        drop(manager);

        let reloaded = ConversationManager::new(db_path.clone());
        let reloaded_messages = reloaded.messages(session_id);
        assert_eq!(reloaded_messages.len(), 1);
        assert_eq!(reloaded_messages[0].id, first.id);
        assert_eq!(reloaded_messages[0].content, first.content);
        assert!(reloaded_messages[0].estimated_tokens.is_some());
        drop(reloaded);

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
        let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    }
}
