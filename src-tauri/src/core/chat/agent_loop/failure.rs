use std::collections::HashMap;

use crate::core::chat::limits::MAX_CONSECUTIVE_TOOL_FAILURES;

use super::types::ToolOutcome;

/// Injected once when the same tool+args fails with the same error again,
/// giving the model a chance to change strategy before a hard stop.
pub const IDENTICAL_ERROR_CHALLENGE: &str = concat!(
    "[System] Identical tool failure: the same tool call with the same ",
    "arguments returned the same error again. Do NOT retry that exact call. ",
    "Read the error, then change strategy — different path/arguments, a ",
    "different tool, request permission, or stop and report the blocker ",
    "clearly to the user.",
);

/// How many identical (tool+args+error) failures are allowed before hard stop.
/// 1 = first failure recorded; 2 = challenge; 3 = stop.
const IDENTICAL_ERROR_STOP_AFTER: u32 = 3;

/// Result of folding a tool batch into the failure circuit breaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureAction {
    /// Keep the agent loop going.
    Continue,
    /// Inject a challenge and run another model turn (do not stop yet).
    Challenge { status_kind: String, message: String },
    /// Hard-stop the turn with a user-visible reason.
    Stop { reason: String },
}

/// 失败熔断与同错误防重复：连续失败超过阈值立即停止；同一工具以相同参数
/// 反复返回同一错误时，先挑战一次换策略，再犯才硬停，避免无效循环。
#[derive(Default)]
pub struct FailureBreaker {
    consecutive_tool_failures: u32,
    /// tool|args → (error text, consecutive identical count)
    repeated_tool_errors: HashMap<String, (String, u32)>,
    /// Keys that already received an identical-error challenge this turn.
    challenged_keys: std::collections::HashSet<String>,
}

impl FailureBreaker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold a batch of tool outcomes (from one step) into the failure state.
    pub fn check(&mut self, outcomes: &[ToolOutcome]) -> FailureAction {
        let mut action = FailureAction::Continue;
        for outcome in outcomes {
            if outcome.user_denied {
                continue;
            }
            if !outcome.success {
                self.consecutive_tool_failures += 1;

                let key = format!("{}|{}", outcome.tool_name, outcome.arguments);
                let next_count = match self.repeated_tool_errors.get(&key) {
                    Some((previous, count)) if previous == &outcome.result => count + 1,
                    _ => 1,
                };
                self.repeated_tool_errors
                    .insert(key.clone(), (outcome.result.clone(), next_count));

                // Prefer the more specific identical-error stop over the generic
                // consecutive-failure breaker when both would fire.
                if next_count >= IDENTICAL_ERROR_STOP_AFTER {
                    return FailureAction::Stop {
                        reason: format!(
                            "工具 `{}` 以相同参数反复返回同一错误，已停止重试。请换路径/参数/工具后再试，或发送「继续」。",
                            outcome.tool_name
                        ),
                    };
                }

                if self.consecutive_tool_failures >= MAX_CONSECUTIVE_TOOL_FAILURES {
                    return FailureAction::Stop {
                        reason: format!(
                            "工具连续失败 {} 次，已触发熔断。请换一种做法，或发送「继续」让我接着处理。",
                            MAX_CONSECUTIVE_TOOL_FAILURES
                        ),
                    };
                }

                // Second identical failure → challenge once (first was only recorded).
                if next_count >= 2 && !self.challenged_keys.contains(&key) {
                    self.challenged_keys.insert(key);
                    action = FailureAction::Challenge {
                        status_kind: "identical_error".into(),
                        message: format!(
                            "{IDENTICAL_ERROR_CHALLENGE}\n\nLast error from `{}`:\n{}",
                            outcome.tool_name,
                            truncate_error(&outcome.result, 800)
                        ),
                    };
                }
            } else {
                self.consecutive_tool_failures = 0;
            }
        }
        action
    }
}

fn truncate_error(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let cut: String = trimmed.chars().take(max_chars).collect();
    format!("{cut}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(tool: &str, args: &str, result: &str, success: bool) -> ToolOutcome {
        ToolOutcome {
            call_id: "1".into(),
            tool_name: tool.into(),
            arguments: args.into(),
            result: result.into(),
            success,
            user_denied: false,
        }
    }

    #[test]
    fn identical_error_challenges_before_stop() {
        let mut breaker = FailureBreaker::new();

        assert_eq!(
            breaker.check(&[outcome("write_file", r#"{"path":"a"}"#, "denied", false)]),
            FailureAction::Continue
        );
        match breaker.check(&[outcome("write_file", r#"{"path":"a"}"#, "denied", false)]) {
            FailureAction::Challenge { status_kind, .. } => {
                assert_eq!(status_kind, "identical_error");
            }
            other => panic!("expected challenge, got {other:?}"),
        }
        match breaker.check(&[outcome("write_file", r#"{"path":"a"}"#, "denied", false)]) {
            FailureAction::Stop { reason } => {
                assert!(reason.contains("write_file"), "{reason}");
                assert!(reason.contains("相同参数"), "{reason}");
            }
            other => panic!("expected stop, got {other:?}"),
        }
    }

    #[test]
    fn consecutive_failures_still_stop() {
        let mut breaker = FailureBreaker::new();
        assert!(matches!(
            breaker.check(&[outcome("a", "1", "e1", false)]),
            FailureAction::Continue
        ));
        assert!(matches!(
            breaker.check(&[outcome("b", "2", "e2", false)]),
            FailureAction::Continue
        ));
        match breaker.check(&[outcome("c", "3", "e3", false)]) {
            FailureAction::Stop { reason } => assert!(reason.contains("连续失败")),
            other => panic!("expected stop, got {other:?}"),
        }
    }
}
