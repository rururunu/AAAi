use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use sqlx::SqlitePool;
use tauri::async_runtime;
use tokio::sync::mpsc;

use crate::core::runtime::{ChatMessage, MessageStatus};

#[derive(Debug, Clone)]
pub struct JournalEvent {
    pub session_id: String,
    pub turn_id: String,
    pub message_id: String,
    pub kind: String,
    pub payload: Value,
}

#[derive(Clone)]
pub struct SessionJournal {
    tx: mpsc::UnboundedSender<JournalEvent>,
    delta_buffer: Arc<Mutex<DeltaBuffer>>,
}

struct DeltaBuffer {
    session_id: String,
    turn_id: String,
    message_id: String,
    content: String,
    reasoning: String,
    last_flush: Instant,
}

impl SessionJournal {
    pub fn new(pool: SqlitePool) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<JournalEvent>();
        async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(error) = insert_event(&pool, &event).await {
                    eprintln!("Failed to append journal event: {error}");
                }
            }
        });

        Self {
            tx,
            delta_buffer: Arc::new(Mutex::new(DeltaBuffer {
                session_id: String::new(),
                turn_id: String::new(),
                message_id: String::new(),
                content: String::new(),
                reasoning: String::new(),
                last_flush: Instant::now() - Duration::from_secs(1),
            })),
        }
    }

    pub fn append(
        &self,
        session_id: &str,
        turn_id: &str,
        message_id: &str,
        kind: &str,
        payload: Value,
    ) {
        let _ = self.tx.send(JournalEvent {
            session_id: session_id.to_string(),
            turn_id: turn_id.to_string(),
            message_id: message_id.to_string(),
            kind: kind.to_string(),
            payload,
        });
    }

    pub fn record_delta(
        &self,
        session_id: &str,
        turn_id: &str,
        message_id: &str,
        delta: &str,
        is_reasoning: bool,
    ) {
        let should_flush = {
            let Ok(mut buf) = self.delta_buffer.lock() else {
                return;
            };
            if buf.message_id != message_id {
                buf.session_id = session_id.to_string();
                buf.turn_id = turn_id.to_string();
                buf.message_id = message_id.to_string();
                buf.content.clear();
                buf.reasoning.clear();
                buf.last_flush = Instant::now() - Duration::from_secs(1);
            }
            if is_reasoning {
                buf.reasoning.push_str(delta);
            } else {
                buf.content.push_str(delta);
            }
            buf.last_flush.elapsed() >= Duration::from_millis(200)
                || buf.content.len() + buf.reasoning.len() >= 2048
        };
        if should_flush {
            self.flush_delta();
        }
    }

    pub fn flush_delta(&self) {
        let snapshot = {
            let Ok(mut buf) = self.delta_buffer.lock() else {
                return;
            };
            if buf.message_id.is_empty() {
                return;
            }
            if buf.content.is_empty() && buf.reasoning.is_empty() {
                return;
            }
            let snap = (
                buf.session_id.clone(),
                buf.turn_id.clone(),
                buf.message_id.clone(),
                buf.content.clone(),
                buf.reasoning.clone(),
            );
            buf.last_flush = Instant::now();
            snap
        };
        self.append(
            &snapshot.0,
            &snapshot.1,
            &snapshot.2,
            "delta",
            json!({
                "content": snapshot.3,
                "reasoning": snapshot.4,
            }),
        );
    }
}

async fn insert_event(pool: &SqlitePool, event: &JournalEvent) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let payload = serde_json::to_string(&event.payload).unwrap_or_else(|_| "{}".into());
    sqlx::query(
        "INSERT INTO chat_journal_events (session_id, turn_id, message_id, kind, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&event.session_id)
    .bind(&event.turn_id)
    .bind(&event.message_id)
    .bind(&event.kind)
    .bind(payload)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn init_journal_schema(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS chat_journal_events (
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            turn_id TEXT NOT NULL,
            message_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_chat_journal_message
         ON chat_journal_events(message_id, seq);",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Rebuild accumulated content/reasoning for a message from journal delta events.
pub async fn rebuild_partial_from_journal(
    pool: &SqlitePool,
    message_id: &str,
) -> Result<(String, String), String> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT kind, payload_json FROM chat_journal_events
         WHERE message_id = ?1 AND kind = 'delta'
         ORDER BY seq ASC",
    )
    .bind(message_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut content = String::new();
    let mut reasoning = String::new();
    for (_kind, payload_json) in rows {
        let Ok(value) = serde_json::from_str::<Value>(&payload_json) else {
            continue;
        };
        // Snapshot-style deltas: latest flush replaces prior buffer snapshot.
        if let Some(text) = value.get("content").and_then(|v| v.as_str()) {
            content = text.to_string();
        }
        if let Some(text) = value.get("reasoning").and_then(|v| v.as_str()) {
            reasoning = text.to_string();
        }
    }
    Ok((content, reasoning))
}

/// Apply journal partials onto orphaned streaming/pending messages before settle.
pub async fn hydrate_orphaned_from_journal(
    pool: &SqlitePool,
    messages: &mut [ChatMessage],
) -> Result<(), String> {
    for message in messages.iter_mut() {
        if !matches!(
            message.status,
            MessageStatus::Pending | MessageStatus::Streaming
        ) {
            continue;
        }
        let (content, reasoning) = rebuild_partial_from_journal(pool, &message.id).await?;
        if !content.is_empty() {
            message.content = content;
        }
        if !reasoning.is_empty() {
            message.reasoning = Some(reasoning);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn journal_schema_and_delta_rebuild() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("memory sqlite");
        init_journal_schema(&pool).await.expect("schema");
        let event = JournalEvent {
            session_id: "s1".into(),
            turn_id: "t1".into(),
            message_id: "m1".into(),
            kind: "delta".into(),
            payload: json!({"content": "hello", "reasoning": "think"}),
        };
        insert_event(&pool, &event).await.expect("insert");
        let event2 = JournalEvent {
            session_id: "s1".into(),
            turn_id: "t1".into(),
            message_id: "m1".into(),
            kind: "delta".into(),
            payload: json!({"content": "hello world", "reasoning": "think more"}),
        };
        insert_event(&pool, &event2).await.expect("insert2");
        let (content, reasoning) = rebuild_partial_from_journal(&pool, "m1")
            .await
            .expect("rebuild");
        assert_eq!(content, "hello world");
        assert_eq!(reasoning, "think more");
    }
}
