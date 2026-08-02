use serde_json::Value;

pub fn extract_description_from_body(body: &str) -> Option<String> {
    let trimmed = strip_bom(body.trim());
    if trimmed.is_empty() {
        return None;
    }

    if looks_like_sse(trimmed) {
        if let Some(text) = extract_from_sse(trimmed) {
            return Some(text);
        }
    }

    if let Some(text) = extract_from_json_text(trimmed) {
        return Some(text);
    }

    if looks_like_plain_description(trimmed) {
        return Some(trimmed.to_string());
    }

    None
}

pub fn parse_multimodal_description_body(body: &str) -> Result<String, String> {
    let trimmed = strip_bom(body.trim());
    if trimmed.is_empty() {
        return Err(
            "Multimodal model returned an empty body (HTTP 200 with no content). Check the Base URL and model name, or try again with a proxy enabled."
                .into(),
        );
    }

    if looks_like_html(trimmed) {
        return Err(html_instead_of_json_error(trimmed, None, None));
    }

    if let Some(value) = parse_json_value(trimmed) {
        if let Some(err) = extract_error_message(&value) {
            return Err(format!("Multimodal model returned an error: {err}"));
        }
        if let Some(refusal) = extract_refusal(&value) {
            return Err(format!("Multimodal model refused to answer: {refusal}"));
        }
        if let Some(finish_reason) = empty_content_finish_reason(&value) {
            return Err(format!(
                "Multimodal model returned empty content (finish_reason={finish_reason}). The image may be unsupported, filtered, or the model produced no text. Debug: {}. Snippet: {}",
                diagnose_body(trimmed),
                truncate_body(trimmed, 480)
            ));
        }
    }

    if let Some(text) = extract_description_from_body(body) {
        if !text.trim().is_empty() {
            return Ok(text);
        }
    }

    Err(format!(
        "Failed to extract an image description from the multimodal response. Debug: {}. Snippet: {}",
        diagnose_body(trimmed),
        truncate_body(trimmed, 480)
    ))
}

fn looks_like_html(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    let trimmed = lower.trim_start();
    trimmed.starts_with("<!doctype html")
        || trimmed.starts_with("<html")
        || trimmed.starts_with("<head")
        || (trimmed.starts_with('<')
            && (lower.contains("<html") || lower.contains("<body") || lower.contains("<title")))
}

pub fn body_looks_like_html(body: &str) -> bool {
    looks_like_html(strip_bom(body.trim()))
}

pub fn html_instead_of_json_error(
    body: &str,
    request_url: Option<&str>,
    content_type: Option<&str>,
) -> String {
    let trimmed = strip_bom(body.trim());
    let mut parts = vec![
        "Multimodal provider returned an HTML page instead of a JSON/SSE API response.".to_string(),
    ];
    if let Some(url) = request_url.filter(|u| !u.is_empty()) {
        parts.push(format!("Request URL: {url}"));
        if url.contains("/chat/completions") && !url.contains("/v1/") && !url.contains("/v1beta/") {
            parts.push(
                "Likely cause: the chat-completions path is missing /v1 (common for NewAPI). Peek should call .../v1/chat/completions."
                    .into(),
            );
        } else {
            parts.push(
                "The endpoint returned the provider's web UI/HTML instead of the API. Check proxy/network reachability for the API path."
                    .into(),
            );
        }
    } else {
        parts.push(
            "The endpoint returned HTML instead of the API payload. For NewAPI/OpenAI-compatible relays the path should be .../v1/chat/completions."
                .into(),
        );
    }
    if let Some(ct) = content_type.filter(|c| !c.is_empty()) {
        parts.push(format!("Content-Type: {ct}"));
    }
    parts.push(format!("HTML hint: {}", html_error_hint(trimmed)));
    parts.push(format!("Snippet: {}", truncate_body(trimmed, 280)));
    parts.join(" ")
}

