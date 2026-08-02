//! Minimal MCP stdio JSON-RPC client (tools/list + tools/call).

use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{json, Value};

use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::registry::ToolRegistry;
use crate::models::settings::{AppSettings, McpServerConfig};
use crate::runtime::terminal::prepare_command;

/// Extra dirs GUI apps often miss (nvm, hermes, Volta, Scoop, system Node).
fn known_node_bin_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(pf) = env::var("ProgramFiles") {
        dirs.push(PathBuf::from(pf).join("nodejs"));
    }
    if let Ok(pf86) = env::var("ProgramFiles(x86)") {
        dirs.push(PathBuf::from(pf86).join("nodejs"));
    }
    if let Ok(local) = env::var("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        dirs.push(local.join("Programs").join("nodejs"));
        dirs.push(local.join("hermes").join("node"));
        dirs.push(local.join("fnm"));
        dirs.push(local.join("Volta").join("bin"));
    }
    if let Ok(appdata) = env::var("APPDATA") {
        let appdata = PathBuf::from(appdata);
        dirs.push(appdata.join("npm"));
        dirs.push(appdata.join("nvm"));
    }
    if let Ok(userprofile) = env::var("USERPROFILE") {
        let home = PathBuf::from(userprofile);
        dirs.push(home.join(".volta").join("bin"));
        dirs.push(home.join("scoop").join("shims"));
        dirs.push(home.join("AppData").join("Roaming").join("npm"));
        dirs.push(home.join(".local").join("bin"));
    }
    // Common nvm-windows symlink / install roots
    dirs.push(PathBuf::from(r"C:\nvm4w\nodejs"));
    dirs.push(PathBuf::from(r"C:\Program Files\nodejs"));
    dirs
}

fn current_path_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for key in ["PATH", "Path"] {
        if let Ok(value) = env::var(key) {
            out.extend(env::split_paths(&value));
        }
    }
    out
}

fn enriched_path_value() -> Option<std::ffi::OsString> {
    let mut dirs = current_path_dirs();
    for dir in known_node_bin_dirs() {
        if dir.is_dir() && !dirs.iter().any(|d| d == &dir) {
            dirs.push(dir);
        }
    }
    env::join_paths(dirs).ok()
}

fn file_exists(path: &Path) -> bool {
    path.is_file()
}

#[cfg(windows)]
fn windows_shim_candidates(name: &str) -> Vec<String> {
    // Prefer `.cmd`/`.exe` — nvm's bare `npx`/`npm` files are shell scripts and
    // CreateProcess returns os error 193 (%1 is not a valid Win32 application).
    if name.contains('.') {
        vec![name.to_string()]
    } else {
        vec![
            format!("{name}.cmd"),
            format!("{name}.exe"),
            format!("{name}.bat"),
            format!("{name}.com"),
        ]
    }
}

