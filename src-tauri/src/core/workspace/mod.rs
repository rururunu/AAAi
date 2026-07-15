use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Row, SqlitePool};
pub type WorkspaceId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub root: PathBuf,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct WorkspaceManager {
    current: Arc<RwLock<Option<Workspace>>>,
    workspaces: Arc<RwLock<Vec<Workspace>>>,
    db_pool: SqlitePool,
}

impl WorkspaceManager {
    pub fn new(db_path: PathBuf) -> Self {
        let db_pool = tauri::async_runtime::block_on(async {
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)
                    .expect("Failed to create workspace database directory");
            }
            let options = SqliteConnectOptions::new()
                .filename(db_path)
                .create_if_missing(true);
            let pool = SqlitePool::connect_with(options)
                .await
                .expect("Failed to connect to workspace database");
            init_schema(&pool)
                .await
                .expect("Failed to initialize workspace database");
            pool
        });

        let (workspaces, current) = tauri::async_runtime::block_on(load_state(&db_pool))
            .expect("Failed to load workspaces");

        Self {
            current: Arc::new(RwLock::new(current)),
            workspaces: Arc::new(RwLock::new(workspaces)),
            db_pool,
        }
    }

    pub fn current(&self) -> Option<Workspace> {
        self.current.read().ok().and_then(|value| value.clone())
    }

    pub fn list(&self) -> Vec<Workspace> {
        self.workspaces
            .read()
            .map(|items| items.clone())
            .unwrap_or_default()
    }

    pub async fn create(&self, root: PathBuf) -> Result<Workspace, String> {
        validate_root(&root)?;

        let id = root.to_string_lossy().to_string();
        if let Some(existing) = self.list().into_iter().find(|workspace| workspace.id == id) {
            return Ok(existing);
        }

        let workspace = Workspace {
            id,
            name: workspace_name(&root),
            root,
            description: None,
            created_at: Utc::now(),
        };
        let should_select = self.current().is_none();
        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|error| error.to_string())?;

        sqlx::query(
            "INSERT INTO workspace (id, name, root, description, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&workspace.id)
        .bind(&workspace.name)
        .bind(workspace.root.to_string_lossy().to_string())
        .bind(&workspace.description)
        .bind(workspace.created_at.to_rfc3339())
        .execute(&mut *transaction)
        .await
        .map_err(map_workspace_write_error)?;

        if should_select {
            save_current_id(&mut transaction, Some(workspace.id.clone())).await?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;

        self.workspaces
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())?
            .push(workspace.clone());
        if should_select {
            *self
                .current
                .write()
                .map_err(|_| "Workspace lock is poisoned".to_string())? = Some(workspace.clone());
        }

        Ok(workspace)
    }

    pub async fn switch(&self, id: WorkspaceId) -> Result<Workspace, String> {
        let workspace = self
            .list()
            .into_iter()
            .find(|workspace| workspace.id == id)
            .ok_or_else(|| "Workspace not found".to_string())?;

        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|error| error.to_string())?;
        save_current_id(&mut transaction, Some(id)).await?;
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;
        *self
            .current
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())? = Some(workspace.clone());

        Ok(workspace)
    }

    pub async fn clear_current(&self) -> Result<(), String> {
        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|error| error.to_string())?;
        save_current_id(&mut transaction, None).await?;
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;
        *self
            .current
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())? = None;
        Ok(())
    }

    pub async fn delete(&self, id: WorkspaceId) -> Result<(), String> {
        if !self.list().iter().any(|workspace| workspace.id == id) {
            return Err("Workspace not found".to_string());
        }

        let deleting_current = self.current().is_some_and(|workspace| workspace.id == id);
        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM workspace WHERE id = ?")
            .bind(&id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        if deleting_current {
            save_current_id(&mut transaction, None).await?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;

        self.workspaces
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())?
            .retain(|workspace| workspace.id != id);
        if deleting_current {
            *self
                .current
                .write()
                .map_err(|_| "Workspace lock is poisoned".to_string())? = None;
        }
        Ok(())
    }
}

fn validate_root(root: &Path) -> Result<(), String> {
    if !root.is_absolute() {
        return Err("Workspace root must be an absolute path".to_string());
    }
    if !root.is_dir() {
        return Err("Workspace root does not exist or is not a directory".to_string());
    }
    Ok(())
}

fn workspace_name(root: &Path) -> String {
    root.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| root.display().to_string())
}

