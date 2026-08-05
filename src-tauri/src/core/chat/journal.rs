use std::collections::HashMap;
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
    tx: mpsc::UnboundedSender<JournalCommand>,
    delta_buffers: Arc<Mutex<HashMap<String, DeltaBuffer>>>,
}

enum JournalCommand {
    Append(JournalEvent),
    DeleteMessage(String),
    DeleteSession(String),
    DeleteAll,
}

struct DeltaBuffer {
    session_id: String,
    turn_id: String,
    message_id: String,
    content: String,
    reasoning: String,
    last_flush: Instant,
    flushed_len: usize,
}

impl SessionJournal {
    pub fn new(pool: SqlitePool) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<JournalCommand>();
        async_runtime::spawn(async move {
            while let Some(command) = rx.recv().await {
                let result = match command {
                    JournalCommand::Append(event) => insert_event(&pool, &event).await,
                    JournalCommand::DeleteMessage(message_id) => {
                        delete_by_column(&pool, "message_id", &message_id).await
                    }
                    JournalCommand::DeleteSession(session_id) => {
                        delete_by_column(&pool, "session_id", &session_id).await
                    }
                    JournalCommand::DeleteAll => sqlx::query("DELETE FROM chat_journal_events")
                        .execute(&pool)
                        .await
                        .map(|_| ())
                        .map_err(|error| error.to_string()),
                };
                if let Err(error) = result {
                    eprintln!("Failed to update chat journal: {error}");
                }
            }
        });