fn html_error_hint(body: &str) -> String {
    let lower = body.to_ascii_lowercase();
    if lower.contains("cloudflare") || lower.contains("cf-ray") {
        return "Cloudflare/challenge page".into();
    }
    if lower.contains("just a moment") {
        return "bot-protection interstitial".into();
    }
    if lower.contains("401") || lower.contains("unauthorized") {
        return "unauthorized HTML page".into();
    }
    if lower.contains("403") || lower.contains("forbidden") {
        return "forbidden HTML page".into();
    }
    if lower.contains("404") || lower.contains("not found") {
        return "not-found HTML page".into();
    }
    if lower.contains("502") || lower.contains("bad gateway") {
        return "bad-gateway HTML page".into();
    }
    if lower.contains("nginx") {
        return "nginx HTML page".into();
    }
    if let Some(title) = extract_html_title(body) {
        return format!("title=\"{title}\"");
    }
    "unrecognized HTML".into()
}

fn extract_html_title(body: &str) -> Option<String> {
    let lower = body.to_ascii_lowercase();
    let start = lower.find("<title")?;
    let after = &body[start..];
    let after_lower = after.to_ascii_lowercase();
    let open_end = after_lower.find('>')?;
    let rest = &after[open_end + 1..];
    let rest_lower = rest.to_ascii_lowercase();
    let close = rest_lower.find("</title>")?;
    let title = rest[..close].trim();
    if title.is_empty() {
        None
    } else {
        Some(collapse_ws(title))
    }
}

fn collapse_ws(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_bom(body: &str) -> &str {
    body.strip_prefix('\u{feff}').unwrap_or(body)
}

fn looks_like_sse(body: &str) -> bool {
    body.lines().any(|line| {
        let line = line.trim();
        line.starts_with("data:") || line.starts_with("data: ")
    })
}

fn parse_json_value(body: &str) -> Option<Value> {
    let slice = extract_json_slice(body)?;
    let value = serde_json::from_str::<Value>(slice).ok()?;
    Some(unwrap_stringified_json(value))
}

fn extract_from_json_text(body: &str) -> Option<String> {
    let value = parse_json_value(body)?;
    if extract_error_message(&value).is_some() {
        return None;
    }
    extract_from_json_value(&value)
}

fn unwrap_stringified_json(value: Value) -> Value {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                if let Ok(inner) = serde_json::from_str::<Value>(trimmed) {
                    return unwrap_stringified_json(inner);
                }
            }
            Value::String(text)
        }
        Value::Object(map) => {
            if let Some(Value::String(text)) = map.get("data") {
                let trimmed = text.trim();
                if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    if let Ok(inner) = serde_json::from_str::<Value>(trimmed) {
                        return unwrap_stringified_json(inner);
                    }
                }
            }
            Value::Object(map)
        }
        other => other,
    }
}

fn extract_json_slice(body: &str) -> Option<&str> {
    let trimmed = body.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Some(trimmed);
    }

    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return Some(&trimmed[start..=end]);
            }
        }
    }
    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            if end > start {
                return Some(&trimmed[start..=end]);
            }
        }
    }
    None
}

fn looks_like_plain_description(body: &str) -> bool {
    if body.starts_with('{')
        || body.starts_with('[')
        || looks_like_sse(body)
        || looks_like_html(body)
    {
        return false;
    }
    body.chars().any(|c| !c.is_whitespace())
}

fn extract_from_sse(body: &str) -> Option<String> {
    let mut out = String::new();
    for line in body.lines() {
        let line = line.trim();
        if !line.starts_with("data:") {
            continue;
        }
        let payload = line["data:".len()..].trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<Value>(payload) {
            let value = unwrap_stringified_json(parsed);
            if let Some(chunk) = extract_from_json_value(&value) {
                out.push_str(&chunk);
                continue;
            }
        } else if looks_like_plain_description(payload) {
            out.push_str(payload);
        }
    }
    non_empty(out)
}

