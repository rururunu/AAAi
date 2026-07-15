use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::core::tools::error::ToolError;
use crate::core::tools::preview::ToolPreview;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSnap {
    pub path: String,
    /// None means the file did not exist (restore deletes it).
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    pub turn: usize,
    pub time: u64,
    pub prompt: String,
    pub files: Vec<FileSnap>,
    #[serde(default)]
    pub user_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CheckpointIndex {
    checkpoints: Vec<Checkpoint>,
}

struct ActiveTurn {
    turn: usize,
    prompt: String,
    user_message_id: Option<String>,
    snapped: HashMap<String, FileSnap>,
}

pub struct CheckpointStore {
    root: PathBuf,
    active: Mutex<HashMap<String, ActiveTurn>>,
}

impl CheckpointStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            active: Mutex::new(HashMap::new()),
        }
    }

    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.root.join(session_id)
    }

    fn index_path(&self, session_id: &str) -> PathBuf {
        self.session_dir(session_id).join("index.json")
    }

    pub fn begin_turn(
        &self,
        session_id: &str,
        turn: usize,
        prompt: &str,
        user_message_id: Option<String>,
    ) {
        if let Ok(mut active) = self.active.lock() {
            active.insert(
                session_id.to_string(),
                ActiveTurn {
                    turn,
                    prompt: prompt.to_string(),
                    user_message_id,
                    snapped: HashMap::new(),
                },
            );
        }
    }

    pub fn snapshot_preview(
        &self,
        session_id: &str,
        workspace_root: &Path,
        preview: &ToolPreview,
    ) -> Result<(), ToolError> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| ToolError::new("checkpoint lock poisoned"))?;
        let Some(turn) = active.get_mut(session_id) else {
            return Ok(());
        };
        if turn.snapped.contains_key(&preview.path) {
            return Ok(());
        }
        let abs = workspace_root.join(&preview.path);
        let content = if abs.exists() {
            Some(fs::read_to_string(&abs)?)
        } else {
            None
        };
        turn.snapped.insert(
            preview.path.clone(),
            FileSnap {
                path: preview.path.clone(),
                content,
            },
        );
        Ok(())
    }

    pub fn finish_turn(&self, session_id: &str) -> Result<(), ToolError> {
        let finished = {
            let mut active = self
                .active
                .lock()
                .map_err(|_| ToolError::new("checkpoint lock poisoned"))?;
            active.remove(session_id)
        };
        let Some(turn) = finished else {
            return Ok(());
        };
        // Always persist a checkpoint when we have a user message id so conversation
        // rewind stays available even for turns that did not mutate files.
        if turn.snapped.is_empty() && turn.user_message_id.is_none() {
            return Ok(());
        }
        let dir = self.session_dir(session_id);
        fs::create_dir_all(&dir)?;
        let mut index = self.load_index(session_id)?;
        let checkpoint = Checkpoint {
            turn: turn.turn,
            time: now_secs(),
            prompt: turn.prompt,
            files: turn.snapped.into_values().collect(),
            user_message_id: turn.user_message_id,
        };
        index.checkpoints.retain(|c| c.turn != checkpoint.turn);
        index.checkpoints.push(checkpoint);
        index.checkpoints.sort_by_key(|c| c.turn);
        fs::write(
            self.index_path(session_id),
            serde_json::to_string_pretty(&index)?,
        )?;
        Ok(())
    }

    pub fn list(&self, session_id: &str) -> Result<Vec<Checkpoint>, ToolError> {
        Ok(self.load_index(session_id)?.checkpoints)
    }

    pub fn restore_code(
        &self,
        session_id: &str,
        turn: usize,
        workspace_root: &Path,
    ) -> Result<usize, ToolError> {
        let index = self.load_index(session_id)?;
        let Some(checkpoint) = index.checkpoints.iter().find(|c| c.turn == turn) else {
            return Err(ToolError::new(format!("checkpoint turn {turn} not found")));
        };
        let mut restored = 0usize;
        for snap in &checkpoint.files {
            let abs = workspace_root.join(&snap.path);
            match &snap.content {
                None => {
                    if abs.exists() {
                        fs::remove_file(&abs)?;
                        restored += 1;
                    }
                }
                Some(content) => {
                    if let Some(parent) = abs.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&abs, content)?;
                    restored += 1;
                }
            }
        }
        Ok(restored)
    }

    /// Drop the checkpoint for `turn` and all later turns (after conversation rewind).
    pub fn drop_from_turn(&self, session_id: &str, turn: usize) -> Result<(), ToolError> {
        let mut index = self.load_index(session_id)?;
        index.checkpoints.retain(|c| c.turn < turn);
        let dir = self.session_dir(session_id);
        fs::create_dir_all(&dir)?;
        fs::write(
            self.index_path(session_id),
            serde_json::to_string_pretty(&index)?,
        )?;
        Ok(())
    }

    fn load_index(&self, session_id: &str) -> Result<CheckpointIndex, ToolError> {
        let path = self.index_path(session_id);
        if !path.exists() {
            return Ok(CheckpointIndex::default());
        }
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn shared_checkpoint_store() -> &'static CheckpointStore {
    static STORE: OnceLock<CheckpointStore> = OnceLock::new();
    STORE.get_or_init(|| {
        let root = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("peek")
            .join("checkpoints");
        let _ = fs::create_dir_all(&root);
        CheckpointStore::new(root)
    })
}
