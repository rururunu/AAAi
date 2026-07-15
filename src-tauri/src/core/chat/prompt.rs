use crate::core::chat::limits::{
    truncate_chars, ACTIVE_WINDOW_MAX_CHARS, CLIPBOARD_MAX_CHARS, CONTEXT_BLOCKS_TOTAL_MAX_CHARS,
    MEMORIES_MAX_CHARS, RULES_MAX_CHARS, SELECTED_FILES_MAX_CHARS,
};
use crate::core::runtime::{ChatMessage, ChatRequest, MessageStatus, RequestContext, Role};
use crate::models::settings::{AppLanguage, ReasoningLanguage};

use super::prompts::SYSTEM_PROMPT;

/// Prompt 组装偏好 — 来自设置，不进入稳定 system。
#[derive(Debug, Clone, Default)]
pub struct PromptPreferences {
    pub app_language: AppLanguage,
    pub reasoning_language: ReasoningLanguage,
}

/// AI Runtime Prompt 组装 — System → History → Context → User。
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build(
        request_id: &str,
        session_id: &str,
        history: &[ChatMessage],
        context: &RequestContext,
        project_rules: Option<&str>,
        recalled_memories: Option<&str>,
        provider: Option<String>,
        preferences: &PromptPreferences,
    ) -> ChatRequest {
        let mut messages = Vec::with_capacity(history.len() + 4);

        // System
        if !history.iter().any(|message| message.role == Role::System) {
            messages.push(system_message(session_id));
        }

        // Current workspace and captured context belong immediately after System.
        inject_context(&mut messages, session_id, context);
        inject_system_block(&mut messages, session_id, "rules", project_rules);
        inject_memories(&mut messages, session_id, recalled_memories);

        // History（排除 pending 的空 assistant）
        let (prior, current_user) = split_current_user(history);
        messages.extend(
            prior
                .into_iter()
                .filter(|message| !message.content.trim().is_empty()),
        );

        // 当前用户输入（含 transient 语言块）
        if let Some(mut user_message) = current_user {
            user_message.content = inject_language_blocks(&user_message.content, preferences);
            messages.push(user_message);
        }

        ChatRequest {
            request_id: request_id.to_string(),
            session_id: session_id.to_string(),
            messages,
            context: context.clone(),
            provider,
            stream: true,
            tools: std::sync::Arc::from([]),
            temperature: None,
            max_tokens: None,
        }
    }
}

fn inject_system_block(
    messages: &mut Vec<ChatMessage>,
    session_id: &str,
    kind: &str,
    block: Option<&str>,
) {
    let Some(content) = block.map(str::trim).filter(|content| !content.is_empty()) else {
        return;
    };
    let capped = truncate_chars(content, RULES_MAX_CHARS);
    messages.push(ChatMessage {
        id: format!("{kind}-{session_id}"),
        session_id: session_id.to_string(),
        role: Role::System,
        content: capped,
        reasoning: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 0,
    });
}

fn inject_memories(messages: &mut Vec<ChatMessage>, session_id: &str, memories: Option<&str>) {
    let Some(content) = memories
        .map(str::trim)
        .filter(|content| !content.is_empty())
    else {
        return;
    };
    let capped = truncate_chars(content, MEMORIES_MAX_CHARS);
    messages.push(ChatMessage {
        id: format!("memories-{session_id}"),
        session_id: session_id.to_string(),
        role: Role::System,
        content: capped,
        reasoning: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 0,
    });
}

fn system_message(session_id: &str) -> ChatMessage {
    ChatMessage {
        id: format!("system-{session_id}"),
        session_id: session_id.to_string(),
        role: Role::System,
        content: SYSTEM_PROMPT.to_string(),
        reasoning: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 0,
    }
}

fn split_current_user(history: &[ChatMessage]) -> (Vec<ChatMessage>, Option<ChatMessage>) {
    if history.is_empty() {
        return (Vec::new(), None);
    }

    if history
        .last()
        .is_some_and(|message| message.role == Role::User)
    {
        let prior = history[..history.len() - 1].to_vec();
        let current = history.last().cloned();
        return (prior, current);
    }

    (history.to_vec(), None)
}

