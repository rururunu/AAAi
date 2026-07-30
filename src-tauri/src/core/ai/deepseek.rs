use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use tokio::sync::mpsc::Sender;

use crate::core::runtime::{ChatMessage, ChatRequest, Role, StreamEvent, ToolCallPayload};
use crate::models::chat::ChatModelInfo;
use crate::models::settings::ReasoningEffort;

use super::provider::{AIProvider, ProviderError};

mod multimodal;

#[cfg(test)]
use multimodal::{
    antigravity_model_for_image_describe, multimodal_http_error_message,
    multimodal_transport_error_message, should_retry_multimodal_as_stream,
};
use multimodal::{describe_image, multimodal_http_client, resolve_multimodal_endpoint};

const API_URL: &str = "https://api.deepseek.com/chat/completions";
const MODELS_URL: &str = "https://api.deepseek.com/models";
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_PRE_TOKEN_RETRIES: u32 = 3;
const RETRY_BACKOFF: Duration = Duration::from_millis(500);

const USER_STREAM_INTERRUPTED: &str = "Connection interrupted, please retry";
const USER_STREAM_STALLED: &str = "Response timed out, please retry";

pub struct DeepSeekProvider {
    app: tauri::AppHandle,
    resolve_api_key: Arc<dyn Fn() -> String + Send + Sync>,
    resolve_model: Arc<dyn Fn() -> String + Send + Sync>,
    resolve_effort: Arc<dyn Fn() -> ReasoningEffort + Send + Sync>,
    resolve_pass_tool_reasoning: Arc<dyn Fn() -> bool + Send + Sync>,
    /// Optional resolver that returns a custom chat-completions URL.
    /// When `None` (or the resolver returns `None`) the default `API_URL` is used.
    resolve_base_url: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
}

impl DeepSeekProvider {
    pub fn new(
        app: tauri::AppHandle,
        resolve_api_key: Arc<dyn Fn() -> String + Send + Sync>,
        resolve_model: Arc<dyn Fn() -> String + Send + Sync>,
        resolve_effort: Arc<dyn Fn() -> ReasoningEffort + Send + Sync>,
        resolve_pass_tool_reasoning: Arc<dyn Fn() -> bool + Send + Sync>,
        resolve_base_url: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
    ) -> Self {
        Self {
            app,
            resolve_api_key,
            resolve_model,
            resolve_effort,
            resolve_pass_tool_reasoning,
            resolve_base_url,
        }
    }

    fn api_key(&self) -> Result<String, ProviderError> {
        let api_key = (self.resolve_api_key)();
        if api_key.trim().is_empty() {
            return Err(ProviderError::message(
                "Model credentials are not configured. Sign in to Gemini (Antigravity) or enter an API Key in Settings.",
            ));
        }
        Ok(api_key.trim().to_string())
    }

    fn model(&self) -> Result<String, ProviderError> {
        let model = (self.resolve_model)();
        let trimmed = model.trim();
        if trimmed.is_empty() {
            return Err(ProviderError::message(
                "No model selected. Configure a provider and choose a model in Settings first.",
            ));
        }
        Ok(trimmed.to_string())
    }

    fn effort(&self) -> ReasoningEffort {
        (self.resolve_effort)()
    }

    fn pass_tool_reasoning(&self) -> bool {
        (self.resolve_pass_tool_reasoning)()
    }

    fn chat_completions_url(&self) -> String {
        if let Some(resolver) = &self.resolve_base_url {
            if let Some(base) = resolver() {
                return normalize_chat_completions_url(&base);
            }
        }
        API_URL.to_string()
    }
}

#[derive(Debug, Deserialize)]
struct ApiStreamResponse {
    choices: Vec<ApiStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct ApiStreamChoice {
    delta: ApiStreamDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ApiStreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ApiToolCallDelta>>,
}

#[derive(Debug, Deserialize, Default)]
struct ApiToolCallDelta {
    index: Option<usize>,
    id: Option<String>,
    #[serde(default)]
    function: Option<ApiToolCallFunction>,
}

#[derive(Debug, Deserialize, Default)]
struct ApiToolCallFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Default)]
struct ToolCallBuilder {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Debug, Default)]
struct StreamReadOutcome {
    emitted: bool,
    saw_done: bool,
    finish_reason: Option<String>,
}

impl StreamReadOutcome {
    fn is_complete(&self) -> bool {
        self.saw_done || self.finish_reason.is_some()
    }
}

