use crate::core::runtime::stream::ToolCallPayload;
use crate::core::runtime::{ChatMessage, MessageStatus, Role, ToolActivity};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn init_db(db_path: &Path) -> Result<SqlitePool, String> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .map_err(|e| e.to_string())?;

    // Create messages table if not exists
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS chat_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            reasoning TEXT,
            tool_activities TEXT,
            tool_calls TEXT,
            tool_call_id TEXT,
            name TEXT,
            status TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        );",
    )
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let message_columns = sqlx::query("PRAGMA table_info(chat_messages)")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if !message_columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "tool_activities")
    {
        sqlx::query("ALTER TABLE chat_messages ADD COLUMN tool_activities TEXT")
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Create index on session_id for faster history lookup
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_session_id ON chat_messages(session_id);",
    )
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    init_chat_session_schema(&pool).await?;

    Ok(pool)
}

async fn init_chat_session_schema(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS chat_sessions (
            session_id TEXT PRIMARY KEY,
            workspace_id TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    let columns = sqlx::query("PRAGMA table_info(chat_sessions)")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let column_names = columns
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();
    if !column_names.iter().any(|name| name == "workspace_id") {
        sqlx::query("ALTER TABLE chat_sessions ADD COLUMN workspace_id TEXT")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if column_names.iter().any(|name| name == "workspace_path") {
        sqlx::query(
            "UPDATE chat_sessions SET workspace_id = workspace_path
             WHERE workspace_id IS NULL AND workspace_path IS NOT NULL",
        )
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub async fn load_session_workspaces(pool: &SqlitePool) -> Result<HashMap<String, String>, String> {
    let rows = sqlx::query(
        "SELECT session_id, workspace_id FROM chat_sessions WHERE workspace_id IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get("session_id"), row.get("workspace_id")))
        .collect())
}

pub async fn bind_session_workspace(
    pool: &SqlitePool,
    session_id: &str,
    workspace_id: &str,
) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    sqlx::query(
        "INSERT OR IGNORE INTO chat_sessions
         (session_id, workspace_id, created_at, updated_at) VALUES (?, ?, ?, ?)",
    )
    .bind(session_id)
    .bind(workspace_id)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod session_workspace_tests {
    use super::*;

    #[tokio::test]
    async fn migrates_legacy_workspace_path_and_preserves_first_binding() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE chat_sessions (
                session_id TEXT PRIMARY KEY,
                workspace_path TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO chat_sessions VALUES ('legacy', 'D:\\Code\\Peek', 1, 1)")
            .execute(&pool)
            .await
            .unwrap();

        init_chat_session_schema(&pool).await.unwrap();
        bind_session_workspace(&pool, "new", "D:\\Code\\VueAdmin")
            .await
            .unwrap();
        bind_session_workspace(&pool, "new", "D:\\Code\\Other")
            .await
            .unwrap();
        let workspaces = load_session_workspaces(&pool).await.unwrap();

        assert_eq!(workspaces["legacy"], "D:\\Code\\Peek");
        assert_eq!(workspaces["new"], "D:\\Code\\VueAdmin");
    }
}

pub async fn save_message(pool: &SqlitePool, msg: &ChatMessage) -> Result<(), String> {
    let role_str = match msg.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    };

    let status_str = match msg.status {
        MessageStatus::Pending => "pending",
        MessageStatus::Streaming => "streaming",
        MessageStatus::Done => "done",
        MessageStatus::Error => "error",
        MessageStatus::Cancelled => "cancelled",
    };

    let tool_calls_json = msg
        .tool_calls
        .as_ref()
        .and_then(|tc| serde_json::to_string(tc).ok());
    let tool_activities_json = msg
        .tool_activities
        .as_ref()
        .and_then(|value| serde_json::to_string(value).ok());
    let timestamp_val = msg.timestamp as i64;

    sqlx::query(
        "INSERT OR REPLACE INTO chat_messages (
            id, session_id, role, content, reasoning, tool_activities, tool_calls, tool_call_id, name, status, timestamp
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);"
    )
    .bind(&msg.id)
    .bind(&msg.session_id)
    .bind(role_str)
    .bind(&msg.content)
    .bind(&msg.reasoning)
    .bind(tool_activities_json)
    .bind(tool_calls_json)
    .bind(&msg.tool_call_id)
    .bind(&msg.name)
    .bind(status_str)
    .bind(timestamp_val)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn load_all_messages(pool: &SqlitePool) -> Result<Vec<ChatMessage>, String> {
    let rows = sqlx::query(
        "SELECT id, session_id, role, content, reasoning, tool_activities, tool_calls, tool_call_id, name, status, timestamp
         FROM chat_messages
         ORDER BY timestamp ASC;"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut messages = Vec::new();
    for row in rows {
        let id: String = row.get("id");
        let session_id: String = row.get("session_id");

        let role_str: String = row.get("role");
        let role = match role_str.as_str() {
            "system" => Role::System,
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            _ => Role::User,
        };

        let content: String = row.get("content");
        let reasoning: Option<String> = row.get("reasoning");
        let tool_activities_str: Option<String> = row.get("tool_activities");
        let tool_activities: Option<Vec<ToolActivity>> =
            tool_activities_str.and_then(|value| serde_json::from_str(&value).ok());

        let tool_calls_str: Option<String> = row.get("tool_calls");
        let tool_calls: Option<Vec<ToolCallPayload>> =
            tool_calls_str.and_then(|s| serde_json::from_str(&s).ok());

        let tool_call_id: Option<String> = row.get("tool_call_id");
        let name: Option<String> = row.get("name");

        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "pending" => MessageStatus::Pending,
            "streaming" => MessageStatus::Streaming,
            "done" => MessageStatus::Done,
            "error" => MessageStatus::Error,
            "cancelled" => MessageStatus::Cancelled,
            _ => MessageStatus::Done,
        };

        let timestamp_val: i64 = row.get("timestamp");
        let timestamp = timestamp_val as u64;

        messages.push(ChatMessage {
            id,
            session_id,
            role,
            content,
            reasoning,
            tool_activities,
            tool_calls,
            tool_call_id,
            name,
            status,
            timestamp,
        });
    }

    Ok(messages)
}

#[cfg(test)]
mod message_persistence_tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn round_trips_tool_activities() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE chat_messages (
                id TEXT PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL,
                content TEXT NOT NULL, reasoning TEXT, tool_activities TEXT,
                tool_calls TEXT, tool_call_id TEXT, name TEXT,
                status TEXT NOT NULL, timestamp INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        let message = ChatMessage {
            id: "assistant-1".into(),
            session_id: "session-1".into(),
            role: Role::Assistant,
            content: "done".into(),
            reasoning: None,
            tool_activities: Some(vec![ToolActivity {
                id: "activity-1".into(),
                tool_name: "replace_in_file".into(),
                title: "Modify src/main.ts".into(),
                kind: "edit".into(),
                detail: None,
                arguments: Some(json!({ "path": "src/main.ts" })),
                result: Some("replaced".into()),
                success: true,
                status: "done".into(),
            }]),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
        };

        save_message(&pool, &message).await.unwrap();
        let loaded = load_all_messages(&pool).await.unwrap();

        assert_eq!(loaded, vec![message]);
    }
}