fn inject_context(messages: &mut Vec<ChatMessage>, session_id: &str, context: &RequestContext) {
    let mut blocks = Vec::new();

    if let Some(workspace) = &context.workspace {
        blocks.push(format!(
            "[Current Workspace]\nName: {}\nRoot Directory: {}\nTreat this as the exact active project. All file operations must use this root. When asked which workspace is active, answer with this name and root; never infer another workspace from the application identity, active window, or conversation history.",
            workspace.name, workspace.root
        ));
    }
    if !context.selected_files.is_empty() {
        let files = truncate_chars(
            &context.selected_files.join("\n"),
            SELECTED_FILES_MAX_CHARS,
        );
        blocks.push(format!(
            "[Selected Files]\nPaths are relative to the current workspace root.\n{files}"
        ));
    }
    if let Some(active_window) = non_empty(&context.active_window) {
        let capped = truncate_chars(&active_window, ACTIVE_WINDOW_MAX_CHARS);
        blocks.push(format!("[Active Window]\n{capped}"));
    }
    if let Some(clipboard) = non_empty(&context.clipboard) {
        let capped = truncate_chars(&clipboard, CLIPBOARD_MAX_CHARS);
        blocks.push(format!("[Clipboard]\n{capped}"));
    }

    if blocks.is_empty() {
        return;
    }

    let mut content = String::new();
    for block in blocks {
        let next = if content.is_empty() {
            block
        } else {
            format!("{content}\n\n{block}")
        };
        if next.chars().count() > CONTEXT_BLOCKS_TOTAL_MAX_CHARS {
            content = truncate_chars(&next, CONTEXT_BLOCKS_TOTAL_MAX_CHARS);
            break;
        }
        content = next;
    }

    messages.push(ChatMessage {
        id: format!("context-{session_id}-{}", messages.len()),
        session_id: session_id.to_string(),
        role: Role::System,
        content,
        reasoning: None,
        tool_activities: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        status: MessageStatus::Done,
        timestamp: 0,
    });
}

fn inject_language_blocks(content: &str, preferences: &PromptPreferences) -> String {
    let mut blocks = Vec::new();

    if let Some(block) = reasoning_language_block(preferences, content) {
        blocks.push(block);
    }
    if let Some(block) = response_language_block(preferences) {
        blocks.push(block);
    }

    if blocks.is_empty() {
        return content.to_string();
    }

    format!("{}\n\n{}", blocks.join("\n\n"), content)
}

fn reasoning_language_block(preferences: &PromptPreferences, user_content: &str) -> Option<String> {
    let mode = match preferences.reasoning_language {
        ReasoningLanguage::Auto => {
            if infer_chinese_user_text(user_content) {
                "zh"
            } else {
                return None;
            }
        }
        ReasoningLanguage::Zh => "zh",
        ReasoningLanguage::En => "en",
    };

    Some(match mode {
        "zh" => "<reasoning-language>\n可见推理/思考文本偏好：当模型服务暴露可见推理或思考文本时，请使用简体中文。代码、标识符、文件路径、shell 命令和未翻译的技术术语保持原文。此偏好不会覆盖用户对最终回答语言的明确要求。\n</reasoning-language>".to_string(),
        _ => "<reasoning-language>\nVisible reasoning/thinking text preference: use English when the provider exposes reasoning text. Keep code, identifiers, file paths, shell commands, and untranslated technical terms in their original form. This preference does not override an explicit user request for the final answer language.\n</reasoning-language>".to_string(),
    })
}

fn response_language_block(preferences: &PromptPreferences) -> Option<String> {
    let mode = match preferences.reasoning_language {
        ReasoningLanguage::Auto => match preferences.app_language {
            AppLanguage::ZhCn => "zh",
            AppLanguage::EnUs => "en",
            AppLanguage::JaJp => "ja",
            AppLanguage::RuRu => "ru",
            AppLanguage::DeDe => "de",
            AppLanguage::FrFr => "fr",
            AppLanguage::KoKr => "ko",
        },
        ReasoningLanguage::Zh => "zh",
        ReasoningLanguage::En => "en",
    };

    Some(match mode {
        "zh" => "<response-language>\nFinal answer language preference: use Simplified Chinese for user-facing replies unless the user explicitly asks for another language. Keep code, identifiers, file paths, shell commands, and untranslated technical terms in their original form.\n</response-language>".to_string(),
        "ja" => response_language_instruction("Japanese"),
        "ru" => response_language_instruction("Russian"),
        "de" => response_language_instruction("German"),
        "fr" => response_language_instruction("French"),
        "ko" => response_language_instruction("Korean"),
        _ => response_language_instruction("English"),
    })
}

fn response_language_instruction(language: &str) -> String {
    format!("<response-language>\nFinal answer language preference: use {language} for user-facing replies unless the user explicitly asks for another language. Keep code, identifiers, file paths, shell commands, and untranslated technical terms in their original form.\n</response-language>")
}