#[async_trait]
impl AIProvider for DeepSeekProvider {
    fn id(&self) -> &'static str {
        "deepseek"
    }

    async fn stream(
        &self,
        request: ChatRequest,
        tx: Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let settings = crate::services::settings_store::get_settings(&self.app).unwrap_or_default();
        let mut request = request;
        let primary_model = self.model()?;
        let primary_api_key = self.api_key()?;
        let primary_url = self.chat_completions_url();
        let effort = self.effort();
        let pass_tool_reasoning = self.pass_tool_reasoning();
        let include_thinking = !primary_url.contains("generativelanguage.googleapis.com");

        let has_images = request
            .messages
            .iter()
            .any(|msg| msg.role == Role::User && msg.content.contains("![image]("));

        let _ = tx.send(StreamEvent::Start).await;
        let client = reqwest::Client::new();

        let mut model = primary_model.clone();
        let mut api_key = primary_api_key.clone();
        let mut url = primary_url.clone();

        if has_images {
            let body = build_api_body(
                &request,
                &primary_model,
                true,
                effort,
                pass_tool_reasoning,
                include_thinking,
            );
            match run_chat_stream(&client, &primary_url, &primary_api_key, &body, &tx).await {
                Ok(()) => return Ok(()),
                // Multimodal split-analysis is only for text-only primaries.
                // Gemini / gpt-4o / Claude already see images natively — never describe→reask.
                Err(error)
                    if crate::core::ai::multimodal::is_vision_unsupported_error(&error)
                        && !crate::core::ai::multimodal::primary_model_has_native_vision(
                            &primary_model,
                        ) =>
                {
                    match apply_image_input_fallback(&mut request, &settings, &self.app, &tx).await
                    {
                        Ok(FallbackPlan::RetryPrimary) => {
                            model = primary_model;
                            api_key = primary_api_key;
                            url = primary_url;
                        }
                        Ok(FallbackPlan::SwitchToMultimodal {
                            model: mm_model,
                            api_key: mm_key,
                            url: mm_url,
                        }) => {
                            model = mm_model;
                            api_key = mm_key;
                            url = mm_url;
                        }
                        Err(error) => return emit_stream_error(&tx, error).await,
                    }
                }
                Err(error) => return emit_stream_error(&tx, error).await,
            }
        }

        let body = build_api_body(
            &request,
            &model,
            true,
            effort,
            pass_tool_reasoning,
            include_thinking,
        );
        match run_chat_stream(&client, &url, &api_key, &body, &tx).await {
            Ok(()) => Ok(()),
            Err(error) => emit_stream_error(&tx, error).await,
        }
    }
}

enum FallbackPlan {
    RetryPrimary,
    SwitchToMultimodal {
        model: String,
        api_key: String,
        url: String,
    },
}

async fn apply_image_input_fallback(
    request: &mut ChatRequest,
    settings: &crate::models::settings::AppSettings,
    app: &tauri::AppHandle,
    tx: &Sender<StreamEvent>,
) -> Result<FallbackPlan, ProviderError> {
    let mm_model = settings.multimodal_model.trim();

    if !mm_model.is_empty()
        && (settings.multimodal_model_provider.trim().is_empty()
            || settings.multimodal_model_provider.trim() == "gemini")
        && crate::services::gemini_oauth::can_use_antigravity_for_model(settings, mm_model)
        && !settings.multimodal_split_analysis
    {
        apply_split_image_analysis(request, settings, app, tx).await?;
        return Ok(FallbackPlan::RetryPrimary);
    }

    if settings.multimodal_split_analysis {
        apply_split_image_analysis(request, settings, app, tx).await?;
        return Ok(FallbackPlan::RetryPrimary);
    }

    if mm_model.is_empty() {
        return Err(ProviderError::message(
            "The primary model does not support image input. Configure a multimodal model in Settings, or enable multimodal split analysis.",
        ));
    }

    if (settings.multimodal_model_provider.trim().is_empty()
        || settings.multimodal_model_provider.trim() == "gemini")
        && crate::services::gemini_oauth::can_use_antigravity_for_model(settings, mm_model)
    {
        return Err(ProviderError::message(
            "Gemini multimodal models cannot be switched wholesale to the OpenAI API. Enable multimodal split analysis, or use a model such as gpt-4o.",
        ));
    }

    let endpoint = resolve_multimodal_endpoint(
        settings,
        mm_model,
        settings.multimodal_model_provider.trim(),
    )?;
    Ok(FallbackPlan::SwitchToMultimodal {
        model: mm_model.to_string(),
        api_key: endpoint.api_key,
        url: endpoint.url,
    })
}

