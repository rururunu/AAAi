//! Antigravity (agy) Google OAuth2 — same Desktop client / scopes / Cloud Code
//! project bootstrap as jcode, so Gemini rides the free Code Assist quota.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

use crate::models::chat::ChatModelInfo;
use crate::services::settings_store::{get_settings, set_settings};

mod catalog;
mod credentials;

pub use catalog::resolve_antigravity_model_id;
use catalog::{fallback_chat_model_infos, fetch_available_models, to_chat_model_infos};
use credentials::{load_oauth_credentials, parse_oauth_credentials_file};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v1/userinfo?alt=json";
const GOOGLE_OAUTH_USER_AGENT: &str = "google-api-nodejs-client/9.15.1";

const ANTIGRAVITY_VERSION: &str = "1.18.3";
const ANTIGRAVITY_SCOPES: &str = "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/experimentsandconfigs";

const DEFAULT_CALLBACK_PORT: u16 = 51121;
const REDIRECT_PATH: &str = "/oauth-callback";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(180);
const EXPIRY_SKEW_SECS: i64 = 60;

const PRODUCTION_ENDPOINT: &str = "https://cloudcode-pa.googleapis.com";

/// Used only by `loadCodeAssist` to bootstrap the GCP project id.
const LOAD_ENDPOINTS: &[&str] = &[
    PRODUCTION_ENDPOINT,
    "https://daily-cloudcode-pa.sandbox.googleapis.com",
    "https://autopush-cloudcode-pa.sandbox.googleapis.com",
];

pub const X_GOOG_API_CLIENT: &str = "google-cloud-sdk vscode_cloudshelleditor/0.1";

const ANTIGRAVITY_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const ANTIGRAVITY_REQUEST_TIMEOUT: Duration = Duration::from_secs(180);

pub const MAX_ANTIGRAVITY_RETRIES: u32 = 2;
pub const ANTIGRAVITY_RETRY_BACKOFF: Duration = Duration::from_secs(1);

/// Build an HTTP client for Antigravity API calls (system proxy + timeouts).
/// Uses native TLS + HTTP/1.1 for parity with agycli/Go on Windows and proxy environments.
pub fn antigravity_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(ANTIGRAVITY_CONNECT_TIMEOUT)
        .timeout(ANTIGRAVITY_REQUEST_TIMEOUT)
        .http1_only()
        .build()
        .map_err(|error| format!("Failed to create Antigravity HTTP client: {error}"))
}

/// Production-only URL for chat/model APIs (never use sandbox — it needs Staging API enabled).
pub fn antigravity_production_api_url(path: &str) -> String {
    format!("{PRODUCTION_ENDPOINT}/{path}")
}

pub fn is_retryable_antigravity_status(status: reqwest::StatusCode) -> bool {
    status.as_u16() == 429
        || status.as_u16() == 500
        || status.as_u16() == 502
        || status.as_u16() == 503
        || status.as_u16() == 504
}

fn format_reqwest_error_chain(error: &reqwest::Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = std::error::Error::source(error);
    while let Some(err) = source {
        let text = err.to_string();
        if parts.last().is_none_or(|last| last != &text) {
            parts.push(text);
        }
        source = err.source();
    }
    parts.join(" | ")
}

