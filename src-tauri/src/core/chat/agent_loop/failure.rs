use std::collections::HashMap;

use crate::core::chat::limits::MAX_CONSECUTIVE_TOOL_FAILURES;

use super::types::ToolOutcome;

/// 失败熔断与同错误防重复：连续失败超过阈值，或同一工具以相同参数反复返回同一
/// 错误，立即停止本轮，避免无效循环。
#[derive(Default)]
pub struct FailureBreaker {
    consecutive_tool_failures: u32,
    repeated_tool_errors: HashMap<String, String>,
}

impl FailureBreaker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold a batch of tool outcomes (from one step) into the failure state,
    /// returning a human-readable stop reason once the circuit breaker trips.
    pub fn check(&mut self, outcomes: &[ToolOutcome]) -> Option<String> {
        let mut stop_reason = None;
        for outcome in outcomes {
            if outcome.user_denied {
                continue;
            }
            if !outcome.success {
                self.consecutive_tool_failures += 1;
                if self.consecutive_tool_failures >= MAX_CONSECUTIVE_TOOL_FAILURES {
                    stop_reason = Some(format!(
                        "工具连续失败 {} 次，已触发熔断",
                        MAX_CONSECUTIVE_TOOL_FAILURES
                    ));
                    break;
                }
                let key = format!("{}|{}", outcome.tool_name, outcome.arguments);
                match self.repeated_tool_errors.get(&key) {
                    Some(previous) if previous == &outcome.result => {
                        stop_reason = Some(format!(
                            "工具 `{}` 以相同参数反复返回同一错误，已停止重试",
                            outcome.tool_name
                        ));
                        break;
                    }
                    _ => {
                        self.repeated_tool_errors.insert(key, outcome.result.clone());
                    }
                }
            } else {
                self.consecutive_tool_failures = 0;
            }
        }
        stop_reason
    }
}