async fn apply_split_image_analysis(
    request: &mut ChatRequest,
    settings: &crate::models::settings::AppSettings,
    app: &tauri::AppHandle,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    let client = multimodal_http_client();
    let mm_model = if settings.multimodal_model.trim().is_empty() {
        "gpt-4o".to_string()
    } else {
        settings.multimodal_model.trim().to_string()
    };

    let image_re = match regex::Regex::new(r"!\[image\]\((.*?)\)") {
        Ok(re) => re,
        Err(_) => return Err(ProviderError::message("invalid image regex")),
    };

    let mut any_api_calls = false;
    let mut patches: Vec<(String, String)> = Vec::new();

    for message in &mut request.messages {
        if message.role != Role::User || !message.content.contains("![image](") {
            continue;
        }

        let image_markdowns: Vec<String> = image_re
            .find_iter(&message.content)
            .map(|m| m.as_str().to_string())
            .collect();

        let mut stored = message.content.clone();
        let original = stored.clone();

        for image_markdown in &image_markdowns {
            if crate::core::ai::image_analysis::usable_analysis_after_image(&stored, image_markdown)
                .is_some()
            {
                continue;
            }

            if crate::core::ai::image_analysis::analysis_after_image(&stored, image_markdown)
                .is_some()
            {
                stored = crate::core::ai::image_analysis::remove_analysis_after_image(
                    &stored,
                    image_markdown,
                );
            }

            if !any_api_calls {
                any_api_calls = true;
                let _ = tx
                    .send(StreamEvent::Status {
                        kind: "analyzing_images".to_string(),
                    })
                    .await;
            }

            let image_url = image_re
                .captures(image_markdown)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .unwrap_or("");

            let text = match describe_image(&client, app, image_url).await {
                Ok(desc) => desc,
                Err(err) => {
                    let _ = tx
                        .send(StreamEvent::Status {
                            kind: String::new(),
                        })
                        .await;
                    return Err(err);
                }
            };

            stored = crate::core::ai::image_analysis::insert_analysis_after_image(
                &stored,
                image_markdown,
                &mm_model,
                &text,
            );
        }

        if stored != original {
            patches.push((message.id.clone(), stored.clone()));
        }
        message.content = stored;
    }

    for (message_id, content) in &patches {
        let _ = tx
            .send(StreamEvent::UserContentPatch {
                message_id: message_id.clone(),
                content: content.clone(),
            })
            .await;
    }

    if any_api_calls {
        let _ = tx
            .send(StreamEvent::Status {
                kind: String::new(),
            })
            .await;
    }

    for message in &mut request.messages {
        if message.role == Role::User
            && (message.content.contains("![image](")
                || message.content.contains("peek-image-analysis"))
        {
            message.content = crate::core::ai::image_analysis::replace_images_with_analysis_text(
                &message.content,
            );
        }
    }

    Ok(())
}

async fn run_chat_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    let mut last_error: Option<ProviderError> = None;

    for attempt in 0..MAX_PRE_TOKEN_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(RETRY_BACKOFF * attempt).await;
        }

        let response = match post_stream_request(client, url, api_key, body).await {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(error.clone());
                if attempt + 1 < MAX_PRE_TOKEN_RETRIES && is_retryable_before_token(&error) {
                    continue;
                }
                return Err(error);
            }
        };

        match read_sse_stream(response, tx).await {
            Ok(outcome) if outcome.is_complete() => {
                let _ = tx.send(StreamEvent::Finish).await;
                return Ok(());
            }
            Ok(outcome) if outcome.emitted => {
                return Err(ProviderError::message(USER_STREAM_INTERRUPTED));
            }
            Ok(_) if attempt + 1 < MAX_PRE_TOKEN_RETRIES => {
                last_error = Some(ProviderError::message(USER_STREAM_INTERRUPTED));
                continue;
            }
            Ok(_) => return Err(ProviderError::message(USER_STREAM_INTERRUPTED)),
            Err(error)
                if attempt + 1 < MAX_PRE_TOKEN_RETRIES && is_retryable_before_token(&error) =>
            {
                last_error = Some(error);
                continue;
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| ProviderError::message(USER_STREAM_INTERRUPTED)))
}

async fn post_stream_request(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
) -> Result<reqwest::Response, ProviderError> {
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|error| ProviderError::message(format!("network error: {error}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        return Err(ProviderError::message(format!(
            "DeepSeek API {status}: {text}"
        )));
    }

    Ok(response)
}

