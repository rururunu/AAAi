use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::core::runtime::{ChatRequest, StreamEvent};

#[derive(Debug, Clone)]
pub enum ProviderError {
    Cancelled,
    Message(String),
}

impl ProviderError {
    pub fn message(value: impl Into<String>) -> Self {
        Self::Message(value.into())
    }

    pub fn cancelled() -> Self {
        Self::Cancelled
    }
}

impl From<String> for ProviderError {
    fn from(value: String) -> Self {
        Self::Message(value.into())
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "provider request cancelled"),
            Self::Message(message) => write!(f, "{message}"),
        }
    }
}

/// AI Provider 抽象 — 仅 `stream()` 接口。
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn id(&self) -> &'static str;

    async fn stream(
        &self,
        request: ChatRequest,
        tx: Sender<StreamEvent>,
    ) -> Result<(), ProviderError>;
}