fn look_for_command(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    #[cfg(windows)]
    let candidates = windows_shim_candidates(name);
    #[cfg(not(windows))]
    let candidates = vec![name.to_string()];
    for dir in dirs {
        for candidate in &candidates {
            let path = dir.join(candidate);
            if file_exists(&path) {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(windows)]
fn prefer_win32_executable(path: PathBuf) -> PathBuf {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(ext.as_str(), "exe" | "cmd" | "bat" | "com") {
        return path;
    }
    for ext in ["cmd", "exe", "bat"] {
        let sibling = path.with_extension(ext);
        if file_exists(&sibling) {
            return sibling;
        }
    }
    path
}

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = current_path_dirs();
    dirs.extend(known_node_bin_dirs());
    dirs
}

pub fn find_node_exe() -> Option<PathBuf> {
    look_for_command("node", &search_dirs())
}

/// Resolve npm's JS entry (e.g. `npx-cli.js`) next to `node.exe`.
pub fn find_npm_js_cli(cli_file: &str) -> Option<PathBuf> {
    let node = find_node_exe()?;
    let cli = node
        .parent()?
        .join("node_modules")
        .join("npm")
        .join("bin")
        .join(cli_file);
    file_exists(&cli).then_some(cli)
}

pub fn find_uvx_exe() -> Option<PathBuf> {
    look_for_command("uvx", &search_dirs())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRuntimeSupport {
    /// Can launch npm-based stdio servers (`npx` / `node npx-cli.js`).
    pub npm: bool,
    /// Can launch PyPI-based stdio servers (`uvx`).
    pub pypi: bool,
    pub node_path: Option<String>,
    pub npx_cli_path: Option<String>,
    pub uvx_path: Option<String>,
}

pub fn runtime_support() -> McpRuntimeSupport {
    let node = find_node_exe();
    let npx_cli = find_npm_js_cli("npx-cli.js");
    let uvx = find_uvx_exe();
    McpRuntimeSupport {
        npm: node.is_some() && npx_cli.is_some(),
        pypi: uvx.is_some(),
        node_path: node.map(|p| p.to_string_lossy().into_owned()),
        npx_cli_path: npx_cli.map(|p| p.to_string_lossy().into_owned()),
        uvx_path: uvx.map(|p| p.to_string_lossy().into_owned()),
    }
}

fn resolve_mcp_program(command: &str) -> PathBuf {
    let as_path = PathBuf::from(command);
    if as_path.components().count() > 1 || file_exists(&as_path) {
        #[cfg(windows)]
        {
            return prefer_win32_executable(as_path);
        }
        #[cfg(not(windows))]
        {
            return as_path;
        }
    }

    if let Some(found) = look_for_command(command, &search_dirs()) {
        return found;
    }

    as_path
}

fn apply_mcp_env(cmd: &mut Command, config: &McpServerConfig) {
    if let Some(path) = enriched_path_value() {
        cmd.env("PATH", &path);
        #[cfg(windows)]
        cmd.env("Path", &path);
    }
    for (k, v) in &config.env {
        cmd.env(k, v);
    }
}

fn quote_cmd_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }
    if arg.chars().any(|c| c.is_whitespace() || c == '"') {
        format!("\"{}\"", arg.replace('"', "\"\""))
    } else {
        arg.to_string()
    }
}

/// Build a spawnable command. On Windows, prefer `node.exe npx-cli.js` so we never
/// CreateProcess the extensionless nvm `npx` shim (os error 193).
fn build_mcp_command(config: &McpServerConfig) -> Result<(Command, String), ToolError> {
    let command = config.command.trim();
    let lower = command.to_ascii_lowercase();

    #[cfg(windows)]
    {
        if lower == "npx" {
            if let (Some(node), Some(cli)) = (find_node_exe(), find_npm_js_cli("npx-cli.js")) {
                let summary = format!("{} {}", node.display(), cli.display());
                let mut cmd = Command::new(&node);
                cmd.arg(&cli);
                cmd.args(&config.args);
                apply_mcp_env(&mut cmd, config);
                return Ok((cmd, summary));
            }
        }
        if lower == "npm" {
            if let (Some(node), Some(cli)) = (find_node_exe(), find_npm_js_cli("npm-cli.js")) {
                let summary = format!("{} {}", node.display(), cli.display());
                let mut cmd = Command::new(&node);
                cmd.arg(&cli);
                cmd.args(&config.args);
                apply_mcp_env(&mut cmd, config);
                return Ok((cmd, summary));
            }
        }
    }

    #[cfg(not(windows))]
    {
        if lower == "npx" {
            if let (Some(node), Some(cli)) = (find_node_exe(), find_npm_js_cli("npx-cli.js")) {
                let summary = format!("{} {}", node.display(), cli.display());
                let mut cmd = Command::new(&node);
                cmd.arg(&cli);
                cmd.args(&config.args);
                apply_mcp_env(&mut cmd, config);
                return Ok((cmd, summary));
            }
        }
    }

    let program = resolve_mcp_program(command);

    #[cfg(windows)]
    {
        let ext = program
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let need_cmd = matches!(ext.as_str(), "cmd" | "bat")
            || matches!(
                lower.as_str(),
                "npx" | "npm" | "uvx" | "pnpm" | "yarn" | "bun" | "deno"
            );
        if need_cmd {
            // Never pass an extensionless shim path into CreateProcess.
            let launch = if matches!(ext.as_str(), "cmd" | "bat" | "exe" | "com") {
                program.clone()
            } else if let Some(found) = look_for_command(command, &search_dirs()) {
                found
            } else {
                PathBuf::from(format!("{command}.cmd"))
            };
            let mut parts = vec![quote_cmd_arg(&launch.to_string_lossy())];
            parts.extend(config.args.iter().map(|a| quote_cmd_arg(a)));
            let line = parts.join(" ");
            let summary = format!("cmd.exe /C {line}");
            let mut cmd = Command::new("cmd.exe");
            cmd.args(["/D", "/S", "/C", &line]);
            apply_mcp_env(&mut cmd, config);
            return Ok((cmd, summary));
        }
    }

    if !file_exists(&program) && program.components().count() == 1 {
        return Err(ToolError::new(format!(
            "cannot find MCP program `{command}` on PATH (looked in Node/nvm dirs). Install Node.js or set a full path in MCP settings."
        )));
    }

    let summary = program.display().to_string();
    let mut cmd = Command::new(&program);
    cmd.args(&config.args);
    apply_mcp_env(&mut cmd, config);
    Ok((cmd, summary))
}

struct McpProcess {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl McpProcess {
    fn spawn(config: &McpServerConfig) -> Result<Self, ToolError> {
        let (mut cmd, resolved) = build_mcp_command(config)?;
        eprintln!(
            "MCP `{}` launching via: {resolved} {:?}",
            config.command, config.args
        );
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        prepare_command(&mut cmd);
        let mut child = cmd.spawn().map_err(|e| {
            ToolError::new(format!(
                "failed to start MCP `{}` via `{resolved}`: {e}",
                config.command
            ))
        })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ToolError::new("MCP stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ToolError::new("MCP stdout unavailable"))?;
        let mut proc = Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
            next_id: 1,
        };
        let _ = proc.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "AAAi", "version": "0.1.2" }
            }),
        )?;
        proc.notify("notifications/initialized", json!({}))?;
        Ok(proc)
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, ToolError> {
        let id = self.next_id;
        self.next_id += 1;
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        self.write_message(&msg)?;
        loop {
            let response = self.read_message()?;
            if response.get("id").and_then(|v| v.as_u64()) == Some(id)
                || response.get("id").and_then(|v| v.as_i64()) == Some(id as i64)
            {
                if let Some(error) = response.get("error") {
                    return Err(ToolError::new(format!("MCP error: {error}")));
                }
                return Ok(response.get("result").cloned().unwrap_or(Value::Null));
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), ToolError> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
    }

    fn write_message(&mut self, msg: &Value) -> Result<(), ToolError> {
        let body = serde_json::to_vec(msg)?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())
            .map_err(|e| ToolError::new(e.to_string()))?;
        self.stdin
            .write_all(&body)
            .map_err(|e| ToolError::new(e.to_string()))?;
        self.stdin
            .flush()
            .map_err(|e| ToolError::new(e.to_string()))
    }

    fn read_message(&mut self) -> Result<Value, ToolError> {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            self.reader
                .read_line(&mut line)
                .map_err(|e| ToolError::new(e.to_string()))?;
            if line == "\r\n" || line == "\n" || line.is_empty() {
                break;
            }
            let lower = line.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("content-length:") {
                content_length = Some(
                    rest.trim()
                        .parse::<usize>()
                        .map_err(|e| ToolError::new(e.to_string()))?,
                );
            }
        }
        let len = content_length.ok_or_else(|| ToolError::new("MCP missing Content-Length"))?;
        let mut buf = vec![0u8; len];
        self.reader
            .read_exact(&mut buf)
            .map_err(|e| ToolError::new(e.to_string()))?;
        serde_json::from_slice(&buf).map_err(|e| ToolError::new(e.to_string()))
    }

    fn list_tools(&mut self) -> Result<Vec<Value>, ToolError> {
        let result = self.request("tools/list", json!({}))?;
        Ok(result
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default())
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Result<String, ToolError> {
        let result = self.request(
            "tools/call",
            json!({
                "name": name,
                "arguments": arguments
            }),
        )?;
        Ok(serde_json::to_string_pretty(&result)?)
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

struct NamedMcpTool {
    full_name: String,
    server_id: String,
    local_name: String,
    description: String,
    input_schema: Value,
}

impl Tool for NamedMcpTool {
    fn name(&self) -> &str {
        &self.full_name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters_schema(&self) -> Value {
        self.input_schema.clone()
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        shared_mcp_manager().call(&self.server_id, &self.local_name, args)
    }
}

pub struct McpManager {
    servers: Mutex<Vec<McpServerConfig>>,
    processes: Mutex<HashMap<String, McpProcess>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            servers: Mutex::new(Vec::new()),
            processes: Mutex::new(HashMap::new()),
        }
    }

    pub fn configure(&self, settings: &AppSettings) {
        if let Ok(mut s) = self.servers.lock() {
            *s = settings.mcp_servers.clone();
        }
        if let Ok(mut p) = self.processes.lock() {
            p.clear();
        }
    }

    pub fn register_enabled(&self, registry: &ToolRegistry) -> Result<usize, ToolError> {
        // Drop stale dynamic tools / processes before reconnecting.
        registry.unregister_dynamic_prefix("mcp__");
        if let Ok(mut procs) = self.processes.lock() {
            procs.clear();
        }

        let servers = self
            .servers
            .lock()
            .map_err(|_| ToolError::new("mcp lock"))?
            .clone();
        let mut count = 0usize;
        let mut budget = crate::core::chat::limits::MCP_MAX_TOTAL_TOOLS;
        for server in servers.into_iter().filter(|s| s.enabled) {
            if budget == 0 {
                eprintln!(
                    "MCP registration stopped: reached MCP_MAX_TOTAL_TOOLS ({})",
                    crate::core::chat::limits::MCP_MAX_TOTAL_TOOLS
                );
                break;
            }
            match self.connect_server_with_budget(&server, registry, budget) {
                Ok(n) => {
                    count += n;
                    budget = budget.saturating_sub(n);
                }
                Err(error) => {
                    eprintln!("MCP server `{}` failed to connect: {error}", server.id);
                }
            }
        }
        Ok(count)
    }

    pub fn connect_server(
        &self,
        server: &McpServerConfig,
        registry: &ToolRegistry,
    ) -> Result<usize, ToolError> {
        self.connect_server_with_budget(
            server,
            registry,
            crate::core::chat::limits::MCP_MAX_TOTAL_TOOLS,
        )
    }

    fn connect_server_with_budget(
        &self,
        server: &McpServerConfig,
        registry: &ToolRegistry,
        remaining_budget: usize,
    ) -> Result<usize, ToolError> {
        let mut proc = McpProcess::spawn(server)?;
        let tools = proc.list_tools()?;
        {
            let mut procs = self
                .processes
                .lock()
                .map_err(|_| ToolError::new("mcp lock"))?;
            procs.insert(server.id.clone(), proc);
        }
        let per_server_cap =
            crate::core::chat::limits::MCP_MAX_TOOLS_PER_SERVER.min(remaining_budget);
        let mut registered = 0usize;
        let mut skipped = 0usize;
        for tool in tools {
            if registered >= per_server_cap {
                skipped += 1;
                continue;
            }
            let name = tool
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .to_string();
            let description = truncate_mcp_text(
                tool.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("MCP tool"),
                crate::core::chat::limits::MCP_MAX_TOOL_SCHEMA_CHARS / 4,
            );
            let input_schema = truncate_mcp_schema(
                tool.get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
            );
            let full_name = format!("mcp__{}__{name}", server.id);
            registry.register_dynamic(Arc::new(NamedMcpTool {
                full_name,
                server_id: server.id.clone(),
                local_name: name,
                description,
                input_schema,
            }));
            registered += 1;
        }
        if skipped > 0 {
            eprintln!(
                "MCP server `{}`: registered {registered} tools, skipped {skipped} (cap {})",
                server.id, per_server_cap
            );
        }
        Ok(registered)
    }

    pub fn connect_by_id(&self, id: &str, registry: &ToolRegistry) -> Result<usize, ToolError> {
        let servers = self
            .servers
            .lock()
            .map_err(|_| ToolError::new("mcp lock"))?
            .clone();
        let server = servers
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| ToolError::new(format!("unknown MCP server `{id}`")))?;
        self.connect_server(&server, registry)
    }

    pub fn disconnect_by_id(&self, id: &str, registry: &ToolRegistry) {
        if let Ok(mut procs) = self.processes.lock() {
            procs.remove(id);
        }
        registry.unregister_dynamic_prefix(&format!("mcp__{id}__"));
    }

    pub fn reconnect_by_id(&self, id: &str, registry: &ToolRegistry) -> Result<usize, ToolError> {
        self.disconnect_by_id(id, registry);
        self.connect_by_id(id, registry)
    }

    pub fn call(&self, server_id: &str, tool_name: &str, args: Value) -> Result<String, ToolError> {
        let mut procs = self
            .processes
            .lock()
            .map_err(|_| ToolError::new("mcp lock"))?;
        let proc = procs
            .get_mut(server_id)
            .ok_or_else(|| ToolError::new(format!("MCP server `{server_id}` is not connected")))?;
        proc.call_tool(tool_name, args)
    }
}

