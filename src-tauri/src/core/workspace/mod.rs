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
    pub source: Option<String>,
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
            source: None,
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

    pub async fn remember_from_ide(
        &self,
        root: PathBuf,
        ide: &str,
    ) -> Result<(Workspace, bool), String> {
        validate_root(&root)?;
        let source = normalize_ide_source(ide)
            .ok_or_else(|| "IDE workspace source must not be empty".to_string())?;
        let id = root.to_string_lossy().to_string();

        if let Some(mut existing) = self.list().into_iter().find(|workspace| workspace.id == id) {
            if existing.source.as_deref() == Some(source.as_str()) {
                return Ok((existing, false));
            }

            sqlx::query("UPDATE workspace SET source = ? WHERE id = ?")
                .bind(&source)
                .bind(&id)
                .execute(&self.db_pool)
                .await
                .map_err(|error| error.to_string())?;

            existing.source = Some(source);
            let mut workspaces = self
                .workspaces
                .write()
                .map_err(|_| "Workspace lock is poisoned".to_string())?;
            if let Some(workspace) = workspaces.iter_mut().find(|workspace| workspace.id == id) {
                *workspace = existing.clone();
            }
            drop(workspaces);
            if self.current().is_some_and(|workspace| workspace.id == id) {
                *self
                    .current
                    .write()
                    .map_err(|_| "Workspace lock is poisoned".to_string())? =
                    Some(existing.clone());
            }
            return Ok((existing, true));
        }

        let workspace = Workspace {
            id,
            name: workspace_name(&root),
            root,
            description: None,
            source: Some(source),
            created_at: Utc::now(),
        };
        sqlx::query(
            "INSERT INTO workspace (id, name, root, description, source, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&workspace.id)
        .bind(&workspace.name)
        .bind(workspace.root.to_string_lossy().to_string())
        .bind(&workspace.description)
        .bind(&workspace.source)
        .bind(workspace.created_at.to_rfc3339())
        .execute(&self.db_pool)
        .await
        .map_err(map_workspace_write_error)?;

        self.workspaces
            .write()
            .map_err(|_| "Workspace lock is poisoned".to_string())?
            .push(workspace.clone());
        Ok((workspace, true))
    }

    pub async fn select_known_ide_workspace(
        &self,
        root: &Path,
    ) -> Result<Option<Workspace>, String> {
        let Some(workspace) = self.list().into_iter().find(|workspace| {
            workspace.source.is_some() && workspace_roots_equal(&workspace.root, root)
        }) else {
            return Ok(None);
        };
        if self
            .current()
            .is_some_and(|current| current.id == workspace.id)
        {
            return Ok(None);
        }
        self.switch(workspace.id.clone()).await.map(Some)
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

fn normalize_ide_source(ide: &str) -> Option<String> {
    let ide = ide.trim();
    if ide.is_empty() {
        return None;
    }
    Some(match ide.to_ascii_lowercase().as_str() {
        "visual studio code" | "vs code" | "vscode" => "vscode".to_string(),
        "idea" | "intellij" | "intellij idea" => "idea".to_string(),
        _ => ide.chars().take(64).collect(),
    })
}

fn workspace_roots_equal(left: &Path, right: &Path) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    if cfg!(windows) {
        left.to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .eq_ignore_ascii_case(right.to_string_lossy().trim_end_matches(['\\', '/']))
    } else {
        left == right
    }
}

async fn init_schema(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS workspace (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            root TEXT NOT NULL UNIQUE,
            description TEXT,
            source TEXT,
            created_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;

    let columns = sqlx::query("PRAGMA table_info(workspace)")
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    if !columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "source")
    {
        sqlx::query("ALTER TABLE workspace ADD COLUMN source TEXT")
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
    }
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
    let rows =
        sqlx::query("SELECT id, root, source, created_at FROM workspace ORDER BY created_at ASC")
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
        source: row.get::<Option<String>, _>("source"),
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
        assert!(workspaces[0].source.is_none());
        assert_eq!(current.unwrap().id, workspaces[0].id);
    }

    #[tokio::test]
    async fn adds_source_column_to_existing_workspace_database() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE workspace (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                root TEXT NOT NULL UNIQUE,
                description TEXT,
                created_at TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        init_schema(&pool).await.unwrap();

        let columns = sqlx::query("PRAGMA table_info(workspace)")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(columns
            .iter()
            .any(|row| row.get::<String, _>("name") == "source"));
    }

    #[tokio::test]
    async fn remembers_ide_workspace_source_without_switching_current_workspace() {
        let manager = manager().await;
        let current = manager.create(std::env::temp_dir()).await.unwrap();
        let ide_root = std::env::current_dir().unwrap();

        let (remembered, changed) = manager
            .remember_from_ide(ide_root.clone(), "visual studio code")
            .await
            .unwrap();

        assert!(changed);
        assert_eq!(remembered.root, ide_root);
        assert_eq!(remembered.source.as_deref(), Some("vscode"));
        assert_eq!(manager.current().unwrap().id, current.id);

        let (persisted, persisted_current) = load_state(&manager.db_pool).await.unwrap();
        assert_eq!(
            persisted
                .iter()
                .find(|workspace| workspace.id == remembered.id)
                .and_then(|workspace| workspace.source.as_deref()),
            Some("vscode")
        );
        assert_eq!(persisted_current.unwrap().id, current.id);

        let (_, changed_again) = manager.remember_from_ide(ide_root, "vscode").await.unwrap();
        assert!(!changed_again);
    }

    #[tokio::test]
    async fn selects_a_previously_remembered_ide_workspace_on_report() {
        let manager = manager().await;
        let current = manager.create(std::env::temp_dir()).await.unwrap();
        let ide_root = std::env::current_dir().unwrap();
        let (remembered, _) = manager
            .remember_from_ide(ide_root.clone(), "idea")
            .await
            .unwrap();
        assert_eq!(manager.current().unwrap().id, current.id);

        let selected = manager
            .select_known_ide_workspace(&ide_root)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(selected.id, remembered.id);
        assert_eq!(manager.current().unwrap().id, remembered.id);
        assert!(manager
            .select_known_ide_workspace(&ide_root)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn does_not_auto_select_a_manually_added_workspace() {
        let manager = manager().await;
        let current = manager.create(std::env::temp_dir()).await.unwrap();
        let manual_root = std::env::current_dir().unwrap();
        let manual = manager.create(manual_root.clone()).await.unwrap();
        assert!(manual.source.is_none());

        assert!(manager
            .select_known_ide_workspace(&manual_root)
            .await
            .unwrap()
            .is_none());
        assert_eq!(manager.current().unwrap().id, current.id);
    }
}