pub fn extract_from_json_value(value: &Value) -> Option<String> {
    const POINTER_PATHS: &[&str] = &[
        "/choices/0/message/content",
        "/choices/0/message/reasoning_content",
        "/choices/0/message/reasoning",
        "/choices/0/message/multi_content",
        "/choices/0/message/contents",
        "/choices/0/text",
        "/choices/0/delta/content",
        "/choices/0/delta/reasoning_content",
        "/choices/0/delta/text",
        "/output_text",
        "/output/0/content/0/text",
        "/output/1/content/0/text",
        "/output/text",
        "/output/response",
        "/result",
        "/result/output_text",
        "/result/text",
        "/data/choices/0/message/content",
        "/data/choices/0/message/reasoning_content",
        "/data/choices/0/delta/content",
        "/data/choices/0/text",
        "/response/output_text",
        "/response/choices/0/message/content",
        "/response/text",
        "/message/content",
        "/content/0/text",
        "/content/text",
        "/text",
        "/answer",
        "/output",
        "/data/output_text",
        "/data/output/text",
        "/data/text",
        "/data/content/text",
        "/data/content/0/text",
        "/data/content",
        "/data/result",
        "/data/result/output_text",
        "/data/response",
        "/data/answer",
        "/data",
    ];

    for path in POINTER_PATHS {
        if let Some(text) = value
            .pointer(path)
            .and_then(collect_text_from_content_value)
        {
            return Some(text);
        }
    }

    if let Some(text) = value
        .pointer("/response/candidates/0/content/parts")
        .or_else(|| value.pointer("/candidates/0/content/parts"))
        .and_then(collect_text_from_content_value)
    {
        return Some(text);
    }

    collect_text_from_content_value(value)
}

