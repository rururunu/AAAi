use crate::core::mcp::{runtime_support, McpRuntimeSupport};

#[tauri::command]
pub fn get_mcp_runtime_support() -> McpRuntimeSupport {
    runtime_support()
}
