//! Prompt assembly with fixed slots for KV-cache prefix stability.

mod language;
mod slots;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use crate::core::runtime::{ChatMessage, ChatRequest, RequestContext, Role};
use crate::models::settings::{AppLanguage, ReasoningLanguage};

use language::inject_language_blocks;
use slots::{
    inject_context, inject_memories, inject_optional_policy_suffix, inject_system_block,
    split_current_user, system_message,
};

/// Prompt 组装偏好 — 来自设置，不进入稳定 system。
#[derive(Debug, Clone, Default)]
pub struct PromptPreferences {
    pub app_language: AppLanguage,
    pub reasoning_language: ReasoningLanguage,
    pub collaboration_models: Vec<String>,
    /// Inject the optional minimal-coding ladder when enabled in Settings.
    pub minimal_coding: bool,
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

/// AI Runtime Prompt 组装 — 固定槽位以保护 KV cache 前缀稳定性：
///
/// ```text
/// [0] SYSTEM_PROMPT
/// [1] workspace / captured context
/// [2] project rules
/// [3] recalled memories
/// [4] optional policy suffix (collab / minimal-coding / …)
/// [5..] history + current user
/// ```
///
/// Optional strategy toggles only populate slot [4]; they never insert ahead of
/// context/rules/memories, so enabling/disabling them cannot shift the stable
/// prefix.
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
        let mut messages = Vec::with_capacity(history.len() + 6);

        // [0] Stable system — never moves.
        if !history.iter().any(|message| message.role == Role::System) {
            messages.push(system_message(session_id));
        }

        // [1]–[3] Core context slots (order locked).
        inject_context(&mut messages, session_id, context);
        inject_system_block(&mut messages, session_id, "rules", project_rules);
        inject_memories(&mut messages, session_id, recalled_memories);

        // [4] Optional policy suffix — toggles only hang here.
        inject_optional_policy_suffix(
            &mut messages,
            session_id,
            &preferences.collaboration_models,
            preferences.minimal_coding,
        );

        // [5..] History（排除 pending 的空 assistant）
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
