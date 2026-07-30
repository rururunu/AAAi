//! Antigravity Cloud Code PA provider (`v1internal:generateContent`).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::core::runtime::{ChatMessage, ChatRequest, Role, StreamEvent, ToolCallPayload};
use crate::services::gemini_oauth::{
    self, antigravity_http_error_message, antigravity_production_api_url,
    antigravity_transport_error_message, antigravity_user_agent, client_metadata_header,
    antigravity_http_client, is_retryable_antigravity_status, ANTIGRAVITY_RETRY_BACKOFF,
    MAX_ANTIGRAVITY_RETRIES, X_GOOG_API_CLIENT,
};

use super::provider::{AIProvider, ProviderError};
use super::image_analysis::{
    decode_image_inline_payload, split_image_content, ImageContentSegment,
};

const FUNCTION_CALL_GUARD: &str = "\n\n## Function calling\n\
     - When you call a tool, emit a native function call, not code. Never write \
     Python (or any language) that calls the tool, and never wrap a call in \
     print(...) or a code block.\n\
     - Use the function name exactly as defined. Do not prepend `default_api.` \
     or any other namespace to the function name.";

const IMAGE_DESCRIBE_PROMPT: &str = "You are a professional visual analyst. Provide a detailed, structured description of this image covering: (1) all visible text transcribed via precise OCR; (2) primary subjects and scene content; (3) charts, diagrams, or layout structure; (4) key information; and (5) color palette and visual style. Output the analysis directly—no preamble, greetings, or closing remarks.";

pub struct AntigravityProvider {
    app: tauri::AppHandle,
    model_override: Option<String>,
}

impl AntigravityProvider {
    pub fn for_model(app: tauri::AppHandle, model: String) -> Self {
        Self { app, model_override: Some(model) }
    }
}

#[async_trait]
impl AIProvider for AntigravityProvider {
    fn id(&self) -> &'static str {
        "antigravity"
    }

    async fn stream(
        &self,
        request: ChatRequest,
        tx: Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let _ = tx.send(StreamEvent::Start).await;

        let project_id = gemini_oauth::ensure_project_id_async(&self.app)
            .await
            .map_err(ProviderError::message)?;

        let settings = crate::services::settings_store::get_settings(&self.app)
            .map_err(ProviderError::message)?;
        let selected_model = self.model_override.as_deref().unwrap_or(settings.chat_model.trim());
        let model = gemini_oauth::resolve_antigravity_model_id(selected_model);
        if model.is_empty() {
            return emit_error(
                &tx,
                ProviderError::message(
                    "No model selected. Sign in to Gemini in Settings and choose a model first.",
                ),
            )
            .await;
        }

        let (system, contents) = match build_contents(&request.messages) {
            Ok(value) => value,
            Err(error) => return emit_error(&tx, error).await,
        };
        let tools = build_tools(&request.tools);
        let has_tools = tools.is_some();
        let system_instruction = build_system_instruction(&system, has_tools);

        let body = CodeAssistGenerateRequest {
            model: model.clone(),
            project: project_id.clone(),
            request_id: Uuid::new_v4().to_string(),
            user_agent: "antigravity".to_string(),
            request: VertexGenerateContentRequest {
                contents,
                system_instruction,
                tools,
                tool_config: if has_tools {
                    Some(GeminiToolConfig {
                        function_calling_config: GeminiFunctionCallingConfig { mode: "AUTO" },
                    })
                } else {
                    None
                },
                session_id: Some(request.session_id.clone()),
                generation_config: None,
            },
        };

        let client = antigravity_http_client().map_err(ProviderError::message)?;
        let parsed = match post_generate_content(&self.app, &body, &client).await {
            Ok(value) => value,
            Err(error) => return emit_error(&tx, error).await,
        };
        let candidate = parsed
            .response
            .and_then(|r| r.candidates)
            .and_then(|mut c| c.drain(..).next())
            .ok_or_else(|| ProviderError::message("Antigravity returned no candidate content"))?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();
        let mut pending_signature: Option<String> = None;
        let mut last_signature: Option<String> = None;

        if let Some(parts_content) = candidate.content {
            for part in parts_content.parts {
                let part_signature = part
                    .thought_signature
                    .as_ref()
                    .filter(|sig| !sig.is_empty())
                    .cloned();
                if let Some(sig) = &part_signature {
                    last_signature = Some(sig.clone());
                }

                if let Some(text) = part.text.filter(|t| !t.is_empty()) {
                    content.push_str(&text);
                    let _ = tx.send(StreamEvent::Delta(text)).await;
                }

                if let Some(function_call) = part.function_call {
                    let signature = part_signature
                        .or_else(|| pending_signature.take())
                        .or_else(|| last_signature.clone());
                    let id = function_call
                        .id
                        .filter(|id| !id.trim().is_empty())
                        .unwrap_or_else(|| Uuid::new_v4().to_string());
                    let arguments = match function_call.args {
                        Value::String(s) => s,
                        other => other.to_string(),
                    };
                    let payload = ToolCallPayload {
                        id: id.clone(),
                        name: function_call.name.clone(),
                        arguments: arguments.clone(),
                        thought_signature: signature,
                    };
                    tool_calls.push(payload.clone());
                    let _ = tx.send(StreamEvent::ToolCall(payload)).await;
                } else if let Some(signature) = part_signature {
                    pending_signature = Some(signature);
                }
            }
        }

        let finish_reason = candidate.finish_reason.clone();
        if content.is_empty()
            && tool_calls.is_empty()
            && finish_reason
                .as_deref()
                .map(|reason| {
                    !matches!(
                        reason.to_ascii_uppercase().as_str(),
                        "STOP" | "MAX_TOKENS" | "FINISH_REASON_UNSPECIFIED" | ""
                    )
                })
                .unwrap_or(false)
        {
            return emit_error(
                &tx,
                ProviderError::message(format!(
                    "Antigravity produced no usable output (finish_reason={})",
                    finish_reason.as_deref().unwrap_or("unknown")
                )),
            )
            .await;
        }

        let _ = tx
            .send(StreamEvent::TurnComplete {
                content,
                reasoning: None,
                tool_calls,
                finish_reason,
            })
            .await;
        let _ = tx.send(StreamEvent::Finish).await;
        Ok(())
    }
}

