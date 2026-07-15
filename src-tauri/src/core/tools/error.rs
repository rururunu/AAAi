use std::fmt;

#[derive(Debug, Clone)]
pub struct ToolError {
    pub message: String,
    terminal: bool,
}

impl ToolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            terminal: false,
        }
    }

    pub fn user_denied(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            terminal: true,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolError {}

impl From<std::io::Error> for ToolError {
    fn from(value: std::io::Error) -> Self {
        Self::new(value.to_string())
    }
}

impl From<serde_json::Error> for ToolError {
    fn from(value: serde_json::Error) -> Self {
        Self::new(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_denial_is_a_terminal_tool_error() {
        assert!(ToolError::user_denied("denied").is_terminal());
        assert!(!ToolError::new("ordinary failure").is_terminal());
    }
}
