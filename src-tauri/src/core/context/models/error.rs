#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum CaptureError {
    WindowDetection(String),
    Clipboard(String),
    Explorer(String),
    ComInit(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WindowDetection(msg) => write!(f, "window detection failed: {msg}"),
            Self::Clipboard(msg) => write!(f, "clipboard capture failed: {msg}"),
            Self::Explorer(msg) => write!(f, "explorer capture failed: {msg}"),
            Self::ComInit(msg) => write!(f, "COM init failed: {msg}"),
        }
    }
}

impl std::error::Error for CaptureError {}
