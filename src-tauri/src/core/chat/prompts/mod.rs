//! 系统提示词与工具目录 — 源自 DeepSeek-Reasonix，适配 Peek overlay。
//!
//! 源文件位于 `src-tauri/prompts/*.md`，可用 `include_str!` 热更新需重新编译。

/// 完整 system prompt（稳定前缀，尽量不因轮次变化）。
pub const SYSTEM_PROMPT: &str = concat!(
    include_str!("../../../../prompts/system.md"),
    "\n\n",
    include_str!("../../../../prompts/context.md"),
    "\n\n",
    include_str!("../../../../prompts/policies.md"),
    "\n\n",
    include_str!("../../../../prompts/tools.md"),
);

/// LLM 历史压缩用的 system prompt（Reasonix `summarySystemPrompt`）。
/// 当前 Peek 使用机械折叠；接入 LLM 摘要压缩时直接使用此常量。
#[allow(dead_code)]
pub const COMPACT_SUMMARY_SYSTEM_PROMPT: &str =
    include_str!("../../../../prompts/compact-summary.md");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_includes_peek_identity_and_tools() {
        assert!(SYSTEM_PROMPT.contains("You are AltAltAi"));
        assert!(SYSTEM_PROMPT.contains("[Selection]"));
        assert!(SYSTEM_PROMPT.contains("update_tasks"));
        assert!(SYSTEM_PROMPT.contains("ask_user"));
        assert!(SYSTEM_PROMPT.contains("Infer proactively what will remain useful"));
        assert!(SYSTEM_PROMPT.contains("Call `search_memory` when the user refers to prior chats"));
    }

    #[test]
    fn compact_summary_has_required_headings() {
        assert!(COMPACT_SUMMARY_SYSTEM_PROMPT.contains("## Goal"));
        assert!(COMPACT_SUMMARY_SYSTEM_PROMPT.contains("## Pending & next step"));
    }
}