async fn init_schema(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS workspace (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            root TEXT NOT NULL UNIQUE,
            description TEXT,
            created_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS workspace_state (
            singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
            current_workspace_id TEXT
        )",
    )
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;

    // V1 used UUID identifiers. Preserve existing rows while changing the
    // stable identity to the workspace root path.
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    sqlx::query(
        "UPDATE workspace_state
         SET current_workspace_id = (
             SELECT root FROM workspace WHERE id = workspace_state.current_workspace_id
         )
         WHERE current_workspace_id IS NOT NULL
           AND EXISTS (SELECT 1 FROM workspace WHERE id = workspace_state.current_workspace_id)",
    )
    .execute(&mut *transaction)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query("UPDATE workspace SET id = root WHERE id <> root")
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn load_state(pool: &SqlitePool) -> Result<(Vec<Workspace>, Option<Workspace>), String> {
    let rows = sqlx::query("SELECT id, root, created_at FROM workspace ORDER BY created_at ASC")
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    let workspaces = rows
        .into_iter()
        .map(workspace_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    let current_id =
        sqlx::query("SELECT current_workspace_id FROM workspace_state WHERE singleton_id = 1")
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?
            .and_then(|row| row.get::<Option<String>, _>("current_workspace_id"));
    let current = current_id.and_then(|id| {
        workspaces
            .iter()
            .find(|workspace| workspace.id == id)
            .cloned()
    });
    Ok((workspaces, current))
}

fn workspace_from_row(row: sqlx::sqlite::SqliteRow) -> Result<Workspace, String> {
    let id = row.get::<String, _>("id");
    let root = PathBuf::from(row.get::<String, _>("root"));
    let created_at = DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))
        .map_err(|error| error.to_string())?
        .with_timezone(&Utc);
    Ok(Workspace {
        id,
        name: workspace_name(&root),
        root,
        description: None,
        created_at,
    })
}

async fn save_current_id(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: Option<WorkspaceId>,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO workspace_state (singleton_id, current_workspace_id) VALUES (1, ?)
         ON CONFLICT(singleton_id) DO UPDATE SET current_workspace_id = excluded.current_workspace_id",
    )
    .bind(id)
    .execute(&mut **transaction)
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn map_workspace_write_error(error: sqlx::Error) -> String {
    let message = error.to_string();
    if message.contains("UNIQUE constraint failed: workspace.root") {
        "A workspace with this root already exists".to_string()
    } else {
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn manager() -> WorkspaceManager {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_schema(&pool).await.unwrap();
        WorkspaceManager {
            current: Arc::new(RwLock::new(None)),
            workspaces: Arc::new(RwLock::new(Vec::new())),
            db_pool: pool,
        }
    }

    #[tokio::test]
    async fn create_switch_and_delete_keep_current_state_consistent() {
        let manager = manager().await;
        let first = manager.create(std::env::temp_dir()).await.unwrap();
        assert_eq!(manager.current().unwrap().id, first.id);

        let second = manager
            .create(std::env::current_dir().unwrap())
            .await
            .unwrap();
        assert_eq!(manager.list().len(), 2);
        assert_eq!(manager.current().unwrap().id, first.id);

        assert_eq!(second.id, second.root.display().to_string());
        assert_eq!(
            second.name,
            second.root.file_name().unwrap().to_string_lossy()
        );

        manager.switch(second.id.clone()).await.unwrap();
        assert_eq!(manager.current().unwrap().id, second.id);
        let (_, persisted_current) = load_state(&manager.db_pool).await.unwrap();
        assert_eq!(persisted_current.unwrap().id, second.id);

        manager.clear_current().await.unwrap();
        assert!(manager.current().is_none());
        let (_, persisted_current) = load_state(&manager.db_pool).await.unwrap();
        assert!(persisted_current.is_none());

        manager.switch(second.id.clone()).await.unwrap();

        manager.delete(second.id).await.unwrap();
        assert!(manager.current().is_none());
        assert_eq!(manager.list(), vec![first]);
    }

    #[tokio::test]
    async fn migrates_legacy_uuid_ids_to_root_paths() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_schema(&pool).await.unwrap();
        let root = std::env::temp_dir().join("legacy-project");
        let root_string = root.display().to_string();
        sqlx::query(
            "INSERT INTO workspace (id, name, root, description, created_at)
             VALUES ('legacy-uuid', 'Custom Name', ?, 'Old note', '2026-01-01T00:00:00Z')",
        )
        .bind(&root_string)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO workspace_state (singleton_id, current_workspace_id)
             VALUES (1, 'legacy-uuid')",
        )
        .execute(&pool)
        .await
        .unwrap();

        init_schema(&pool).await.unwrap();
        let (workspaces, current) = load_state(&pool).await.unwrap();

        assert_eq!(workspaces[0].id, root_string);
        assert_eq!(workspaces[0].name, "legacy-project");
        assert!(workspaces[0].description.is_none());
        assert_eq!(current.unwrap().id, workspaces[0].id);
    }
}