async fn read_sse_stream(
    response: reqwest::Response,
    tx: &Sender<StreamEvent>,
) -> Result<StreamReadOutcome, ProviderError> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut outcome = StreamReadOutcome::default();
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut tool_calls: HashMap<usize, ToolCallBuilder> = HashMap::new();

    loop {
        let chunk = match tokio::time::timeout(STREAM_IDLE_TIMEOUT, stream.next()).await {
            Ok(Some(Ok(chunk))) => chunk,
            Ok(Some(Err(error))) => {
                return Err(map_read_error(error.to_string(), outcome.emitted));
            }
            Ok(None) => break,
            Err(_) => return Err(ProviderError::message(USER_STREAM_STALLED)),
        };

        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(line) = next_sse_line(&mut buffer) {
            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };

            let payload = payload.trim();
            if payload.is_empty() {
                continue;
            }
            if payload == "[DONE]" {
                outcome.saw_done = true;
                break;
            }

            let parsed: ApiStreamResponse = serde_json::from_str(payload).map_err(|error| {
                ProviderError::message(format!("invalid stream payload: {error}"))
            })?;

            for choice in parsed.choices {
                if let Some(reason) = choice.finish_reason.filter(|value| !value.is_empty()) {
                    outcome.finish_reason = Some(reason);
                }
                if let Some(reasoning_chunk) = choice.delta.reasoning_content.as_deref() {
                    if !reasoning_chunk.is_empty() {
                        outcome.emitted = true;
                        reasoning.push_str(reasoning_chunk);
                        let _ = tx
                            .send(StreamEvent::Reasoning(reasoning_chunk.to_string()))
                            .await;
                    }
                }
                if let Some(content_chunk) = choice.delta.content.as_deref() {
                    if !content_chunk.is_empty() {
                        outcome.emitted = true;
                        content.push_str(content_chunk);
                        let _ = tx.send(StreamEvent::Delta(content_chunk.to_string())).await;
                    }
                }
                if let Some(calls) = choice.delta.tool_calls {
                    for call in calls {
                        let index = call.index.unwrap_or(0);
                        let entry = tool_calls.entry(index).or_default();
                        if let Some(id) = call.id {
                            entry.id = id;
                        }
                        if let Some(function) = call.function {
                            if let Some(name) = function.name {
                                entry.name = name;
                            }
                            if let Some(args) = function.arguments {
                                entry.arguments.push_str(&args);
                            }
                        }
                        outcome.emitted = true;
                    }
                }
            }

            if outcome.saw_done {
                break;
            }
        }

        if outcome.saw_done {
            break;
        }
    }

    if !outcome.is_complete() {
        return Err(ProviderError::message(USER_STREAM_INTERRUPTED));
    }

    let mut merged_calls: Vec<_> = tool_calls.into_iter().collect();
    merged_calls.sort_by_key(|(index, _)| *index);
    let tool_call_payloads: Vec<ToolCallPayload> = merged_calls
        .into_iter()
        .map(|(_, builder)| ToolCallPayload {
            id: builder.id,
            name: builder.name,
            arguments: builder.arguments,
            thought_signature: None,
        })
        .collect();

    let _ = tx
        .send(StreamEvent::TurnComplete {
            content,
            reasoning: non_empty_option(reasoning),
            tool_calls: tool_call_payloads,
            finish_reason: outcome.finish_reason.clone(),
        })
        .await;

    Ok(outcome)
}