fn truncate_mcp_text(text: &str, max_chars: usize) -> String {
    crate::core::chat::limits::truncate_chars(text, max_chars)
}

fn truncate_mcp_schema(schema: Value) -> Value {
    let serialized = schema.to_string();
    if serialized.chars().count() <= crate::core::chat::limits::MCP_MAX_TOOL_SCHEMA_CHARS {
        return schema;
    }
    // Fall back to a minimal object schema when the upstream schema is enormous.
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": true,
        "description": format!(
            "Original inputSchema truncated ({} chars > {}).",
            serialized.chars().count(),
            crate::core::chat::limits::MCP_MAX_TOOL_SCHEMA_CHARS
        )
    })
}

pub fn shared_mcp_manager() -> Arc<McpManager> {
    static MANAGER: OnceLock<Arc<McpManager>> = OnceLock::new();
    Arc::clone(MANAGER.get_or_init(|| Arc::new(McpManager::new())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_node_and_npx_cli() {
        let node = find_node_exe().expect("node.exe should be discoverable");
        assert!(file_exists(&node), "{node:?}");
        let cli = find_npm_js_cli("npx-cli.js").expect("npx-cli.js next to node");
        assert!(file_exists(&cli), "{cli:?}");
    }

    #[test]
    fn builds_npx_through_node_cli() {
        let config = McpServerConfig {
            id: "test".into(),
            title: None,
            description: None,
            command: "npx".into(),
            args: vec!["--version".into()],
            env: vec![],
            enabled: true,
        };
        let (mut cmd, summary) = build_mcp_command(&config).expect("build npx command");
        assert!(
            summary.to_ascii_lowercase().contains("npx-cli.js")
                || summary.to_ascii_lowercase().contains("npx.cmd"),
            "unexpected launcher: {summary}"
        );
        let output = cmd.output().expect("spawn npx --version");
        assert!(
            output.status.success(),
            "npx --version failed: status={:?} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