fn empty_content_finish_reason(value: &Value) -> Option<String> {
    let finish = value
        .pointer("/choices/0/finish_reason")
        .or_else(|| value.pointer("/data/choices/0/finish_reason"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();

    let has_text = extract_from_json_value(value).is_some();
    if has_text {
        return None;
    }
    Some(finish)
}

fn extract_error_message(value: &Value) -> Option<String> {
    const ERROR_PATHS: &[&str] = &[
        "/error/message",
        "/error/msg",
        "/error",
        "/data/error/message",
        "/data/error",
    ];

    if !has_meaningful_error(value) {
        return None;
    }

    for path in ERROR_PATHS {
        if let Some(text) = value.pointer(path).and_then(|v| match v {
            Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            Value::Object(map) => map
                .get("message")
                .or_else(|| map.get("msg"))
                .and_then(|m| m.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            _ => None,
        }) {
            return Some(text);
        }
    }
    None
}

fn has_meaningful_error(value: &Value) -> bool {
    let error_field = match value.get("error") {
        Some(Value::Null) => false,
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(Value::Object(map)) => !map.is_empty(),
        Some(Value::Bool(false)) => false,
        Some(_) => true,
        None => false,
    };
    let data_error = match value.pointer("/data/error") {
        Some(Value::Null) => false,
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(Value::Object(map)) => !map.is_empty(),
        Some(_) => true,
        None => false,
    };
    let failed_success = value
        .get("success")
        .and_then(|v| v.as_bool())
        .is_some_and(|ok| !ok);
    error_field || data_error || failed_success
}

fn extract_refusal(value: &Value) -> Option<String> {
    value
        .pointer("/choices/0/message/refusal")
        .or_else(|| value.pointer("/data/choices/0/message/refusal"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn collect_text_from_content_value(value: &Value) -> Option<String> {
    non_empty(collect_text(value))
}

fn collect_text(value: &Value) -> String {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            // Ignore stringified JSON envelopes; caller unwraps those separately.
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                if let Ok(inner) = serde_json::from_str::<Value>(trimmed) {
                    return collect_text(&inner);
                }
            }
            trimmed.to_string()
        }
        Value::Array(items) => items
            .iter()
            .map(collect_text)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(map) => {
            for key in [
                "text",
                "output_text",
                "reasoning_content",
                "reasoning",
                "answer",
                "response",
                "refusal",
            ] {
                if let Some(text) = map.get(key).and_then(|v| v.as_str()) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        return trimmed.to_string();
                    }
                }
            }
            if let Some(text) = map
                .get("type")
                .and_then(|v| v.as_str())
                .filter(|t| {
                    matches!(
                        *t,
                        "text"
                            | "output_text"
                            | "input_text"
                            | "text_delta"
                            | "content_block_delta"
                    )
                })
                .and_then(|_| {
                    map.get("text")
                        .or_else(|| map.get("content"))
                        .and_then(|v| v.as_str())
                })
            {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
            for key in [
                "content",
                "multi_content",
                "contents",
                "parts",
                "message",
                "delta",
                "choices",
                "candidates",
                "output",
                "response",
                "result",
                "data",
                "answer",
            ] {
                if let Some(nested) = map.get(key) {
                    let collected = collect_text(nested);
                    if !collected.is_empty() {
                        return collected;
                    }
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}

fn non_empty(text: String) -> Option<String> {
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

fn truncate_body(body: &str, max_len: usize) -> String {
    let trimmed = strip_bom(body.trim());
    if trimmed.is_empty() {
        return "(empty)".into();
    }
    let mut end = trimmed.len().min(max_len);
    while end > 0 && !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    if end >= trimmed.len() {
        trimmed.to_string()
    } else {
        format!("{}…", &trimmed[..end])
    }
}

fn diagnose_body(body: &str) -> String {
    let trimmed = strip_bom(body.trim());
    let mut parts = vec![format!("bytes={}", trimmed.len())];
    if looks_like_sse(trimmed) {
        let data_lines = trimmed
            .lines()
            .filter(|line| line.trim().starts_with("data:"))
            .count();
        parts.push(format!("format=sse;sse_data_lines={data_lines}"));
    }
    if let Some(value) = parse_json_value(trimmed) {
        if let Some(obj) = value.as_object() {
            let keys: Vec<&str> = obj.keys().take(12).map(|k| k.as_str()).collect();
            parts.push(format!("keys={}", keys.join(",")));
        }
        if let Some(choices) = value
            .pointer("/choices")
            .or_else(|| value.pointer("/data/choices"))
            .and_then(|v| v.as_array())
        {
            parts.push(format!("choices={}", choices.len()));
            if let Some(first) = choices.first() {
                if let Some(finish) = first.get("finish_reason").and_then(|v| v.as_str()) {
                    parts.push(format!("finish_reason={finish}"));
                }
                if let Some(message) = first.get("message") {
                    parts.push(format!(
                        "message.content={}",
                        describe_json_shape(message.get("content"))
                    ));
                }
                if let Some(delta) = first.get("delta") {
                    parts.push(format!(
                        "delta.content={}",
                        describe_json_shape(delta.get("content"))
                    ));
                }
            }
        }
    } else if trimmed.starts_with('<') {
        parts.push("format=html".into());
    } else {
        parts.push("format=non-json".into());
    }
    parts.join("; ")
}

fn describe_json_shape(value: Option<&Value>) -> String {
    match value {
        None => "missing".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::String(s)) => format!("string(len={})", s.len()),
        Some(Value::Array(items)) => format!("array(len={})", items.len()),
        Some(Value::Object(_)) => "object".into(),
        Some(Value::Number(_)) => "number".into(),
        Some(Value::Bool(_)) => "bool".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_openai_string_content() {
        let body = json!({
            "choices": [{ "message": { "content": "a cat" } }]
        });
        let text = extract_description_from_body(&body.to_string()).expect("text");
        assert_eq!(text, "a cat");
    }

    #[test]
    fn parses_openai_part_array_content() {
        let body = json!({
            "choices": [{
                "message": {
                    "content": [
                        { "type": "text", "text": "line one" },
                        { "type": "text", "text": "line two" }
                    ]
                }
            }]
        });
        let text = extract_description_from_body(&body.to_string()).expect("text");
        assert_eq!(text, "line one\nline two");
    }

    #[test]
    fn parses_sse_stream() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"hel\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\ndata: [DONE]\n";
        let text = extract_description_from_body(body).expect("text");
        assert_eq!(text, "hello");
    }

    #[test]
    fn parses_gemini_candidate_parts() {
        let body = json!({
            "response": {
                "candidates": [{
                    "content": {
                        "parts": [{ "text": "office scene" }]
                    }
                }]
            }
        });
        let text = extract_description_from_body(&body.to_string()).expect("text");
        assert_eq!(text, "office scene");
    }

    #[test]
    fn empty_body_is_actionable() {
        let err = parse_multimodal_description_body("  ").unwrap_err();
        assert!(err.contains("empty body"));
    }

    #[test]
    fn parses_plain_text_body() {
        let text = extract_description_from_body("A red mug on a wooden table.").expect("text");
        assert_eq!(text, "A red mug on a wooden table.");
    }

    #[test]
    fn parses_bom_prefixed_json() {
        let body = format!(
            "\u{feff}{}",
            json!({ "choices": [{ "message": { "content": "bom ok" } }] })
        );
        let text = extract_description_from_body(&body).expect("text");
        assert_eq!(text, "bom ok");
    }

    #[test]
    fn parses_json_wrapped_in_noise() {
        let body = format!(
            "OK\n```json\n{}\n```",
            json!({ "choices": [{ "message": { "content": "wrapped" } }] })
        );
        let text = extract_description_from_body(&body).expect("text");
        assert_eq!(text, "wrapped");
    }

    #[test]
    fn surfaces_provider_error_object() {
        let body = json!({ "error": { "message": "invalid image" } }).to_string();
        let err = parse_multimodal_description_body(&body).unwrap_err();
        assert!(err.contains("invalid image"));
    }

    #[test]
    fn ignores_null_error_field() {
        let body = json!({
            "error": null,
            "choices": [{ "message": { "content": "still works" } }]
        })
        .to_string();
        let text = parse_multimodal_description_body(&body).expect("text");
        assert_eq!(text, "still works");
    }

    #[test]
    fn surfaces_refusal() {
        let body = json!({
            "choices": [{ "message": { "content": null, "refusal": "policy block" } }]
        })
        .to_string();
        let err = parse_multimodal_description_body(&body).unwrap_err();
        assert!(err.contains("policy block"));
    }

    #[test]
    fn surfaces_empty_content_finish_reason() {
        let body = json!({
            "choices": [{
                "message": { "content": "" },
                "finish_reason": "content_filter"
            }]
        })
        .to_string();
        let err = parse_multimodal_description_body(&body).unwrap_err();
        assert!(err.contains("finish_reason=content_filter"));
        assert!(err.contains("Debug:"));
    }

    #[test]
    fn parses_responses_api_output_text() {
        let body = json!({
            "output": [
                { "type": "reasoning", "summary": [] },
                {
                    "type": "message",
                    "content": [{ "type": "output_text", "text": "desk lamp" }]
                }
            ]
        });
        let text = extract_description_from_body(&body.to_string()).expect("text");
        assert_eq!(text, "desk lamp");
    }

    #[test]
    fn parses_stringified_data_payload() {
        let inner = json!({
            "choices": [{ "message": { "content": "nested ok" } }]
        })
        .to_string();
        let body = json!({ "data": inner }).to_string();
        let text = extract_description_from_body(&body).expect("text");
        assert_eq!(text, "nested ok");
    }

    #[test]
    fn parses_data_text_string() {
        let body = json!({ "code": 0, "data": "relay description" }).to_string();
        let text = extract_description_from_body(&body).expect("text");
        assert_eq!(text, "relay description");
    }

    #[test]
    fn surfaces_html_response_clearly() {
        let body = "<!DOCTYPE html><html><head><title>Cloudflare</title></head><body>Just a moment...</body></html>";
        let err = parse_multimodal_description_body(body).unwrap_err();
        assert!(err.contains("HTML"));
        assert!(err.contains("v1") || err.contains("API"));
        assert!(err.contains("Cloudflare") || err.contains("challenge") || err.contains("title="));
    }

    #[test]
    fn truncate_body_is_utf8_safe() {
        let body = "图片描述：这是一张测试图";
        let truncated = truncate_body(body, 10);
        assert!(truncated.ends_with('…') || truncated == body);
        assert!(
            truncated.is_char_boundary(truncated.trim_end_matches('…').len())
                || truncated.ends_with('…')
        );
    }
}