pub fn antigravity_transport_error_message(error: &reqwest::Error) -> String {
    let detail = format_reqwest_error_chain(error);
    let lower = detail.to_lowercase();

    let reason = if lower.contains("timed out") || lower.contains("timeout") {
        "Connection timed out: Google Antigravity did not respond. Generation may take longer, or the network/proxy is too slow."
    } else if lower.contains("dns")
        || lower.contains("name resolution")
        || lower.contains("no such host")
    {
        "DNS resolution failed: could not resolve cloudcode-pa.googleapis.com. Check network or DNS settings."
    } else if lower.contains("handshake eof")
        || lower.contains("unexpected eof")
        || lower.contains("connection reset")
        || lower.contains("forcibly closed")
    {
        "TLS handshake interrupted: the HTTPS connection was reset by the peer or an intermediary. Common with proxies that do not cover the desktop app, Clash fake-ip mismatches, or direct Google resets. Use TUN/system proxy mode and include AAAi in proxy rules; do not route googleapis.com via DIRECT."
    } else if lower.contains("certificate") || lower.contains("invalid certificate") {
        "TLS certificate verification failed: check system time, or whether a proxy MITMs HTTPS without a trusted certificate."
    } else if lower.contains("tls") || lower.contains("ssl") {
        "TLS connection failed: check proxy and HTTPS settings."
    } else if lower.contains("connection refused") {
        "Connection refused: Google API is unreachable, or the local proxy port is not running."
    } else if lower.contains("error sending request") {
        "Could not connect to Google Antigravity (cloudcode-pa.googleapis.com). A successful OAuth login does not guarantee API reachability—confirm Google services are reachable and enable the system proxy (Clash/V2Ray, etc.); add AAAi to proxy rules and retry."
    } else {
        "Antigravity network request failed: check network, system proxy, and Gemini sign-in status."
    };

    format!("{reason} Details: {detail}")
}

