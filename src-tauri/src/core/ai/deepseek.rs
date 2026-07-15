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

const API_URL: &str = "https://api.deepseek.com/chat/completions";
const MODELS_URL: &str = "https://api.deepseek.com/models";
const DEFAULT_MODEL: &str = "deepseek-chat";
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_PRE_TOKEN_RETRIES: u32 = 3;
const RETRY_BACKOFF: Duration = Duration::from_millis(500);

const USER_STREAM_INTERRUPTED: &str = "连接中断，请重试";
const USER_STREAM_STALLED: &str = "响应超时，请重试";

pub struct DeepSeekProvider {
    resolve_api_key: Arc<dyn Fn() -> String + Send + Sync>,
    resolve_model: Arc<dyn Fn() -> String + Send + Sync>,
    resolve_effort: Arc<dyn Fn() -> ReasoningEffort + Send + Sync>,
    resolve_pass_tool_reasoning: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl DeepSeekProvider {
    pub fn new(
        resolve_api_key: Arc<dyn Fn() -> String + Send + Sync>,
        resolve_model: Arc<dyn Fn() -> String + Send + Sync>,
        resolve_effort: Arc<dyn Fn() -> ReasoningEffort + Send + Sync>,
        resolve_pass_tool_reasoning: Arc<dyn Fn() -> bool + Send + Sync>,
    ) -> Self {
        Self {
            resolve_api_key,
            resolve_model,
            resolve_effort,
            resolve_pass_tool_reasoning,
        }
    }

    fn api_key(&self) -> Result<String, ProviderError> {
        let api_key = (self.resolve_api_key)();
        if api_key.trim().is_empty() {
            return Err(ProviderError::message(
                "DeepSeek API Key 未配置，请在设置中填写",
            ));
        }
        Ok(api_key.trim().to_string())
    }

    fn model(&self) -> String {
        let model = (self.resolve_model)();
        let trimmed = model.trim();
        if trimmed.is_empty() {
            DEFAULT_MODEL.to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn effort(&self) -> ReasoningEffort {
        (self.resolve_effort)()
    }

    fn pass_tool_reasoning(&self) -> bool {
        (self.resolve_pass_tool_reasoning)()
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
        let api_key = self.api_key()?;
        let model = self.model();
        let effort = self.effort();
        let pass_tool_reasoning = self.pass_tool_reasoning();
        let body = build_api_body(&request, &model, true, effort, pass_tool_reasoning);

        let _ = tx.send(StreamEvent::Start).await;

        let client = reqwest::Client::new();
        let mut last_error: Option<ProviderError> = None;

        for attempt in 0..MAX_PRE_TOKEN_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(RETRY_BACKOFF * attempt).await;
            }

            let response = match post_stream_request(&client, &api_key, &body).await {
                Ok(response) => response,
                Err(error) => {
                    last_error = Some(error.clone());
                    if attempt + 1 < MAX_PRE_TOKEN_RETRIES && is_retryable_before_token(&error) {
                        continue;
                    }
                    return emit_stream_error(&tx, error).await;
                }
            };

            match read_sse_stream(response, &tx).await {
                Ok(outcome) if outcome.is_complete() => {
                    let _ = tx.send(StreamEvent::Finish).await;
                    return Ok(());
                }
                Ok(outcome) if outcome.emitted => {
                    let error = ProviderError::message(USER_STREAM_INTERRUPTED);
                    return emit_stream_error(&tx, error).await;
                }
                Ok(_) if attempt + 1 < MAX_PRE_TOKEN_RETRIES => {
                    last_error = Some(ProviderError::message(USER_STREAM_INTERRUPTED));
                    continue;
                }
                Ok(_) => {
                    let error = ProviderError::message(USER_STREAM_INTERRUPTED);
                    return emit_stream_error(&tx, error).await;
                }
                Err(error)
                    if attempt + 1 < MAX_PRE_TOKEN_RETRIES && is_retryable_before_token(&error) =>
                {
                    last_error = Some(error);
                    continue;
                }
                Err(error) => return emit_stream_error(&tx, error).await,
            }
        }

        emit_stream_error(
            &tx,
            last_error.unwrap_or_else(|| ProviderError::message(USER_STREAM_INTERRUPTED)),
        )
        .await
    }
}

async fn post_stream_request(
    client: &reqwest::Client,
    api_key: &str,
    body: &Value,
) -> Result<reqwest::Response, ProviderError> {
    let response = client
        .post(API_URL)
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
        ProviderError::Cancelled => "请求已取消".to_string(),
        ProviderError::Message(message) => {
            if message.starts_with("DeepSeek API") {
                return message.clone();
            }
            if message.contains("API Key") {
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

    apply_thinking_effort(&mut body, effort);

    Value::Object(body)
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
        "content": message.content,
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
            "DeepSeek API Key 未配置，请在设置中填写",
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
        })
        .collect())
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
        request.tools = std::sync::Arc::from([json!({"type": "function", "function": {"name": "read_file"}})]);
        let body = build_api_body(
            &request,
            "deepseek-chat",
            true,
            ReasoningEffort::Disabled,
            true,
        );
        assert!(body["tools"].is_array());
    }
}
