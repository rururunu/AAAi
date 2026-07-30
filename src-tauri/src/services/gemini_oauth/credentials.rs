use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tauri::{AppHandle, Manager};

const OAUTH_LOCAL_FILE_NAMES: &[&str] = &[
    "agy-oauth.local.json",
    "google-oauth.local.json",
    "client_secret.local.json",
];

#[derive(Debug, Clone)]
pub(super) struct OAuthCredentials {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Deserialize)]
struct OAuthCredentialsFile {
    #[serde(default)]
    #[serde(alias = "clientId")]
    client_id: String,
    #[serde(default)]
    #[serde(alias = "clientSecret")]
    client_secret: String,
    #[serde(default)]
    installed: Option<OAuthCredentialsBlock>,
    #[serde(default)]
    web: Option<OAuthCredentialsBlock>,
}

#[derive(Debug, Deserialize)]
struct OAuthCredentialsBlock {
    #[serde(default)]
    #[serde(alias = "clientId")]
    client_id: String,
    #[serde(default)]
    #[serde(alias = "clientSecret")]
    client_secret: String,
}

pub(super) fn load_oauth_credentials(app: &AppHandle) -> Result<OAuthCredentials, String> {
    let env_client_id = std::env::var("AAAI_AGY_OAUTH_CLIENT_ID")
        .or_else(|_| std::env::var("AGY_OAUTH_CLIENT_ID"))
        .unwrap_or_default();
    let env_client_secret = std::env::var("AAAI_AGY_OAUTH_CLIENT_SECRET")
        .or_else(|_| std::env::var("AGY_OAUTH_CLIENT_SECRET"))
        .unwrap_or_default();
    if let Some(credentials) = normalize_oauth_credentials(env_client_id, env_client_secret) {
        return Ok(credentials);
    }

    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::current_dir() {
        push_oauth_candidates(&mut candidates, dir);
    }
    if let Ok(dir) = app.path().app_config_dir() {
        push_oauth_candidates(&mut candidates, dir);
    }

    let mut seen = Vec::<PathBuf>::new();
    for path in candidates {
        if seen.iter().any(|seen_path| seen_path == &path) {
            continue;
        }
        seen.push(path.clone());
        if path.is_file() {
            return parse_oauth_credentials_file(&path);
        }
    }

    let searched = seen
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "Missing Antigravity OAuth credentials. Create agy-oauth.local.json with client_id and client_secret, or set AAAI_AGY_OAUTH_CLIENT_ID / AAAI_AGY_OAUTH_CLIENT_SECRET. Searched: {searched}"
    ))
}

fn push_oauth_candidates(candidates: &mut Vec<PathBuf>, dir: PathBuf) {
    for name in OAUTH_LOCAL_FILE_NAMES {
        candidates.push(dir.join(name));
    }
    for name in OAUTH_LOCAL_FILE_NAMES {
        candidates.push(dir.join("src-tauri").join(name));
    }
}

pub(super) fn parse_oauth_credentials_file(path: &Path) -> Result<OAuthCredentials, String> {
    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read OAuth credentials at {}: {error}",
            path.display()
        )
    })?;
    parse_oauth_credentials_json(&raw).ok_or_else(|| {
        format!(
            "OAuth credentials at {} are missing client_id/client_secret",
            path.display()
        )
    })
}

fn parse_oauth_credentials_json(raw: &str) -> Option<OAuthCredentials> {
    let parsed: OAuthCredentialsFile = serde_json::from_str(raw).ok()?;
    normalize_oauth_credentials(parsed.client_id, parsed.client_secret)
        .or_else(|| {
            parsed
                .installed
                .and_then(|block| normalize_oauth_credentials(block.client_id, block.client_secret))
        })
        .or_else(|| {
            parsed
                .web
                .and_then(|block| normalize_oauth_credentials(block.client_id, block.client_secret))
        })
}

fn normalize_oauth_credentials(
    client_id: String,
    client_secret: String,
) -> Option<OAuthCredentials> {
    let client_id = client_id.trim().to_string();
    let client_secret = client_secret.trim().to_string();
    if client_id.is_empty() || client_secret.is_empty() {
        return None;
    }
    Some(OAuthCredentials {
        client_id,
        client_secret,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_oauth_credentials_json;

    #[test]
    fn parses_flat_local_credentials() {
        let credentials = parse_oauth_credentials_json(
            r#"{"client_id":"local-client","client_secret":"local-secret"}"#,
        )
        .expect("credentials");

        assert_eq!(credentials.client_id, "local-client");
        assert_eq!(credentials.client_secret, "local-secret");
    }

    #[test]
    fn parses_google_installed_credentials() {
        let credentials = parse_oauth_credentials_json(
            r#"{"installed":{"client_id":"google-client","client_secret":"google-secret"}}"#,
        )
        .expect("credentials");

        assert_eq!(credentials.client_id, "google-client");
        assert_eq!(credentials.client_secret, "google-secret");
    }

    #[test]
    fn rejects_incomplete_credentials() {
        assert!(parse_oauth_credentials_json(r#"{"client_id":"only-client"}"#).is_none());
    }
}