fn non_empty_option(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

async fn emit_stream_error(
    tx: &Sender<StreamEvent>,
    error: ProviderError,
) -> Result<(), ProviderError> {
    let message = user_facing_stream_error(&error);
    let _ = tx.send(StreamEvent::Error(message.clone())).await;
    Err(ProviderError::message(message))
}

fn user_facing_stream_error(error: &ProviderError) -> String {
    match error {
        ProviderError::Cancelled => "Request cancelled".to_string(),
        ProviderError::Message(message) => {
            if message.starts_with("DeepSeek API") {
                return message.clone();
            }
            if message.contains("API Key") {
                return message.clone();
            }
            if message.contains("multimodal")
                || message.contains("Multimodal")
                || message.contains("image analysis")
                || message.contains("vision")
                || message.contains("Vision")
            {
                return message.clone();
            }
            if message.contains("invalid stream payload") {
                return USER_STREAM_INTERRUPTED.to_string();
            }
            if message == USER_STREAM_STALLED || message == USER_STREAM_INTERRUPTED {
                return message.clone();
            }
            if is_connection_error(message) {
                return USER_STREAM_INTERRUPTED.to_string();
            }
            message.clone()
        }
    }
}

fn is_connection_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    [
        "connection reset",
        "connection refused",
        "broken pipe",
        "unexpected eof",
        "incomplete",
        "stalled",
        "timed out",
        "network error",
        "error sending request",
        "error decoding response body",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn is_retryable_before_token(error: &ProviderError) -> bool {
    match error {
        ProviderError::Cancelled => false,
        ProviderError::Message(message) => {
            if message.starts_with("DeepSeek API") {
                return false;
            }
            if message.contains("API Key") {
                return false;
            }
            is_connection_error(message)
        }
    }
}

fn map_read_error(message: String, emitted: bool) -> ProviderError {
    if emitted {
        ProviderError::message(USER_STREAM_INTERRUPTED)
    } else if is_connection_error(&message) {
        ProviderError::message(format!("network error: {message}"))
    } else {
        ProviderError::message(message)
    }
}

pub(crate) fn build_api_body(
    request: &ChatRequest,
    model: &str,
    stream: bool,
    effort: ReasoningEffort,
    pass_tool_reasoning: bool,
    include_thinking: bool,
) -> Value {
    let messages: Vec<_> = request
        .messages
        .iter()
        .filter(|message| {
            message.role == Role::Tool
                || message
                    .tool_calls
                    .as_ref()
                    .is_some_and(|calls| !calls.is_empty())
                || !message.content.trim().is_empty()
        })
        .map(|message| message_to_api_json(message, pass_tool_reasoning))
        .collect();

    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert("messages".into(), Value::Array(messages));
    body.insert("stream".into(), json!(stream));

    if stream {
        body.insert("stream_options".into(), json!({ "include_usage": true }));
    }

    if let Some(temperature) = request.temperature {
        body.insert("temperature".into(), json!(temperature));
    }
    if let Some(max_tokens) = request.max_tokens {
        body.insert("max_tokens".into(), json!(max_tokens));
    }

    if !request.tools.is_empty() {
        body.insert(
            "tools".into(),
            Value::Array(request.tools.iter().cloned().collect()),
        );
    }

    if include_thinking {
        apply_thinking_effort(&mut body, effort);
    }

    Value::Object(body)
}

fn parse_multimodal_content(content: &str) -> Value {
    if !content.contains("![image](") {
        return json!(content);
    }

    let re = match regex::Regex::new(r"!\[image\]\((.*?)\)") {
        Ok(re) => re,
        Err(_) => return json!(content),
    };

    let mut parts = Vec::new();
    let mut last_index = 0;

    for cap in re.captures_iter(content) {
        if let Some(mat) = cap.get(0) {
            let before = &content[last_index..mat.start()];
            if !before.trim().is_empty() {
                parts.push(json!({
                    "type": "text",
                    "text": before,
                }));
            }

            if let Some(url_match) = cap.get(1) {
                let url = url_match.as_str();
                parts.push(json!({
                    "type": "image_url",
                    "image_url": {
                        "url": url,
                    }
                }));
            }

            last_index = mat.end();
        }
    }

    if last_index < content.len() {
        let after = &content[last_index..];
        if !after.trim().is_empty() {
            parts.push(json!({
                "type": "text",
                "text": after,
            }));
        }
    }

    if parts.is_empty() {
        json!(content)
    } else {
        Value::Array(parts)
    }
}

fn message_to_api_json(message: &ChatMessage, pass_tool_reasoning: bool) -> Value {
    if message.role == Role::Tool {
        return json!({
            "role": "tool",
            "tool_call_id": message.tool_call_id,
            "content": message.content,
        });
    }

    if message.role == Role::Assistant {
        if let Some(tool_calls) = &message.tool_calls {
            let calls: Vec<Value> = tool_calls
                .iter()
                .map(|call| {
                    json!({
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": call.arguments,
                        }
                    })
                })
                .collect();
            let mut payload = json!({
                "role": "assistant",
                "content": if message.content.is_empty() { Value::Null } else { json!(message.content) },
                "tool_calls": calls,
            });
            if pass_tool_reasoning {
                if let Some(reasoning) = message
                    .reasoning
                    .as_ref()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                {
                    payload
                        .as_object_mut()
                        .expect("assistant payload object")
                        .insert("reasoning_content".into(), json!(reasoning));
                }
            }
            return payload;
        }
    }

    json!({
        "role": role_to_api(message.role),
        "content": parse_multimodal_content(&message.content),
    })
}

fn apply_thinking_effort(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    match effort {
        ReasoningEffort::Disabled => {
            body.insert("thinking".into(), json!({ "type": "disabled" }));
        }
        ReasoningEffort::High => {
            body.insert("thinking".into(), json!({ "type": "enabled" }));
            body.insert("reasoning_effort".into(), json!("high"));
        }
        ReasoningEffort::Max => {
            body.insert("thinking".into(), json!({ "type": "enabled" }));
            body.insert("reasoning_effort".into(), json!("max"));
        }
    }
}

fn role_to_api(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => "tool",
    }
}