pub fn antigravity_http_error_message(status: reqwest::StatusCode, body: &str) -> String {
    let code = status.as_u16();
    let lower = body.to_lowercase();
    let reason = match code {
        400 if lower.contains("user location is not supported")
            || lower.contains("location is not supported") =>
        {
            "Your network/account region is not supported by Google Gemini (Antigravity). Signing in to Google does not guarantee API access—use an endpoint in a supported region, or switch to another AI provider."
        }
        401 => "Gemini sign-in expired: please sign in again with your Google account in Settings.",
        403 if lower.contains("staging-cloudaicompanion") || lower.contains("(staging)") => {
            "Accidentally connected to the Google Staging environment (should not happen). Update to the latest version and retry; report if it persists."
        }
        403 if lower.contains("service_disabled") || lower.contains("has not been used in project") => {
            "Gemini API is not enabled for the current Google Cloud project. Sign in to Gemini again in Settings, or wait a few minutes and retry."
        }
        403 => "Permission denied: the current Google account or Cloud project cannot call Antigravity. Sign in to Gemini again.",
        404 if lower.contains("not_found") || lower.contains("was not found") => {
            "Model or resource not found: confirm the Gemini model name is correct and available for your account (for vision, prefer image-capable models such as gemini-3-flash)."
        }
        429 => "Rate limited or free quota exhausted: please retry later.",
        500..=599 => "Google Antigravity is temporarily unavailable: please retry later.",
        _ if status.is_client_error() => {
            "Request rejected by Google: check the selected model and account status."
        }
        _ => "Antigravity API call failed.",
    };

    let detail = body.trim();
    if detail.is_empty() {
        format!("Antigravity API returned {code}. {reason}")
    } else {
        let mut end = detail.len().min(320);
        while end > 0 && !detail.is_char_boundary(end) {
            end -= 1;
        }
        let truncated = if end >= detail.len() {
            detail.to_string()
        } else {
            format!("{}…", &detail[..end])
        };
        format!("Antigravity API returned {code}. {reason} Response: {truncated}")
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiAuthStatus {
    pub logged_in: bool,
    pub email: String,
    pub has_client_secret: bool,
    pub client_id: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    #[serde(default)]
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadCodeAssistResponse {
    #[serde(default)]
    cloudaicompanion_project: Option<serde_json::Value>,
}

pub fn antigravity_user_agent() -> String {
    if cfg!(target_os = "windows") {
        format!("antigravity/{ANTIGRAVITY_VERSION} windows/amd64")
    } else if cfg!(target_arch = "aarch64") {
        format!("antigravity/{ANTIGRAVITY_VERSION} darwin/arm64")
    } else {
        format!("antigravity/{ANTIGRAVITY_VERSION} darwin/amd64")
    }
}

pub fn client_metadata_header() -> String {
    let platform = if cfg!(target_os = "windows") {
        "WINDOWS"
    } else {
        "MACOS"
    };
    format!(r#"{{"ideType":"ANTIGRAVITY","platform":"{platform}","pluginType":"GEMINI"}}"#)
}

pub fn is_gemini_model(model: &str) -> bool {
    model.trim().to_ascii_lowercase().starts_with("gemini")
}

/// Whether this model should use Antigravity OAuth instead of OpenAI-compatible endpoints.
pub fn can_use_antigravity_for_model(
    settings: &crate::models::settings::AppSettings,
    model: &str,
) -> bool {
    is_gemini_model(model) && settings.gemini_oauth.is_logged_in()
}

pub fn auth_status(app: &AppHandle) -> Result<GeminiAuthStatus, String> {
    let settings = get_settings(app)?;
    let oauth = &settings.gemini_oauth;
    let credentials = load_oauth_credentials(app).ok();
    Ok(GeminiAuthStatus {
        logged_in: oauth.is_logged_in(),
        email: oauth.email.clone(),
        has_client_secret: credentials.is_some(),
        client_id: credentials
            .map(|credentials| credentials.client_id)
            .unwrap_or_else(|| oauth.client_id.clone()),
    })
}

pub fn logout(app: &AppHandle) -> Result<GeminiAuthStatus, String> {
    let mut settings = get_settings(app)?;
    settings.gemini_oauth.access_token.clear();
    settings.gemini_oauth.refresh_token.clear();
    settings.gemini_oauth.expires_at = 0;
    settings.gemini_oauth.email.clear();
    settings.gemini_oauth.project_id.clear();
    set_settings(app, settings)?;
    auth_status(app)
}

/// Kept for IPC compatibility; credentials are loaded from a local ignored file.
pub fn import_client_secrets(app: &AppHandle, path: &str) -> Result<GeminiAuthStatus, String> {
    let credentials = if path.trim().is_empty() {
        load_oauth_credentials(app)?
    } else {
        parse_oauth_credentials_file(Path::new(path.trim()))?
    };
    let mut settings = get_settings(app)?;
    settings.gemini_oauth.client_id = credentials.client_id;
    settings.gemini_oauth.client_secret.clear();
    set_settings(app, settings)?;
    auth_status(app)
}

/// Start the Antigravity Desktop OAuth loopback flow and persist tokens on success.
pub fn login(app: &AppHandle) -> Result<GeminiAuthStatus, String> {
    let credentials = load_oauth_credentials(app)?;
    let client_id = credentials.client_id.as_str();
    let client_secret = credentials.client_secret.as_str();

    let listener =
        TcpListener::bind(format!("127.0.0.1:{DEFAULT_CALLBACK_PORT}")).map_err(|e| {
            format!(
            "Failed to bind Antigravity callback port {DEFAULT_CALLBACK_PORT} (may be in use): {e}"
        )
        })?;
    listener
        .set_nonblocking(false)
        .map_err(|e| format!("Failed to configure callback listener: {e}"))?;
    let redirect_uri = format!("http://127.0.0.1:{DEFAULT_CALLBACK_PORT}{REDIRECT_PATH}");

    let state = Uuid::new_v4().to_string();
    let code_verifier = generate_code_verifier();
    let code_challenge = code_challenge_s256(&code_verifier);

    let auth_url = format!(
        "{AUTH_URL}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}&access_type=offline&prompt=consent",
        urlencoding::encode(client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(ANTIGRAVITY_SCOPES),
        urlencoding::encode(&code_challenge),
        urlencoding::encode(&state),
    );

    app.opener()
        .open_url(&auth_url, None::<&str>)
        .map_err(|e| format!("Failed to open browser for sign-in: {e}"))?;

    let code = wait_for_auth_code(listener, &state)?;
    let tokens = exchange_code(
        client_id,
        client_secret,
        &redirect_uri,
        &code,
        &code_verifier,
    )?;

    let email = fetch_email(&tokens.access_token)
        .or_else(|| email_from_id_token(tokens.id_token.as_deref()))
        .unwrap_or_default();
    let project_id = fetch_project_id(&tokens.access_token).unwrap_or_default();

    let mut next = get_settings(app)?;
    next.gemini_oauth.client_id = client_id.to_string();
    next.gemini_oauth.client_secret.clear();
    next.gemini_oauth.access_token = tokens.access_token;
    if let Some(refresh) = tokens.refresh_token {
        if !refresh.is_empty() {
            next.gemini_oauth.refresh_token = refresh;
        }
    }
    next.gemini_oauth.expires_at = now_unix() + tokens.expires_in.unwrap_or(3600);
    next.gemini_oauth.email = email;
    next.gemini_oauth.project_id = project_id;
    set_settings(app, next)?;
    auth_status(app)
}

/// Return a valid access token, refreshing synchronously when needed.
///
/// Must not be called on a Tokio worker thread — `reqwest::blocking` owns a
/// runtime and dropping it inside async context panics. Prefer
/// [`ensure_access_token_async`] from async code.
pub fn ensure_access_token(app: &AppHandle) -> Result<String, String> {
    let settings = get_settings(app)?;
    let oauth = settings.gemini_oauth;
    if !oauth.access_token.trim().is_empty() && oauth.expires_at > now_unix() + EXPIRY_SKEW_SECS {
        return Ok(oauth.access_token);
    }
    if oauth.refresh_token.trim().is_empty() {
        return Err("Gemini (Antigravity) is not signed in. Please sign in with a Google account in Settings.".into());
    }

    let credentials = load_oauth_credentials(app)?;
    let tokens = refresh_access_token(
        credentials.client_id.as_str(),
        credentials.client_secret.as_str(),
        oauth.refresh_token.trim(),
    )?;

    let mut next = get_settings(app)?;
    next.gemini_oauth.access_token = tokens.access_token.clone();
    if let Some(refresh) = tokens.refresh_token {
        if !refresh.is_empty() {
            next.gemini_oauth.refresh_token = refresh;
        }
    }
    next.gemini_oauth.expires_at = now_unix() + tokens.expires_in.unwrap_or(3600);
    if next.gemini_oauth.project_id.trim().is_empty() {
        if let Ok(project_id) = fetch_project_id(&tokens.access_token) {
            next.gemini_oauth.project_id = project_id;
        }
    }
    set_settings(app, next)?;
    Ok(tokens.access_token)
}

/// Async-safe wrapper around [`ensure_access_token`].
pub async fn ensure_access_token_async(app: &AppHandle) -> Result<String, String> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || ensure_access_token(&app))
        .await
        .map_err(|error| format!("Failed to obtain access token: {error}"))?
}

/// Ensure `project_id` is present (loadCodeAssist), refreshing token if needed.
///
/// See [`ensure_access_token`] — call [`ensure_project_id_async`] from async code.
pub fn ensure_project_id(app: &AppHandle) -> Result<String, String> {
    let settings = get_settings(app)?;
    if !settings.gemini_oauth.project_id.trim().is_empty() {
        return Ok(settings.gemini_oauth.project_id.trim().to_string());
    }
    let access_token = ensure_access_token(app)?;
    let project_id = fetch_project_id(&access_token)?;
    let mut next = get_settings(app)?;
    next.gemini_oauth.project_id = project_id.clone();
    set_settings(app, next)?;
    Ok(project_id)
}

/// Async-safe wrapper around [`ensure_project_id`].
pub async fn ensure_project_id_async(app: &AppHandle) -> Result<String, String> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || ensure_project_id(&app))
        .await
        .map_err(|error| format!("Failed to resolve project: {error}"))?
}

/// Fetch Gemini models from Antigravity `fetchAvailableModels`.
pub async fn list_models(app: &AppHandle) -> Result<Vec<ChatModelInfo>, String> {
    let app_for_token = app.clone();
    let access_token = tokio::task::spawn_blocking(move || ensure_access_token(&app_for_token))
        .await
        .map_err(|error| format!("Failed to obtain access token: {error}"))??;

    let settings = get_settings(app)?;
    let stored_project = settings.gemini_oauth.project_id.trim().to_string();

    if !stored_project.is_empty() {
        match fetch_available_models(&access_token, Some(&stored_project)).await {
            Ok(models) if !models.is_empty() => return Ok(to_chat_model_infos(models)),
            Ok(_) | Err(_) => {}
        }
    }

    let app_for_project = app.clone();
    let access_token = tokio::task::spawn_blocking(move || {
        let project_id = ensure_project_id(&app_for_project)?;
        ensure_access_token(&app_for_project).map(|token| (token, project_id))
    })
    .await
    .map_err(|error| format!("Failed to resolve project: {error}"))??;

    if let Ok(models) = fetch_available_models(&access_token.0, Some(&access_token.1)).await {
        if !models.is_empty() {
            return Ok(to_chat_model_infos(models));
        }
    }

    if let Ok(models) = fetch_available_models(&access_token.0, None).await {
        if !models.is_empty() {
            return Ok(to_chat_model_infos(models));
        }
    }

    eprintln!("fetchAvailableModels returned no Gemini models; using fallback list");
    Ok(fallback_chat_model_infos())
}

fn wait_for_auth_code(listener: TcpListener, expected_state: &str) -> Result<String, String> {
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Failed to configure callback listener: {e}"))?;

    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    let expected = expected_state.to_string();

    thread::spawn(move || {
        let result = (|| {
            let deadline = std::time::Instant::now() + LOGIN_TIMEOUT;
            let (mut stream, _addr) = loop {
                match listener.accept() {
                    Ok(conn) => break conn,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if std::time::Instant::now() >= deadline {
                            return Err(
                                "Timed out waiting for Google sign-in. Please retry.".into()
                            );
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(error) => {
                        return Err(format!("Failed while waiting for Google callback: {error}"));
                    }
                }
            };
            stream
                .set_nonblocking(false)
                .map_err(|e| format!("Failed to configure callback connection: {e}"))?;
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .map_err(|e| format!("Failed to configure callback read timeout: {e}"))?;

            let request = read_http_request(&mut stream)?;
            let params = parse_query_params(&request);
            if let Some(error) = params.get("error") {
                let desc = params
                    .get("error_description")
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let _ = write_html_response(
                    &mut stream,
                    400,
                    &format!("<h2>Sign-in failed</h2><p>{error}: {desc}</p><p>You can close this window and return to AAAi.</p>"),
                );
                return Err(format!("Google denied authorization: {error} {desc}"));
            }
            let state = params.get("state").cloned().unwrap_or_default();
            if state != expected {
                let _ = write_html_response(
                    &mut stream,
                    400,
                    "<h2>Sign-in failed</h2><p>OAuth state mismatch. Please retry.</p>",
                );
                return Err("OAuth state mismatch. Please retry.".into());
            }
            let code = params
                .get("code")
                .cloned()
                .filter(|c| !c.is_empty())
                .ok_or_else(|| "Callback is missing the authorization code".to_string())?;
            let _ = write_html_response(
                &mut stream,
                200,
                "<h2>Sign-in successful</h2><p>You can close this window and return to AAAi.</p>",
            );
            Ok(code)
        })();
        let _ = tx.send(result);
    });

    rx.recv_timeout(LOGIN_TIMEOUT + Duration::from_secs(5))
        .map_err(|_| "Timed out waiting for Google sign-in. Please retry.".to_string())?
}

fn read_http_request(stream: &mut std::net::TcpStream) -> Result<String, String> {
    let mut buffer = [0u8; 8192];
    let n = stream
        .read(&mut buffer)
        .map_err(|e| format!("Failed to read callback request: {e}"))?;
    Ok(String::from_utf8_lossy(&buffer[..n]).into_owned())
}

fn parse_query_params(request: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");
    let query = path.split('?').nth(1).unwrap_or("");
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        if key.is_empty() {
            continue;
        }
        map.insert(
            urlencoding::decode(key)
                .unwrap_or_else(|_| key.into())
                .into_owned(),
            urlencoding::decode(value)
                .unwrap_or_else(|_| value.into())
                .into_owned(),
        );
    }
    map
}

fn write_html_response(
    stream: &mut std::net::TcpStream,
    status: u16,
    body: &str,
) -> Result<(), String> {
    let reason = if status == 200 { "OK" } else { "Bad Request" };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("Failed to write callback response: {e}"))?;
    let _ = stream.flush();
    Ok(())
}

