#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    WindowDetectionFailed(String),
    ClipboardFailed(String),
    ExplorerFailed(String),
    ComInitFailed(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WindowDetectionFailed(msg) => write!(f, "window detection failed: {msg}"),
            Self::ClipboardFailed(msg) => write!(f, "clipboard capture failed: {msg}"),
            Self::ExplorerFailed(msg) => write!(f, "explorer capture failed: {msg}"),
            Self::ComInitFailed(msg) => write!(f, "COM init failed: {msg}"),
        }
    }
}

impl std::error::Error for CaptureError {}