fn next_sse_line(buffer: &mut String) -> Option<String> {
    let newline_index = buffer.find('\n')?;
    let mut line = buffer.drain(..=newline_index).collect::<String>();
    if line.ends_with('\n') {
        line.pop();
    }
    if line.ends_with('\r') {
        line.pop();
    }
    Some(line)
}

#[derive(Debug, Deserialize)]
struct ApiModelsResponse {
    data: Vec<ApiModelItem>,
}

#[derive(Debug, Deserialize)]
struct ApiModelItem {
    id: String,
    owned_by: String,
}

pub async fn list_models(api_key: &str) -> Result<Vec<ChatModelInfo>, ProviderError> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(ProviderError::message(
            "DeepSeek API Key is not configured. Please enter it in Settings.",
        ));
    }

    let client = reqwest::Client::new();
    let response = client
        .get(MODELS_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| ProviderError::message(format!("network error: {error}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        return Err(ProviderError::message(format!(
            "DeepSeek API {status}: {text}"
        )));
    }

    let parsed: ApiModelsResponse = response
        .json()
        .await
        .map_err(|error| ProviderError::message(format!("invalid models payload: {error}")))?;

    Ok(parsed
        .data
        .into_iter()
        .map(|item| ChatModelInfo {
            id: item.id,
            owned_by: item.owned_by,
            provider: "deepseek".to_string(),
            display_name: None,
            thinking_variants: None,
        })
        .collect())
}

/// Normalize an OpenAI-compatible base URL to a chat completions endpoint.
///
/// Bare hosts such as `https://www.micuapi.ai` (NewAPI) must become
/// `.../v1/chat/completions`, not `.../chat/completions`.
pub(crate) fn normalize_chat_completions_url(base_url: &str) -> String {
    let mut base = base_url.trim().trim_end_matches('/').to_string();
    if let Some(stripped) = base.strip_suffix("/chat/completions") {
        base = stripped.trim_end_matches('/').to_string();
    }
    if !has_versioned_api_path(&base) {
        base = format!("{base}/v1");
    }
    format!("{base}/chat/completions")
}

fn has_versioned_api_path(base: &str) -> bool {
    let path = url_path(base);
    if path.is_empty() || path == "/" {
        return false;
    }
    path == "/v1" || path.ends_with("/v1") || path.contains("/v1/") || path.contains("/v1beta")
}

