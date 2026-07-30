use crate::core::chat::limits::{
    truncate_chars, ACTIVE_FILE_MAX_CHARS, ACTIVE_WINDOW_MAX_CHARS, CLIPBOARD_MAX_CHARS,
    CONTEXT_BLOCKS_TOTAL_MAX_CHARS, GIT_STATUS_MAX_CHARS, IDE_SELECTION_MAX_CHARS,
    LAST_SHELL_EXECUTION_MAX_CHARS, MEMORIES_MAX_CHARS, RULES_MAX_CHARS, SELECTED_FILES_MAX_CHARS,
};
use crate::core::runtime::{ChatMessage, ChatRequest, MessageStatus, RequestContext, Role};
use crate::models::settings::{AppLanguage, ReasoningLanguage};

use super::prompts::{MULTI_MODEL_COLLABORATION_PROMPT, SYSTEM_PROMPT};

/// Prompt 组装偏好 — 来自设置，不进入稳定 system。
#[derive(Debug, Clone, Default)]
pub struct PromptPreferences {
    pub app_language: AppLanguage,
    pub reasoning_language: ReasoningLanguage,
    pub collaboration_models: Vec<String>,
}

pub struct PromptBuildInput<'a> {
    pub request_id: &'a str,
    pub session_id: &'a str,
    pub history: &'a [ChatMessage],
    pub context: &'a RequestContext,
    pub project_rules: Option<&'a str>,
    pub recalled_memories: Option<&'a str>,
    pub provider: Option<String>,
    pub preferences: &'a PromptPreferences,
}

