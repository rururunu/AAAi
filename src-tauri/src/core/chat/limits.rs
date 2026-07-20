//! Shared text/token limits for prompt assembly and tool loops.

use regex::Regex;

/// Characters → estimated tokens (coarse).
pub const CHARS_PER_TOKEN: usize = 4;

pub const TOOL_OUTPUT_MAX_CHARS: usize = 12_000;
pub const CLIPBOARD_MAX_CHARS: usize = 8_000;
pub const ACTIVE_WINDOW_MAX_CHARS: usize = 2_000;
pub const SELECTED_FILES_MAX_CHARS: usize = 4_000;
pub const RULES_MAX_CHARS: usize = 8_000;
pub const MEMORIES_MAX_CHARS: usize = 8_000;
pub const CONTEXT_BLOCKS_TOTAL_MAX_CHARS: usize = 16_000;

/// Max agent tool-loop iterations per turn. `0` = unlimited.
pub const DEFAULT_MAX_STEPS: u32 = 0;
/// Per-turn token budget when large context is off.
pub const DEFAULT_MAX_TURN_TOKENS: usize = 200_000;
/// Per-turn token budget when large context (1M) is on.
pub const LARGE_MAX_TURN_TOKENS: usize = 1_000_000;

pub fn max_turn_tokens_for(large_context_enabled: bool) -> usize {
    if large_context_enabled {
        LARGE_MAX_TURN_TOKENS
    } else {
        DEFAULT_MAX_TURN_TOKENS
    }
}

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
    if !text.contains("data:image/") {
        let chars = text.chars().count();
        return (chars / CHARS_PER_TOKEN).max(if chars > 0 { 1 } else { 0 });
    }

    // Strip out base64 image contents to avoid huge token estimation
    let re = match Regex::new(r"data:image/[^)]+") {
        Ok(re) => re,
        Err(_) => {
            let chars = text.chars().count();
            return (chars / CHARS_PER_TOKEN).max(if chars > 0 { 1 } else { 0 });
        }
    };

    let cleaned = re.replace_all(text, "image_placeholder");
    let chars = cleaned.chars().count();
    let base_tokens = (chars / CHARS_PER_TOKEN).max(if chars > 0 { 1 } else { 0 });

    let image_count = text.matches("data:image/").count();
    base_tokens + (image_count * 1000)
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