fn url_path(base: &str) -> &str {
    let rest = base
        .strip_prefix("https://")
        .or_else(|| base.strip_prefix("http://"))
        .unwrap_or(base);
    match rest.find('/') {
        Some(index) => &rest[index..],
        None => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::runtime::{MessageStatus, RequestContext};

    fn sample_request(messages: Vec<ChatMessage>) -> ChatRequest {
        ChatRequest {
            request_id: "req-1".into(),
            session_id: "default".into(),
            messages,
            context: RequestContext::default(),
            provider: Some("deepseek".into()),
            stream: true,
            tools: std::sync::Arc::from([]),
            temperature: None,
            max_tokens: None,
        }
    }

    fn assistant_with_reasoning() -> ChatMessage {
        ChatMessage {
            id: "msg-a".into(),
            session_id: "default".into(),
            role: Role::Assistant,
            content: "final answer".into(),
            reasoning: Some("hidden chain of thought".into()),
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
        }
    }

    #[test]
    fn build_api_body_omits_null_optional_fields() {
        let body = build_api_body(
            &sample_request(vec![]),
            "deepseek-reasoner",
            true,
            ReasoningEffort::High,
            true,
            true,
        );
        let obj = body.as_object().expect("object body");
        assert!(!obj.contains_key("temperature"));
        assert!(!obj.contains_key("max_tokens"));
        assert_eq!(
            obj.get("stream_options"),
            Some(&json!({ "include_usage": true }))
        );
    }

    #[test]
    fn build_api_body_high_effort_includes_thinking() {
        let body = build_api_body(
            &sample_request(vec![]),
            "deepseek-reasoner",
            true,
            ReasoningEffort::High,
            true,
            true,
        );
        let obj = body.as_object().expect("object body");
        assert_eq!(obj.get("thinking"), Some(&json!({ "type": "enabled" })));
        assert_eq!(obj.get("reasoning_effort"), Some(&json!("high")));
    }

    #[test]
    fn build_api_body_disabled_effort_omits_reasoning_effort() {
        let body = build_api_body(
            &sample_request(vec![]),
            "deepseek-chat",
            true,
            ReasoningEffort::Disabled,
            true,
            true,
        );
        let obj = body.as_object().expect("object body");
        assert_eq!(obj.get("thinking"), Some(&json!({ "type": "disabled" })));
        assert!(!obj.contains_key("reasoning_effort"));
    }

    #[test]
    fn build_api_body_drops_stored_reasoning_from_messages() {
        let request = sample_request(vec![assistant_with_reasoning()]);
        let body = build_api_body(
            &request,
            "deepseek-reasoner",
            true,
            ReasoningEffort::High,
            true,
            true,
        );
        let messages = body["messages"].as_array().expect("messages array");
        assert_eq!(messages.len(), 1);
        let message = &messages[0];
        assert_eq!(message["role"], "assistant");
        assert_eq!(message["content"], "final answer");
        assert!(!message
            .as_object()
            .unwrap()
            .contains_key("reasoning_content"));
    }

    #[test]
    fn stream_outcome_complete_when_done_or_finish_reason() {
        let done = StreamReadOutcome {
            saw_done: true,
            ..Default::default()
        };
        assert!(done.is_complete());

        let finish = StreamReadOutcome {
            finish_reason: Some("stop".into()),
            ..Default::default()
        };
        assert!(finish.is_complete());

        let incomplete = StreamReadOutcome::default();
        assert!(!incomplete.is_complete());
    }

    #[test]
    fn user_facing_stream_error_maps_network_failures() {
        let error = ProviderError::message("network error: connection reset");
        assert_eq!(user_facing_stream_error(&error), USER_STREAM_INTERRUPTED);
    }

    #[test]
    fn message_to_api_json_serializes_tool_result() {
        use crate::core::runtime::{MessageStatus, ToolCallPayload};

        let assistant = ChatMessage {
            id: "a1".into(),
            session_id: "default".into(),
            role: Role::Assistant,
            content: String::new(),
            reasoning: None,
            tool_activities: None,
            tool_calls: Some(vec![ToolCallPayload {
                id: "call-1".into(),
                name: "read_file".into(),
                arguments: r#"{"path":"README.md"}"#.into(),
                thought_signature: None,
            }]),
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
        };
        let tool = ChatMessage {
            id: "t1".into(),
            session_id: "default".into(),
            role: Role::Tool,
            content: "file contents".into(),
            reasoning: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: Some("call-1".into()),
            name: Some("read_file".into()),
            status: MessageStatus::Done,
            timestamp: 2,
        };

        let assistant_json = message_to_api_json(&assistant, true);
        assert_eq!(assistant_json["role"], "assistant");
        assert!(assistant_json["tool_calls"].is_array());

        let tool_json = message_to_api_json(&tool, true);
        assert_eq!(tool_json["role"], "tool");
        assert_eq!(tool_json["tool_call_id"], "call-1");
    }

    #[test]
    fn tool_call_turn_includes_reasoning_when_enabled() {
        use crate::core::runtime::ToolCallPayload;

        let assistant = ChatMessage {
            id: "a1".into(),
            session_id: "default".into(),
            role: Role::Assistant,
            content: String::new(),
            reasoning: Some("need to read the file first".into()),
            tool_activities: None,
            tool_calls: Some(vec![ToolCallPayload {
                id: "call-1".into(),
                name: "read_file".into(),
                arguments: r#"{"path":"a.rs"}"#.into(),
                thought_signature: None,
            }]),
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
        };

        let enabled = message_to_api_json(&assistant, true);
        assert_eq!(enabled["reasoning_content"], "need to read the file first");

        let disabled = message_to_api_json(&assistant, false);
        assert!(!disabled
            .as_object()
            .unwrap()
            .contains_key("reasoning_content"));
    }

    #[test]
    fn build_api_body_includes_tools_when_present() {
        let mut request = sample_request(vec![]);
        request.tools =
            std::sync::Arc::from([json!({"type": "function", "function": {"name": "read_file"}})]);
        let body = build_api_body(
            &request,
            "deepseek-chat",
            true,
            ReasoningEffort::Disabled,
            true,
            true,
        );
        assert!(body["tools"].is_array());
    }

    #[test]
    fn normalize_chat_completions_url_avoids_duplication() {
        assert_eq!(
            normalize_chat_completions_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            normalize_chat_completions_url("https://api.openai.com/v1/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            normalize_chat_completions_url("https://proxy.example/v1/"),
            "https://proxy.example/v1/chat/completions"
        );
    }

    #[test]
    fn normalize_chat_completions_url_injects_v1_for_bare_host() {
        assert_eq!(
            normalize_chat_completions_url("https://www.micuapi.ai"),
            "https://www.micuapi.ai/v1/chat/completions"
        );
        assert_eq!(
            normalize_chat_completions_url("https://www.micuapi.ai/"),
            "https://www.micuapi.ai/v1/chat/completions"
        );
        assert_eq!(
            normalize_chat_completions_url("https://www.micuapi.ai/chat/completions"),
            "https://www.micuapi.ai/v1/chat/completions"
        );
    }

    #[test]
    fn resolve_multimodal_endpoint_requires_custom_provider() {
        let settings = crate::models::settings::AppSettings::default();
        let err = resolve_multimodal_endpoint(&settings, "gpt-4o", "").unwrap_err();
        match err {
            ProviderError::Message(msg) => {
                assert!(msg.contains("not configured under any custom provider"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn resolve_multimodal_endpoint_uses_custom_provider() {
        let mut settings = crate::models::settings::AppSettings::default();
        settings
            .custom_providers
            .push(crate::models::settings::CustomProviderConfig {
                id: "openai".into(),
                name: "OpenAI".into(),
                base_url: "https://api.openai.com/v1/chat/completions".into(),
                api_key: "sk-test".into(),
                models: "gpt-4o, gpt-4o-mini".into(),
            });
        let endpoint = resolve_multimodal_endpoint(&settings, "gpt-4o", "openai").unwrap();
        assert_eq!(endpoint.api_key, "sk-test");
        assert_eq!(endpoint.url, "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn resolve_multimodal_endpoint_disambiguates_duplicate_model_ids() {
        let provider = |id: &str, key: &str| crate::models::settings::CustomProviderConfig {
            id: id.into(),
            name: id.into(),
            base_url: format!("https://{id}.example/v1"),
            api_key: key.into(),
            models: "shared-vision-model".into(),
        };
        let settings = crate::models::settings::AppSettings {
            custom_providers: vec![provider("first", "key-1"), provider("second", "key-2")],
            ..Default::default()
        };

        let endpoint =
            resolve_multimodal_endpoint(&settings, "shared-vision-model", "second").unwrap();
        assert_eq!(endpoint.api_key, "key-2");
        assert_eq!(endpoint.url, "https://second.example/v1/chat/completions");
    }

    #[test]
    fn multimodal_http_error_message_explains_502() {
        let msg = multimodal_http_error_message(
            reqwest::StatusCode::BAD_GATEWAY,
            r#"{"error":"Bad gateway"}"#,
        );
        assert!(msg.contains("502"));
        assert!(msg.contains("Bad gateway"));
        assert!(msg.contains("oversized image") || msg.contains("upstream"));
        assert!(msg.contains("Bad gateway"));
        let facing = user_facing_stream_error(&ProviderError::message(msg.clone()));
        assert_eq!(facing, msg);
    }

    #[test]
    fn multimodal_transport_error_message_explains_send_failure() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let err = runtime.block_on(async {
            reqwest::Client::builder()
                .timeout(Duration::from_millis(1))
                .build()
                .unwrap()
                .get("http://127.0.0.1:1/")
                .send()
                .await
                .expect_err("should fail")
        });
        let msg = multimodal_transport_error_message(&err);
        assert!(
            msg.contains("Connection")
                || msg.contains("network")
                || msg.contains("proxy")
                || msg.contains("could not be sent"),
            "unexpected message: {msg}"
        );
        assert!(msg.contains("Details:"));
    }

    #[test]
    fn should_not_retry_stream_after_body_decode_failure() {
        assert!(!should_retry_multimodal_as_stream(&ProviderError::message(
            "Failed to read multimodal response: error decoding response body"
        )));
        assert!(should_retry_multimodal_as_stream(&ProviderError::message(
            "Failed to extract an image description from the multimodal response. Debug: empty. Snippet: {}"
        )));
    }

    #[test]
    fn antigravity_describe_model_only_when_multimodal_is_gemini() {
        let mut settings = crate::models::settings::AppSettings {
            chat_model: "gemini-3.5-flash-low".into(),
            multimodal_model: "gpt-4o".into(),
            ..Default::default()
        };
        settings.gemini_oauth.refresh_token = "rt".into();
        // Chat Gemini must not hijack multimodal describe — chat already sees images natively.
        assert!(antigravity_model_for_image_describe(&settings, "gpt-4o").is_none());
        assert_eq!(
            antigravity_model_for_image_describe(&settings, "gemini-3-flash").as_deref(),
            Some("gemini-3-flash")
        );

        settings.multimodal_model_provider = "custom-gemini".into();
        assert!(antigravity_model_for_image_describe(&settings, "gemini-3-flash").is_none());
    }
}