/// AI Runtime Prompt 组装 — System → History → Context → User。
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build(input: PromptBuildInput<'_>) -> ChatRequest {
        let PromptBuildInput {
            request_id,
            session_id,
            history,
            context,
            project_rules,
            recalled_memories,
            provider,
            preferences,
        } = input;
        let mut messages = Vec::with_capacity(history.len() + 4);

        // System
        if !history.iter().any(|message| message.role == Role::System) {
            messages.push(system_message(session_id));
        }

        // Current workspace and captured context belong immediately after System.
        inject_context(&mut messages, session_id, context);
        inject_system_block(&mut messages, session_id, "rules", project_rules);
        inject_memories(&mut messages, session_id, recalled_memories);
        inject_collaboration_models(&mut messages, session_id, &preferences.collaboration_models);

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

fn inject_collaboration_models(
    messages: &mut Vec<ChatMessage>,
    session_id: &str,
    models: &[String],
) {
    if models.is_empty() {
        return;
    }
    let list = models
        .iter()
        .map(|model| format!("- `{model}`"))
        .collect::<Vec<_>>()
        .join("\n");
    let content = MULTI_MODEL_COLLABORATION_PROMPT.replace("{{MODELS}}", &list);
    inject_system_block(messages, session_id, "collaboration-models", Some(&content));
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

    if let Some(ide) = &context.ide_context {
        let mut lines = vec![format!("IDE:\n{}", ide_display_name(&ide.ide))];
        if let Some(workspace) = &ide.workspace {
            lines.push(format!("Workspace:\n{}", workspace.display()));
        }
        if let Some(active_file) = &ide.active_file {
            lines.push(format!("Active File:\n{}", active_file.display()));
        }
        if let Some(language) = ide
            .language
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("Language:\n{language}"));
        }
        if let Some(cursor) = &ide.cursor {
            lines.push(format!(
                "Cursor:\nLine {}, Column {}",
                cursor.line, cursor.column
            ));
        }
        if let Some(selection) = ide
            .selection
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!(
                "Selection:\n{}",
                truncate_chars(selection, IDE_SELECTION_MAX_CHARS)
            ));
        }
        blocks.push(format!("[IDE Context]\n{}", lines.join("\n\n")));
    }
    if let Some(workspace) = &context.workspace {
        blocks.push(format!(
            "[Current Workspace]\nName: {}\nRoot Directory: {}\nTreat this as the exact active project. All file operations must use this root. When asked which workspace is active, answer with this name and root; never infer another workspace from the application identity, active window, or conversation history.",
            workspace.name, workspace.root
        ));
    }
    if !context.selected_files.is_empty() {
        let files = truncate_chars(&context.selected_files.join("\n"), SELECTED_FILES_MAX_CHARS);
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
    if let Some(active_file) = non_empty(&context.active_file) {
        let capped = truncate_chars(&active_file, ACTIVE_FILE_MAX_CHARS);
        blocks.push(format!("[Active File]\n{capped}"));
    }
    if let Some(git_status) = non_empty(&context.git_status) {
        let capped = truncate_chars(&git_status, GIT_STATUS_MAX_CHARS);
        blocks.push(format!("[Git Status]\n{capped}"));
    }
    if let Some(shell) = non_empty(&context.last_shell_execution) {
        let capped = truncate_chars(&shell, LAST_SHELL_EXECUTION_MAX_CHARS);
        blocks.push(format!("[Last Agent Shell Execution]\n{capped}"));
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

fn ide_display_name(ide: &str) -> String {
    match ide.trim().to_ascii_lowercase().as_str() {
        "vscode" | "visual studio code" => "VSCode".to_string(),
        "idea" | "intellij" | "intellij idea" => "IntelliJ IDEA".to_string(),
        _ => ide.trim().to_string(),
    }
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
    fn collaboration_models_are_injected_only_when_configured() {
        let context = RequestContext::default();
        let preferences = PromptPreferences {
            collaboration_models: vec!["model-a".into(), "model-b".into()],
            ..PromptPreferences::default()
        };
        let request = PromptBuilder::build(PromptBuildInput {
            request_id: "request",
            session_id: "session",
            history: &[],
            context: &context,
            project_rules: None,
            recalled_memories: None,
            provider: None,
            preferences: &preferences,
        });
        let collaboration = request
            .messages
            .iter()
            .find(|message| message.id.starts_with("collaboration-models-"));
        assert!(collaboration.is_some_and(|message| {
            message.content.contains("`model-a`")
                && message.content.contains("`model-b`")
                && message.content.contains("Routing policy")
                && message.content.contains("exact selected model ID")
                && message.content.contains("Never omit `model`")
                && message
                    .content
                    .contains("user-selected list defines eligibility only")
                && !message.content.contains("general-purpose candidate")
        }));

        let default_preferences = PromptPreferences::default();
        let without_models = PromptBuilder::build(PromptBuildInput {
            request_id: "request",
            session_id: "session",
            history: &[],
            context: &context,
            project_rules: None,
            recalled_memories: None,
            provider: None,
            preferences: &default_preferences,
        });
        assert!(!without_models
            .messages
            .iter()
            .any(|message| message.id.starts_with("collaboration-models-")));
    }

    #[test]
    fn injects_reasoning_language_for_chinese_user_text() {
        let prefs = PromptPreferences {
            app_language: AppLanguage::EnUs,
            reasoning_language: ReasoningLanguage::Auto,
            ..PromptPreferences::default()
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
            ..PromptPreferences::default()
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
    fn injects_environment_context_into_agent_prompt() {
        let context = RequestContext {
            git_status: Some("## main\n M src/main.rs".to_string()),
            last_shell_execution: Some(
                "Command: cargo test\nWorking Directory: C:\\work\nResult:\nexit_code: 0"
                    .to_string(),
            ),
            ..RequestContext::default()
        };
        let mut messages = Vec::new();

        inject_context(&mut messages, "session-environment", &context);

        assert_eq!(messages.len(), 1);
        assert!(messages[0].content.contains("[Git Status]\n## main"));
        assert!(messages[0]
            .content
            .contains("[Last Agent Shell Execution]\nCommand: cargo test"));
    }

    #[test]
    fn injects_ide_context_into_agent_prompt() {
        use crate::core::context::models::{CursorPosition, IDEContext};
        use std::path::PathBuf;

        let context = RequestContext {
            ide_context: Some(IDEContext {
                ide: "vscode".to_string(),
                active_file: Some(PathBuf::from(r"C:\project\src\main.rs")),
                workspace: Some(PathBuf::from(r"C:\project")),
                language: Some("rust".to_string()),
                selection: Some("fn main() {}".to_string()),
                cursor: Some(CursorPosition {
                    line: 15,
                    column: 5,
                }),
            }),
            ..RequestContext::default()
        };
        let mut messages = Vec::new();

        inject_context(&mut messages, "session-ide", &context);

        let content = &messages[0].content;
        assert!(content.contains("[IDE Context]"));
        assert!(content.contains("IDE:\nVSCode"));
        assert!(content.contains("Language:\nrust"));
        assert!(content.contains("Line 15, Column 5"));
        assert!(content.contains("Selection:\nfn main() {}"));
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

        let preferences = PromptPreferences::default();
        let request = PromptBuilder::build(PromptBuildInput {
            request_id: "request-1",
            session_id: "session-1",
            history: &history,
            context: &context,
            project_rules: None,
            recalled_memories: None,
            provider: None,
            preferences: &preferences,
        });

        assert!(request.messages[0].id.starts_with("system-"));
        assert!(request.messages[1].id.starts_with("context-"));
        assert_eq!(request.messages[2].id, "old-user");
        assert_eq!(request.messages.last().unwrap().id, "current-user");
    }

    #[test]
    fn recalled_memories_are_injected_as_untrusted_system_context() {
        let context = RequestContext::default();
        let preferences = PromptPreferences::default();
        let request = PromptBuilder::build(PromptBuildInput {
            request_id: "request-1",
            session_id: "session-1",
            history: &[],
            context: &context,
            project_rules: None,
            recalled_memories: Some("<relevant-memories>\nUses pnpm\n</relevant-memories>"),
            provider: None,
            preferences: &preferences,
        });

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[1].role, Role::System);
        assert!(request.messages[1].id.starts_with("memories-"));
        assert!(request.messages[1].content.contains("Uses pnpm"));
    }

    #[test]
    fn project_rules_are_injected_as_system_context() {
        let context = RequestContext::default();
        let preferences = PromptPreferences::default();
        let request = PromptBuilder::build(PromptBuildInput {
            request_id: "request-1",
            session_id: "session-1",
            history: &[],
            context: &context,
            project_rules: Some("<project-rules>\nUse pnpm\n</project-rules>"),
            recalled_memories: None,
            provider: None,
            preferences: &preferences,
        });

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
        let clipboard_body = content.split("[Clipboard]\n").nth(1).unwrap_or_default();
        assert!(clipboard_body.chars().count() <= CLIPBOARD_MAX_CHARS + 1);
    }
}
