//! Shared text/token limits for prompt assembly and tool loops.

/// Characters → estimated tokens (coarse).
pub const CHARS_PER_TOKEN: usize = 4;

pub const TOOL_OUTPUT_MAX_CHARS: usize = 12_000;
pub const CLIPBOARD_MAX_CHARS: usize = 8_000;
pub const ACTIVE_WINDOW_MAX_CHARS: usize = 2_000;
pub const SELECTED_FILES_MAX_CHARS: usize = 4_000;
pub const RULES_MAX_CHARS: usize = 8_000;
pub const MEMORIES_MAX_CHARS: usize = 8_000;
pub const CONTEXT_BLOCKS_TOTAL_MAX_CHARS: usize = 16_000;

pub const DEFAULT_MAX_STEPS: u32 = 30;
pub const DEFAULT_MAX_TURN_TOKENS: usize = 200_000;

pub const LLM_COMPACT_TIMEOUT_SECS: u64 = 8;
pub const FOLD_PAYLOAD_MSG_MAX_CHARS: usize = 800;
pub const FOLD_PAYLOAD_TOTAL_MAX_CHARS: usize = 24_000;

/// Cap MCP tools registered from a single server.
pub const MCP_MAX_TOOLS_PER_SERVER: usize = 64;
/// Cap total MCP dynamic tools across all servers for one registry refresh.
pub const MCP_MAX_TOTAL_TOOLS: usize = 128;
/// Soft cap on serialized inputSchema / description size per MCP tool.
pub const MCP_MAX_TOOL_SCHEMA_CHARS: usize = 8_000;

pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    (chars / CHARS_PER_TOKEN).max(if chars > 0 { 1 } else { 0 })
}

pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max_chars).collect();
    format!("{truncated}…")
}

/// Keep head and tail so models still see start + end of large tool output.
pub fn truncate_tool_output(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    if max_chars < 64 {
        return truncate_chars(text, max_chars);
    }
    let omitted = count - max_chars;
    let head_budget = max_chars * 2 / 3;
    let tail_budget = max_chars - head_budget;
    let head: String = text.chars().take(head_budget).collect();
    let tail: String = text
        .chars()
        .skip(count.saturating_sub(tail_budget))
        .collect();
    format!("{head}\n…[truncated {omitted} chars]…\n{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_output_keeps_head_and_tail() {
        let text = "A".repeat(100) + &"B".repeat(100);
        let out = truncate_tool_output(&text, 80);
        assert!(out.contains("truncated"));
        assert!(out.starts_with('A'));
        assert!(out.ends_with('B'));
        assert!(out.chars().count() < text.chars().count());
    }
}
