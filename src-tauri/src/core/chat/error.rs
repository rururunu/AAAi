#[derive(Debug, Clone)]
pub enum ChatError {
    EmptyMessage,
    MessageNotFound,
    Provider(String),
    Internal(String),
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyMessage => write!(f, "message cannot be empty"),
            Self::MessageNotFound => write!(f, "message not found"),
            Self::Provider(message) => write!(f, "{message}"),
            Self::Internal(message) => write!(f, "{message}"),
        }
    }
}

impl From<crate::core::ai::ProviderError> for ChatError {
    fn from(error: crate::core::ai::ProviderError) -> Self {
        match error {
            crate::core::ai::ProviderError::Cancelled => {
                Self::Provider("request cancelled".to_string())
            }
            crate::core::ai::ProviderError::Message(message) => Self::Provider(message),
        }
    }
}
