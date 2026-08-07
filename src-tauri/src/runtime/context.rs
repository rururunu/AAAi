#![allow(unused_imports)]

pub use crate::core::context::*;

use serde_json::{json, Value};

use crate::runtime::tool::{Tool, ToolContext, ToolError};

pub struct ContextTool;

impl Tool for ContextTool {
    fn name(&self) -> &str {
        "get_context"
    }
    fn description(&self) -> &str {
        "Return fresh selected text, selected files, clipboard, active window, and workspace captured by Anya. Prefer the injected request context when it already answers the question; call this when you need an updated snapshot."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, _args: Value) -> Result<String, ToolError> {
        serde_json::to_string_pretty(&ctx.request_context)
            .map_err(|error| ToolError::new(error.to_string()))
    }
}
