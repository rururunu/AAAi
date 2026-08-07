//! Lightweight workspace index for coding-agent retrieval (symbols + paths + docs).
//! Persisted under `<workspace>/.anya/index/index.sqlite`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".anya",
    ".cursor",
    "vendor",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexHit {
    pub kind: String,
    pub path: String,
    pub symbol: Option<String>,
    pub snippet: String,
    pub score: i32,
}

pub struct WorkspaceIndex {
    root: PathBuf,
    db_path: PathBuf,
}

impl WorkspaceIndex {
    pub fn open(workspace: &Path) -> Result<Self, String> {
        let dir = workspace.join(".anya").join("index");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let db_path = dir.join("index.jsonl");
        Ok(Self {
            root: workspace.to_path_buf(),
            db_path,
        })
    }

    pub fn rebuild(&self) -> Result<usize, String> {
        let mut records = Vec::new();
        let symbol_re = Regex::new(
            r"(?m)^\s*(?:pub\s+)?(?:async\s+)?(?:fn|struct|enum|trait|class|function|def|interface|type)\s+([A-Za-z_][A-Za-z0-9_]*)",
        )
        .map_err(|e| e.to_string())?;

        for entry in WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !(e.file_type().is_dir() && SKIP_DIRS.iter().any(|s| *s == name))
            })
        {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(&self.root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            if !is_indexable(&rel) {
                continue;
            }
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };
            if content.len() > 512 * 1024 {
                continue;
            }
            records.push(serde_json::json!({
                "kind": "file",
                "path": rel,
                "symbol": serde_json::Value::Null,
                "snippet": first_line_snippet(&content),
            }));
            for cap in symbol_re.captures_iter(&content).take(80) {
                let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                if name.is_empty() {
                    continue;
                }
                records.push(serde_json::json!({
                    "kind": "symbol",
                    "path": rel,
                    "symbol": name,
                    "snippet": cap.get(0).map(|m| m.as_str()).unwrap_or("").trim(),
                }));
            }
            if is_decision_doc(&rel) {
                records.push(serde_json::json!({
                    "kind": "decision",
                    "path": rel,
                    "symbol": serde_json::Value::Null,
                    "snippet": first_line_snippet(&content),
                }));
            }
        }

        let mut out = String::new();
        out.push_str(&format!(
            "{}\n",
            serde_json::json!({
                "kind": "meta",
                "builtAtMs": now_ms(),
                "count": records.len(),
            })
        ));
        for record in &records {
            out.push_str(&format!("{record}\n"));
        }
        fs::write(&self.db_path, out).map_err(|e| e.to_string())?;
        Ok(records.len())
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<IndexHit>, String> {
        if !self.db_path.exists() {
            self.rebuild()?;
        }
        let raw = fs::read_to_string(&self.db_path).map_err(|e| e.to_string())?;
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let terms: Vec<&str> = q.split_whitespace().collect();
        let mut hits = Vec::new();
        for line in raw.lines().skip(1) {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let kind = value
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("file")
                .to_string();
            if kind == "meta" {
                continue;
            }
            let path = value
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let symbol = value
                .get("symbol")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let snippet = value
                .get("snippet")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let hay = format!(
                "{} {} {}",
                path.to_ascii_lowercase(),
                symbol.as_deref().unwrap_or("").to_ascii_lowercase(),
                snippet.to_ascii_lowercase()
            );
            let mut score = 0;
            for term in &terms {
                if hay.contains(term) {
                    score += 10;
                }
                if symbol
                    .as_deref()
                    .is_some_and(|s| s.eq_ignore_ascii_case(term))
                {
                    score += 40;
                }
                if path.to_ascii_lowercase().contains(term) {
                    score += 15;
                }
            }
            if score > 0 {
                hits.push(IndexHit {
                    kind,
                    path,
                    symbol,
                    snippet,
                    score,
                });
            }
        }
        hits.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));
        hits.truncate(limit.max(1));
        Ok(hits)
    }
}

fn is_indexable(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.ends_with(".rs")
        || lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".js")
        || lower.ends_with(".jsx")
        || lower.ends_with(".py")
        || lower.ends_with(".go")
        || lower.ends_with(".java")
        || lower.ends_with(".md")
        || lower.ends_with(".toml")
        || lower.ends_with(".json")
}

fn is_decision_doc(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.ends_with("agents.md")
        || lower.contains("/adr/")
        || lower.contains("architecture")
        || lower.ends_with("decisions.md")
}

fn first_line_snippet(content: &str) -> String {
    content
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .chars()
        .take(160)
        .collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_and_finds_symbols() {
        let root = std::env::temp_dir().join(format!("anya-index-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn hello_world() {}\nfn other() {}\n",
        )
        .unwrap();
        fs::write(root.join("AGENTS.md"), "# Agents\nUse pnpm.\n").unwrap();

        let index = WorkspaceIndex::open(&root).unwrap();
        let count = index.rebuild().unwrap();
        assert!(count >= 2);
        let hits = index.search("hello_world", 5).unwrap();
        assert!(hits
            .iter()
            .any(|h| h.symbol.as_deref() == Some("hello_world")));
        let docs = index.search("agents", 5).unwrap();
        assert!(docs.iter().any(|h| h.kind == "decision"));
        let _ = fs::remove_dir_all(root);
    }
}