fn infer_chinese_user_text(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(ch,
            '\u{4E00}'..='\u{9FFF}'
                | '\u{3400}'..='\u{4DBF}'
                | '\u{F900}'..='\u{FAFF}'
        )
    })
}

fn non_empty(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|text| text.trim())
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_reasoning_language_for_chinese_user_text() {
        let prefs = PromptPreferences {
            app_language: AppLanguage::EnUs,
            reasoning_language: ReasoningLanguage::Auto,
        };
        let content = inject_language_blocks("帮我解释这段代码", &prefs);
        assert!(content.starts_with("<reasoning-language>"));
        assert!(content.contains("<response-language>"));
        assert!(content.ends_with("帮我解释这段代码"));
    }

    #[test]
    fn auto_reasoning_skips_english_only_prompt() {
        let prefs = PromptPreferences {
            app_language: AppLanguage::EnUs,
            reasoning_language: ReasoningLanguage::Auto,
        };
        let content = inject_language_blocks("Explain this snippet", &prefs);
        assert!(!content.contains("<reasoning-language>"));
        assert!(content.starts_with("<response-language>"));
    }

    #[test]
    fn workspace_context_identifies_the_exact_active_directory() {
        let context = RequestContext {
            workspace: Some(crate::core::runtime::request::WorkspaceContext {
                name: "Customer App".to_string(),
                root: r"C:\projects\customer-app".to_string(),
            }),
            active_window: Some("Peek - source code".to_string()),
            ..RequestContext::default()
        };
        let mut messages = Vec::new();

        inject_context(&mut messages, "session-1", &context);

        assert_eq!(messages.len(), 1);
        let content = &messages[0].content;
        assert!(content.contains(
            "[Current Workspace]\nName: Customer App\nRoot Directory: C:\\projects\\customer-app"
        ));
        assert!(content.contains("never infer another workspace"));
    }

    #[test]
    fn workspace_context_precedes_history_and_current_user() {
        let message = |id: &str, role: Role, content: &str| ChatMessage {
            id: id.to_string(),
            session_id: "session-1".to_string(),
            role,
            content: content.to_string(),
            reasoning: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
        };
        let history = vec![
            message("old-user", Role::User, "old question"),
            message("old-assistant", Role::Assistant, "old answer"),
            message("current-user", Role::User, "new question"),
        ];
        let context = RequestContext {
            workspace: Some(crate::core::runtime::request::WorkspaceContext {
                name: "Peek".to_string(),
                root: r"D:\Code\Peek".to_string(),
            }),
            ..RequestContext::default()
        };

        let request = PromptBuilder::build(
            "request-1",
            "session-1",
            &history,
            &context,
            None,
            None,
            None,
            &PromptPreferences::default(),
        );

        assert!(request.messages[0].id.starts_with("system-"));
        assert!(request.messages[1].id.starts_with("context-"));
        assert_eq!(request.messages[2].id, "old-user");
        assert_eq!(request.messages.last().unwrap().id, "current-user");
    }

    #[test]
    fn recalled_memories_are_injected_as_untrusted_system_context() {
        let request = PromptBuilder::build(
            "request-1",
            "session-1",
            &[],
            &RequestContext::default(),
            None,
            Some("<relevant-memories>\nUses pnpm\n</relevant-memories>"),
            None,
            &PromptPreferences::default(),
        );

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[1].role, Role::System);
        assert!(request.messages[1].id.starts_with("memories-"));
        assert!(request.messages[1].content.contains("Uses pnpm"));
    }

    #[test]
    fn project_rules_are_injected_as_system_context() {
        let request = PromptBuilder::build(
            "request-1",
            "session-1",
            &[],
            &RequestContext::default(),
            Some("<project-rules>\nUse pnpm\n</project-rules>"),
            None,
            None,
            &PromptPreferences::default(),
        );

        assert_eq!(request.messages[1].id, "rules-session-1");
        assert!(request.messages[1].content.contains("Use pnpm"));
    }

    #[test]
    fn clipboard_context_is_hard_capped() {
        let context = RequestContext {
            clipboard: Some("Z".repeat(CLIPBOARD_MAX_CHARS + 500)),
            ..RequestContext::default()
        };
        let mut messages = Vec::new();
        inject_context(&mut messages, "session-1", &context);
        let content = &messages[0].content;
        assert!(content.chars().count() <= CONTEXT_BLOCKS_TOTAL_MAX_CHARS);
        assert!(content.contains('…') || content.contains("[Clipboard]"));
        let clipboard_body = content
            .split("[Clipboard]\n")
            .nth(1)
            .unwrap_or_default();
        assert!(clipboard_body.chars().count() <= CLIPBOARD_MAX_CHARS + 1);
    }
}