fn exchange_code(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
    code_verifier: &str,
) -> Result<TokenResponse, String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(TOKEN_URL)
        .header(reqwest::header::USER_AGENT, GOOGLE_OAUTH_USER_AGENT)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("code_verifier", code_verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .map_err(|e| format!("Failed to exchange authorization code for token: {e}"))?;
    parse_token_response(response)
}

fn refresh_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<TokenResponse, String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(TOKEN_URL)
        .header(reqwest::header::USER_AGENT, GOOGLE_OAUTH_USER_AGENT)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .map_err(|e| format!("Failed to refresh Antigravity token: {e}"))?;
    parse_token_response(response)
}

fn parse_token_response(response: reqwest::blocking::Response) -> Result<TokenResponse, String> {
    let status = response.status();
    let text = response
        .text()
        .map_err(|e| format!("Failed to read token response: {e}"))?;
    if !status.is_success() {
        return Err(format!("Google token API error ({status}): {text}"));
    }
    serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse token response: {e}; body={text}"))
}

fn fetch_email(access_token: &str) -> Option<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(USERINFO_URL)
        .header(reqwest::header::USER_AGENT, GOOGLE_OAUTH_USER_AGENT)
        .bearer_auth(access_token)
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    response
        .json::<UserInfoResponse>()
        .ok()
        .and_then(|info| info.email)
}