        Self {
            tx,
            delta_buffers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn append(
        &self,
        session_id: &str,
        turn_id: &str,
        message_id: &str,
        kind: &str,
        payload: Value,
    ) {
        let _ = self.tx.send(JournalCommand::Append(JournalEvent {
            session_id: session_id.to_string(),
            turn_id: turn_id.to_string(),
            message_id: message_id.to_string(),
            kind: kind.to_string(),
            payload,
        }));
    }

    pub fn record_delta(
        &self,
        session_id: &str,
        turn_id: &str,
        message_id: &str,
        delta: &str,
        is_reasoning: bool,
    ) {
        let snapshot = {
            let Ok(mut buffers) = self.delta_buffers.lock() else {
                return;
            };
            let buf = buffers
                .entry(message_id.to_string())
                .or_insert_with(|| DeltaBuffer {
                    session_id: session_id.to_string(),
                    turn_id: turn_id.to_string(),
                    message_id: message_id.to_string(),
                    content: String::new(),
                    reasoning: String::new(),
                    last_flush: Instant::now() - Duration::from_secs(1),
                    flushed_len: 0,
                });
            if is_reasoning {
                buf.reasoning.push_str(delta);
            } else {
                buf.content.push_str(delta);
            }
            let current_len = buf.content.len() + buf.reasoning.len();
            if buf.last_flush.elapsed() >= Duration::from_secs(1)
                || current_len.saturating_sub(buf.flushed_len) >= 4096
            {
                buf.last_flush = Instant::now();
                buf.flushed_len = current_len;
                Some(buf.snapshot())
            } else {
                None
            }
        };
        if let Some(snapshot) = snapshot {
            self.append_snapshot(snapshot);
        }
    }

    pub fn flush_message(&self, message_id: &str) {
        let snapshot = {
            let Ok(mut buffers) = self.delta_buffers.lock() else {
                return;
            };
            let Some(buf) = buffers.get_mut(message_id) else {
                return;
            };
            if buf.content.is_empty() && buf.reasoning.is_empty() {
                return;
            }
            buf.last_flush = Instant::now();
            buf.flushed_len = buf.content.len() + buf.reasoning.len();
            buf.snapshot()
        };
        self.append_snapshot(snapshot);
    }

    pub fn discard_message(&self, message_id: &str) {
        if let Ok(mut buffers) = self.delta_buffers.lock() {
            buffers.remove(message_id);
        }
        let _ = self
            .tx
            .send(JournalCommand::DeleteMessage(message_id.to_string()));
    }

    pub fn discard_session(&self, session_id: &str) {
        if let Ok(mut buffers) = self.delta_buffers.lock() {
            buffers.retain(|_, buffer| buffer.session_id != session_id);
        }
        let _ = self
            .tx
            .send(JournalCommand::DeleteSession(session_id.to_string()));
    }

    pub fn discard_all(&self) {
        if let Ok(mut buffers) = self.delta_buffers.lock() {
            buffers.clear();
        }
        let _ = self.tx.send(JournalCommand::DeleteAll);
    }

    /// Record a tool outcome for trajectory mining (failures → rule/skill candidates).
    pub fn record_tool_outcome(
        &self,
        session_id: &str,
        turn_id: &str,
        message_id: &str,
        tool_name: &str,
        arguments: &str,
        success: bool,
        result: &str,
    ) {
        let kind = if success { "tool_result" } else { "tool_error" };
        self.append(
            session_id,
            turn_id,
            message_id,
            kind,
            json!({
                "tool": tool_name,
                "arguments": arguments,
                "success": success,
                "result": truncate_payload(result, 2_000),
            }),
        );
    }

    fn append_snapshot(&self, snapshot: (String, String, String, String, String)) {
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

fn truncate_payload(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

impl DeltaBuffer {
    fn snapshot(&self) -> (String, String, String, String, String) {
        (
            self.session_id.clone(),
            self.turn_id.clone(),
            self.message_id.clone(),
            self.content.clone(),
            self.reasoning.clone(),
        )
    }
}

async fn delete_by_column(pool: &SqlitePool, column: &str, value: &str) -> Result<(), String> {
    let query = format!("DELETE FROM chat_journal_events WHERE {column} = ?");
    sqlx::query(&query)
        .bind(value)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

async fn insert_event(pool: &SqlitePool, event: &JournalEvent) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let payload = serde_json::to_string(&event.payload).unwrap_or_else(|_| "{}".into());
    if event.kind == "delta" {
        let updated = sqlx::query(
            "UPDATE chat_journal_events
             SET session_id = ?1, turn_id = ?2, payload_json = ?3, created_at = ?4
             WHERE seq = (
                 SELECT MAX(seq) FROM chat_journal_events
                 WHERE message_id = ?5 AND kind = 'delta'
             )",
        )
        .bind(&event.session_id)
        .bind(&event.turn_id)
        .bind(&payload)
        .bind(now)
        .bind(&event.message_id)
        .execute(pool)
        .await
        .map_err(|error| error.to_string())?;
        if updated.rows_affected() > 0 {
            return Ok(());
        }
    }
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

/// The journal is crash-recovery state, not permanent history. Keep only the
/// latest stream snapshot for messages that were still active at shutdown.
pub async fn compact_recovery_journal(pool: &SqlitePool) -> Result<(), String> {
    let needs_repair: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1
            FROM chat_journal_events journal
            LEFT JOIN chat_messages message ON message.id = journal.message_id
            WHERE journal.kind != 'delta'
               OR message.id IS NULL
               OR message.status NOT IN ('pending', 'streaming')
               OR journal.seq != (
                   SELECT MAX(candidate.seq)
                   FROM chat_journal_events candidate
                   WHERE candidate.message_id = journal.message_id
                     AND candidate.kind = 'delta'
               )
            LIMIT 1
        )",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    if !needs_repair {
        return Ok(());
    }

    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    for statement in [
        "CREATE TEMP TABLE retained_chat_journal AS
         SELECT session_id, turn_id, message_id, kind, payload_json, created_at
         FROM chat_journal_events
         WHERE kind = 'delta'
           AND seq IN (
               SELECT MAX(seq) FROM chat_journal_events
               WHERE kind = 'delta' GROUP BY message_id
           )
           AND message_id IN (
               SELECT id FROM chat_messages WHERE status IN ('pending', 'streaming')
           )",
        "DROP TABLE chat_journal_events",
        "CREATE TABLE chat_journal_events (
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            turn_id TEXT NOT NULL,
            message_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
        "INSERT INTO chat_journal_events
         (session_id, turn_id, message_id, kind, payload_json, created_at)
         SELECT session_id, turn_id, message_id, kind, payload_json, created_at
         FROM retained_chat_journal",
        "DROP TABLE retained_chat_journal",
        "CREATE INDEX idx_chat_journal_message
         ON chat_journal_events(message_id, seq)",
    ] {
        sqlx::query(statement)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
    }
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())
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
    use sqlx::sqlite::SqlitePoolOptions;

    async fn wait_for_count(pool: &SqlitePool, expected: i64) {
        for _ in 0..100 {
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chat_journal_events")
                .fetch_one(pool)
                .await
                .unwrap();
            if count == expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("journal row count did not reach {expected}");
    }

    async fn wait_for_content(pool: &SqlitePool, message_id: &str, expected: &str) {
        for _ in 0..100 {
            if rebuild_partial_from_journal(pool, message_id)
                .await
                .unwrap()
                .0
                == expected
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("journal content for {message_id} did not reach expected snapshot");
    }

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
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM chat_journal_events WHERE message_id = 'm1' AND kind = 'delta'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn recovery_compaction_keeps_only_latest_active_snapshot() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("memory sqlite");
        sqlx::query("CREATE TABLE chat_messages (id TEXT PRIMARY KEY, status TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO chat_messages VALUES ('active', 'streaming'), ('done', 'done')")
            .execute(&pool)
            .await
            .unwrap();
        init_journal_schema(&pool).await.unwrap();

        for (message_id, content) in [
            ("active", "first"),
            ("active", "latest"),
            ("done", "discard me"),
        ] {
            insert_event(
                &pool,
                &JournalEvent {
                    session_id: "s1".into(),
                    turn_id: "t1".into(),
                    message_id: message_id.into(),
                    kind: "delta".into(),
                    payload: json!({ "content": content }),
                },
            )
            .await
            .unwrap();
        }

        compact_recovery_journal(&pool).await.unwrap();
        assert_eq!(
            rebuild_partial_from_journal(&pool, "active")
                .await
                .unwrap()
                .0,
            "latest"
        );
        assert_eq!(
            rebuild_partial_from_journal(&pool, "done").await.unwrap().0,
            ""
        );
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chat_journal_events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn worker_keeps_independent_snapshots_and_discards_session() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        init_journal_schema(&pool).await.unwrap();
        let journal = SessionJournal::new(pool.clone());

        journal.record_delta("s1", "t1", "m1", "hello", false);
        journal.record_delta("s1", "t2", "m2", "other", false);
        journal.record_delta("s1", "t1", "m1", " world", false);
        journal.flush_message("m1");
        journal.flush_message("m2");
        wait_for_count(&pool, 2).await;
        wait_for_content(&pool, "m1", "hello world").await;
        wait_for_content(&pool, "m2", "other").await;

        assert_eq!(
            rebuild_partial_from_journal(&pool, "m1").await.unwrap().0,
            "hello world"
        );
        assert_eq!(
            rebuild_partial_from_journal(&pool, "m2").await.unwrap().0,
            "other"
        );

        journal.discard_session("s1");
        wait_for_count(&pool, 0).await;
    }
}
