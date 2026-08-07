//! Aggregate journal tool failures into human-reviewable rule/Skill candidates.
//!
//! This is the lightest “trajectory → evolution” loop: mine recurring failure
//! fingerprints, draft a candidate Markdown file, and leave acceptance to a
//! human (no automatic prompt/skill write-back).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureCandidate {
    pub tool_name: String,
    pub error_fingerprint: String,
    pub count: u64,
    pub sample_message: String,
    pub suggested_rule: String,
    pub suggested_skill_draft: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureCandidateReport {
    pub candidates: Vec<FailureCandidate>,
    pub generated_at_ms: u64,
}

/// Normalize noisy tool error text into a stable fingerprint for aggregation.
pub fn fingerprint_error(message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    let mut out = String::new();
    let mut prev_space = false;
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/' | '\\') {
            out.push(ch);
            prev_space = false;
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }
    // Drop volatile tokens (uuids / long hex / absolute windows paths stay truncated).
    let tokens: Vec<&str> = out
        .split_whitespace()
        .filter(|token| {
            !(token.len() >= 32 && token.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))
        })
        .take(24)
        .collect();
    let joined = tokens.join(" ");
    if joined.chars().count() > 160 {
        joined.chars().take(160).collect()
    } else {
        joined
    }
}

pub async fn mine_failure_candidates(
    pool: &SqlitePool,
    min_count: u64,
) -> Result<FailureCandidateReport, String> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        r#"
        SELECT kind, payload_json, message_id
        FROM chat_journal_events
        WHERE kind IN ('tool_error', 'tool_result')
        ORDER BY seq DESC
        LIMIT 5000
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut counts: HashMap<(String, String), (u64, String)> = HashMap::new();
    for (kind, payload, _message_id) in rows {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&payload) else {
            continue;
        };
        let tool_name = value
            .get("tool")
            .or_else(|| value.get("name"))
            .or_else(|| value.get("toolName"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let message = value
            .get("error")
            .or_else(|| value.get("message"))
            .or_else(|| value.get("content"))
            .or_else(|| value.get("result"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let looks_like_error = kind == "tool_error"
            || message.to_ascii_lowercase().contains("error")
            || message.to_ascii_lowercase().contains("failed")
            || message.to_ascii_lowercase().contains("denied");
        if !looks_like_error || tool_name.is_empty() || message.trim().is_empty() {
            continue;
        }
        let fp = fingerprint_error(&message);
        if fp.is_empty() {
            continue;
        }
        let entry = counts.entry((tool_name, fp)).or_insert((0, message));
        entry.0 += 1;
    }

    let mut candidates: Vec<FailureCandidate> = counts
        .into_iter()
        .filter(|(_, (count, _))| *count >= min_count)
        .map(|((tool_name, error_fingerprint), (count, sample_message))| {
            let suggested_rule = format!(
                "- When `{tool_name}` fails like `{error_fingerprint}`, do not retry the same arguments; change strategy or ask the user."
            );
            let suggested_skill_draft = format!(
                "# Skill candidate: recover-from-{tool_name}-failure\n\n## Trigger\n`{tool_name}` error fingerprint: `{error_fingerprint}`\n\n## Steps\n1. Read the error carefully.\n2. Do not repeat the identical tool call.\n3. Gather missing context, then retry with a different approach.\n\n## Sample\n```\n{sample_message}\n```\n"
            );
            FailureCandidate {
                tool_name,
                error_fingerprint,
                count,
                sample_message,
                suggested_rule,
                suggested_skill_draft,
            }
        })
        .collect();
    candidates.sort_by(|a, b| b.count.cmp(&a.count).then(a.tool_name.cmp(&b.tool_name)));

    Ok(FailureCandidateReport {
        candidates,
        generated_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    })
}

/// Write a reviewable Markdown report under `.anya/candidates/` (does not install skills).
pub fn write_candidate_report(
    workspace: &Path,
    report: &FailureCandidateReport,
) -> Result<PathBuf, String> {
    let dir = workspace.join(".anya").join("candidates");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("failures-{}.md", report.generated_at_ms));
    let mut body = String::from(
        "# Failure → Rule/Skill candidates\n\nHuman review required. Nothing below is auto-installed.\n\n",
    );
    for (idx, candidate) in report.candidates.iter().enumerate() {
        body.push_str(&format!(
            "## {}. `{}` ×{}\n\nFingerprint: `{}`\n\nSuggested rule:\n{}\n\nSkill draft:\n\n{}\n",
            idx + 1,
            candidate.tool_name,
            candidate.count,
            candidate.error_fingerprint,
            candidate.suggested_rule,
            candidate.suggested_skill_draft
        ));
    }
    if report.candidates.is_empty() {
        body.push_str("_No recurring failures above threshold._\n");
    }
    std::fs::write(&path, body).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_strip_volatile_ids() {
        let a = fingerprint_error("tool failed uuid 123e4567-e89b-12d3-a456-426614174000 path");
        let b = fingerprint_error("Tool Failed UUID 999e4567-e89b-12d3-a456-426614174999 path");
        assert_eq!(a, b);
    }
}