fn fetch_project_id(access_token: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "metadata": {
            "ideType": "ANTIGRAVITY",
            "platform": "PLATFORM_UNSPECIFIED",
            "pluginType": "GEMINI"
        }
    });
    let mut errors = Vec::new();

    for base_url in LOAD_ENDPOINTS {
        let response = match client
            .post(format!("{base_url}/v1internal:loadCodeAssist"))
            .bearer_auth(access_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::USER_AGENT, antigravity_user_agent())
            .header("x-goog-api-client", X_GOOG_API_CLIENT)
            .header("Client-Metadata", client_metadata_header())
            .json(&body)
            .send()
        {
            Ok(resp) => resp,
            Err(err) => {
                errors.push(format!("{base_url}: {err}"));
                continue;
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            errors.push(format!("{base_url}: HTTP {status} {}", text.trim()));
            continue;
        }

        let parsed: LoadCodeAssistResponse = response
            .json()
            .map_err(|e| format!("Failed to parse loadCodeAssist response: {e}"))?;
        if let Some(project_id) = extract_project_id(parsed.cloudaicompanion_project) {
            return Ok(project_id);
        }
        errors.push(format!("{base_url}: project id missing"));
    }

    Err(format!(
        "Failed to resolve Antigravity project via loadCodeAssist: {}",
        errors.join("; ")
    ))
}

fn extract_project_id(value: Option<serde_json::Value>) -> Option<String> {
    match value {
        Some(serde_json::Value::String(project_id)) => {
            let trimmed = project_id.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Some(serde_json::Value::Object(map)) => map
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string),
        _ => None,
    }
}

fn email_from_id_token(id_token: Option<&str>) -> Option<String> {
    let token = id_token?;
    let payload = token.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    value
        .get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn generate_code_verifier() -> String {
    let raw = format!("{}{}", Uuid::new_v4(), Uuid::new_v4()).replace('-', "");
    URL_SAFE_NO_PAD.encode(raw.as_bytes())
}

fn code_challenge_s256(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
