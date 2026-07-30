mod chat_context;
mod error;
mod ide_context;
mod window_info;

pub use chat_context::{CaptureSource, ChatContext};
pub use error::CaptureError;
pub use ide_context::{CursorPosition, IDEContext};
pub use window_info::WindowInfo;
