use serde_json::{json, Map, Value};

use crate::core::runtime::{ChatMessage, ChatRequest, Role};
use crate::models::settings::ReasoningEffort;

pub(crate) fn build_api_body(
    request: &ChatRequest,
    model: &str,
    stream: bool,
    effort: ReasoningEffort,
    pass_tool_reasoning: bool,
    include_thinking: bool,
) -> Value {
    // After tools have already run in this turn, disable thinking so DeepSeek
    // does not re-generate the same chain-of-thought every continuation step.
    // Historical tool-call turns still pass reasoning_content (protocol).
    let continuing = is_tool_continuation(&request.messages);
    let effective_effort = if continuing {
        ReasoningEffort::Disabled
    } else {
        effort
    };
    // Thinking+tools requires returning prior reasoning_content. Force pass
    // whenever effort is on, or when continuing a turn that already thought.
    let effective_pass = match effort {
        ReasoningEffort::Disabled => {
            continuing && messages_have_tool_call_reasoning(&request.messages)
        }
        _ => true,
    };
    let _ = pass_tool_reasoning; // settings flag is superseded by the rules above

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
        .map(|message| message_to_api_json(message, effective_pass))
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
        apply_thinking_effort(&mut body, effective_effort);
    }

    Value::Object(body)
}

/// True when this request already includes tool results after the latest real
/// user message — i.e. an agent-loop continuation, not the opening model call.
pub(super) fn is_tool_continuation(messages: &[crate::core::runtime::ChatMessage]) -> bool {
    for message in messages.iter().rev() {
        match message.role {
            Role::Tool => return true,
            Role::User if message.content.starts_with("[System]") => return true,
            Role::User => return false,
            _ => {}
        }
    }
    false
}

pub(super) fn messages_have_tool_call_reasoning(
    messages: &[crate::core::runtime::ChatMessage],
) -> bool {
    messages.iter().any(|message| {
        message
            .tool_calls
            .as_ref()
            .is_some_and(|calls| !calls.is_empty())
            && message
                .reasoning
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
    })
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

pub(super) fn message_to_api_json(message: &ChatMessage, pass_tool_reasoning: bool) -> Value {
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
                // DeepSeek requires reasoning_content on every tool-call assistant
                // message once thinking was used; use a space placeholder if empty.
                let reasoning = message
                    .reasoning
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(" ");
                payload
                    .as_object_mut()
                    .expect("assistant payload object")
                    .insert("reasoning_content".into(), json!(reasoning));
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

pub(super) fn non_empty_option(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
