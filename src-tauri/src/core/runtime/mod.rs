//! Chat protocol types shared by providers, agent loop, and persistence.
//!
//! This is **not** the crate-root `crate::runtime` tool-adapter layer.

pub mod message;
pub mod request;
pub mod stream;

pub use message::{ChatMessage, MessageStatus, Role, ToolActivity, DEFAULT_SESSION_ID};
pub use request::{ChatRequest, RequestContext};
pub use stream::{StreamEvent, ToolCallPayload};