async fn emit_error(tx: &Sender<StreamEvent>, error: ProviderError) -> Result<(), ProviderError> {
    let message = error.to_string();
    let _ = tx.send(StreamEvent::Error(message)).await;
    Err(error)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CodeAssistGenerateRequest {
    model: String,
    project: String,
    request_id: String,
    user_agent: String,
    request: VertexGenerateContentRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VertexGenerateContentRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiTool>>,
    #[serde(rename = "toolConfig", skip_serializing_if = "Option::is_none")]
    tool_config: Option<GeminiToolConfig>,
    #[serde(rename = "session_id", skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiContent {
    #[serde(default)]
    role: String,
    #[serde(default)]
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline_data: Option<GeminiInlineData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_call: Option<GeminiFunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_response: Option<GeminiFunctionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thought_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiInlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiFunctionCall {
    name: String,
    #[serde(default)]
    args: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiFunctionResponse {
    name: String,
    response: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
}

#[derive(Debug, Serialize)]
struct GeminiTool {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Debug, Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiToolConfig {
    function_calling_config: GeminiFunctionCallingConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFunctionCallingConfig {
    mode: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodeAssistGenerateResponse {
    #[serde(default)]
    response: Option<VertexGenerateContentResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VertexGenerateContentResponse {
    #[serde(default)]
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    #[serde(default)]
    content: Option<GeminiContent>,
    #[serde(default)]
    finish_reason: Option<String>,
}

fn build_system_instruction(system: &str, has_tools: bool) -> Option<GeminiContent> {
    let mut text = system.trim().to_string();
    if has_tools {
        text.push_str(FUNCTION_CALL_GUARD);
    }
    if text.is_empty() {
        return None;
    }
    Some(GeminiContent {
        role: "user".into(),
        parts: vec![GeminiPart {
            text: Some(text),
            ..Default::default()
        }],
    })
}

fn build_user_parts(content: &str) -> Result<Vec<GeminiPart>, ProviderError> {
    let mut parts = Vec::new();
    for segment in split_image_content(content) {
        match segment {
            ImageContentSegment::Text(text) => {
                if !text.trim().is_empty() {
                    parts.push(GeminiPart {
                        text: Some(text),
                        ..Default::default()
                    });
                }
            }
            ImageContentSegment::ImagePayload(payload) => {
                let (mime_type, data) = decode_image_inline_payload(&payload)
                    .map_err(ProviderError::message)?;
                parts.push(GeminiPart {
                    inline_data: Some(GeminiInlineData { mime_type, data }),
                    ..Default::default()
                });
            }
        }
    }
    Ok(parts)
}

fn build_contents(messages: &[ChatMessage]) -> Result<(String, Vec<GeminiContent>), ProviderError> {
    let mut system = String::new();
    let mut contents = Vec::new();
    let mut last_signature: Option<String> = None;
    let mut tool_name_by_id: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for message in messages {
        match message.role {
            Role::System => {
                if !system.is_empty() {
                    system.push('\n');
                }
                system.push_str(&message.content);
            }
            Role::User => {
                if message.content.trim().is_empty() {
                    continue;
                }
                let parts = build_user_parts(&message.content)?;
                if !parts.is_empty() {
                    contents.push(GeminiContent {
                        role: "user".into(),
                        parts,
                    });
                }
            }
            Role::Assistant => {
                let mut parts = Vec::new();
                if !message.content.trim().is_empty() {
                    parts.push(GeminiPart {
                        text: Some(message.content.clone()),
                        ..Default::default()
                    });
                }
                if let Some(calls) = &message.tool_calls {
                    for call in calls {
                        if let Some(sig) = call
                            .thought_signature
                            .as_ref()
                            .filter(|s| !s.is_empty())
                        {
                            last_signature = Some(sig.clone());
                        }
                        tool_name_by_id.insert(call.id.clone(), call.name.clone());
                        let args: Value = serde_json::from_str(&call.arguments)
                            .unwrap_or_else(|_| json!({ "raw": call.arguments }));
                        parts.push(GeminiPart {
                            function_call: Some(GeminiFunctionCall {
                                name: call.name.clone(),
                                args: if args.is_object() {
                                    args
                                } else {
                                    json!({ "value": args })
                                },
                                id: Some(call.id.clone()),
                            }),
                            thought_signature: call
                                .thought_signature
                                .clone()
                                .filter(|s| !s.is_empty())
                                .or_else(|| last_signature.clone()),
                            ..Default::default()
                        });
                    }
                }
                if !parts.is_empty() {
                    contents.push(GeminiContent {
                        role: "model".into(),
                        parts,
                    });
                }
            }
            Role::Tool => {
                let tool_use_id = message.tool_call_id.clone().unwrap_or_default();
                let name = message
                    .name
                    .clone()
                    .or_else(|| tool_name_by_id.get(&tool_use_id).cloned())
                    .unwrap_or_else(|| "tool".into());
                contents.push(GeminiContent {
                    role: "user".into(),
                    parts: vec![GeminiPart {
                        function_response: Some(GeminiFunctionResponse {
                            name,
                            response: json!({ "content": message.content }),
                            id: if tool_use_id.is_empty() {
                                None
                            } else {
                                Some(tool_use_id)
                            },
                        }),
                        ..Default::default()
                    }],
                });
            }
        }
    }

    Ok((system, contents))
}

fn build_tools(tools: &[Value]) -> Option<Vec<GeminiTool>> {
    if tools.is_empty() {
        return None;
    }
    let mut declarations = Vec::new();
    for tool in tools {
        let function = tool.get("function").unwrap_or(tool);
        let name = function
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if name.is_empty() {
            continue;
        }
        let description = function
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let parameters = function
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));
        declarations.push(GeminiFunctionDeclaration {
            name: name.to_string(),
            description,
            parameters,
        });
    }
    if declarations.is_empty() {
        None
    } else {
        Some(vec![GeminiTool {
            function_declarations: declarations,
        }])
    }
}

/// Describe an image via Antigravity `generateContent` (for multimodal split-analysis fallback).
pub async fn describe_image_via_antigravity(
    app: &tauri::AppHandle,
    model: &str,
    image_payload: &str,
) -> Result<String, ProviderError> {
    let project_id = gemini_oauth::ensure_project_id_async(app)
        .await
        .map_err(ProviderError::message)?;
    let model = gemini_oauth::resolve_antigravity_model_id(model);
    if model.is_empty() {
        return Err(ProviderError::message(
            "Multimodal Gemini model is not configured. Select a Gemini model in Settings.",
        ));
    }

    let (mime_type, data) =
        decode_image_inline_payload(image_payload).map_err(ProviderError::message)?;

    let body = CodeAssistGenerateRequest {
        model: model.clone(),
        project: project_id,
        request_id: Uuid::new_v4().to_string(),
        user_agent: "antigravity".to_string(),
        request: VertexGenerateContentRequest {
            contents: vec![GeminiContent {
                role: "user".into(),
                parts: vec![
                    GeminiPart {
                        text: Some(IMAGE_DESCRIBE_PROMPT.to_string()),
                        ..Default::default()
                    },
                    GeminiPart {
                        inline_data: Some(GeminiInlineData { mime_type, data }),
                        ..Default::default()
                    },
                ],
            }],
            system_instruction: None,
            tools: None,
            tool_config: None,
            session_id: None,
            generation_config: Some(json!({ "maxOutputTokens": 4096 })),
        },
    };

    let client = antigravity_http_client().map_err(ProviderError::message)?;
    let parsed = post_generate_content(app, &body, &client).await?;
    extract_candidate_text(parsed)
}

async fn post_generate_content(
    app: &tauri::AppHandle,
    body: &CodeAssistGenerateRequest,
    client: &reqwest::Client,
) -> Result<CodeAssistGenerateResponse, ProviderError> {
    let access_token = gemini_oauth::ensure_access_token_async(app)
        .await
        .map_err(ProviderError::message)?;
    let project_id_header = format!("project={}", body.project);
    let url = antigravity_production_api_url("v1internal:generateContent");

    let mut last_error = ProviderError::message("Antigravity request failed");

    for attempt in 0..MAX_ANTIGRAVITY_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(ANTIGRAVITY_RETRY_BACKOFF * attempt).await;
        }

        let response = match client
            .post(&url)
            .bearer_auth(&access_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::USER_AGENT, antigravity_user_agent())
            .header("x-goog-api-client", X_GOOG_API_CLIENT)
            .header("x-goog-request-params", &project_id_header)
            .header("Client-Metadata", client_metadata_header())
            .json(body)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                last_error = ProviderError::message(antigravity_transport_error_message(&error));
                if attempt + 1 < MAX_ANTIGRAVITY_RETRIES {
                    continue;
                }
                return Err(last_error);
            }
        };

        let status = response.status();
        let text = match response.text().await {
            Ok(text) => text,
            Err(error) => {
                last_error =
                    ProviderError::message(format!("Failed to read Antigravity response: {error}"));
                if attempt + 1 < MAX_ANTIGRAVITY_RETRIES {
                    continue;
                }
                return Err(last_error);
            }
        };

        if status.is_success() {
            return serde_json::from_str::<CodeAssistGenerateResponse>(&text).map_err(|error| {
                ProviderError::message(format!(
                    "Failed to parse Antigravity response: {error}; body={text}"
                ))
            });
        }

        last_error = ProviderError::message(antigravity_http_error_message(status, &text));
        if attempt + 1 < MAX_ANTIGRAVITY_RETRIES && is_retryable_antigravity_status(status) {
            continue;
        }
        return Err(last_error);
    }

    Err(last_error)
}

fn extract_candidate_text(parsed: CodeAssistGenerateResponse) -> Result<String, ProviderError> {
    let candidate = parsed
        .response
        .and_then(|r| r.candidates)
        .and_then(|mut c| c.drain(..).next())
        .ok_or_else(|| ProviderError::message("Antigravity returned no candidate content"))?;

    let mut content = String::new();
    if let Some(parts_content) = candidate.content {
        for part in parts_content.parts {
            if let Some(text) = part.text.filter(|t| !t.is_empty()) {
                content.push_str(&text);
            }
        }
    }

    if content.trim().is_empty() {
        return Err(ProviderError::message(
            "Antigravity image analysis returned no usable text",
        ));
    }
    Ok(content)
}
