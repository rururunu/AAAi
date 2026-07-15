use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ColorScheme {
    #[serde(rename = "blue-black")]
    BlueBlack,
    Dark,
    Light,
    Midnight,
    Forest,
    Rose,
    Ocean,
    Cream,
    Graphite,
    Ember,
    Frost,
    Teal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AppLanguage {
    #[default]
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "ja-JP")]
    JaJp,
    #[serde(rename = "ru-RU")]
    RuRu,
    #[serde(rename = "de-DE")]
    DeDe,
    #[serde(rename = "fr-FR")]
    FrFr,
    #[serde(rename = "ko-KR")]
    KoKr,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Disabled,
    #[default]
    High,
    Max,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningLanguage {
    #[default]
    Auto,
    Zh,
    En,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum WebSearchProvider {
    #[default]
    Serper,
    Tavily,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ToolApprovalMode {
    #[default]
    Ask,
    Auto,
    AlwaysAllow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LspServerConfig {
    pub id: String,
    pub languages: Vec<String>,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<(String, String)>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub color_scheme: ColorScheme,
    #[serde(default = "default_custom_accent_color")]
    pub custom_accent_color: String,
    pub language: AppLanguage,
    #[serde(default)]
    pub deepseek_api_key: String,
    #[serde(default = "default_memory_enabled")]
    pub memory_enabled: bool,
    #[serde(default)]
    pub mem0_api_key: String,
    #[serde(default = "default_mem0_user_id")]
    pub mem0_user_id: String,
    #[serde(default = "default_mem0_base_url")]
    pub mem0_base_url: String,
    #[serde(default)]
    pub web_search_enabled: bool,
    #[serde(default)]
    pub web_search_provider: WebSearchProvider,
    #[serde(default)]
    pub serper_api_key: String,
    #[serde(default)]
    pub tavily_api_key: String,
    #[serde(default)]
    pub tool_approval_mode: ToolApprovalMode,
    #[serde(default)]
    pub lsp_enabled: bool,
    #[serde(default)]
    pub lsp_servers: Vec<LspServerConfig>,
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
    #[serde(default = "default_opacity")]
    pub opacity: u32,
    #[serde(default = "default_chat_model")]
    pub chat_model: String,
    #[serde(default)]
    pub reasoning_effort: ReasoningEffort,
    #[serde(default)]
    pub reasoning_language: ReasoningLanguage,
    /// When true, assistant turns that contain `tool_calls` include `reasoning_content`
    /// in subsequent API history (required by DeepSeek thinking + tools).
    #[serde(default = "default_true")]
    pub pass_tool_reasoning: bool,
    #[serde(default = "default_zoom")]
    pub zoom: u32,
    /// Secondary overlay shortcut, e.g. `Ctrl+Alt+Space` (recorded in Settings).
    #[serde(default = "default_secondary_hotkey")]
    pub secondary_hotkey: String,
}

fn default_chat_model() -> String {
    "deepseek-chat".to_string()
}

fn default_custom_accent_color() -> String {
    String::new()
}

fn default_zoom() -> u32 {
    100
}

fn default_opacity() -> u32 {
    100
}

fn default_memory_enabled() -> bool {
    true
}
fn default_mem0_user_id() -> String {
    "peek-user".to_string()
}
fn default_mem0_base_url() -> String {
    "https://api.mem0.ai/v1".to_string()
}

fn default_secondary_hotkey() -> String {
    crate::services::hotkey::DEFAULT_SECONDARY_HOTKEY.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsPatch {
    pub color_scheme: Option<ColorScheme>,
    pub custom_accent_color: Option<String>,
    pub language: Option<AppLanguage>,
    pub deepseek_api_key: Option<String>,
    pub memory_enabled: Option<bool>,
    pub mem0_api_key: Option<String>,
    pub mem0_user_id: Option<String>,
    pub mem0_base_url: Option<String>,
    pub web_search_enabled: Option<bool>,
    pub web_search_provider: Option<WebSearchProvider>,
    pub serper_api_key: Option<String>,
    pub tavily_api_key: Option<String>,
    pub tool_approval_mode: Option<ToolApprovalMode>,
    pub lsp_enabled: Option<bool>,
    pub lsp_servers: Option<Vec<LspServerConfig>>,
    pub mcp_servers: Option<Vec<McpServerConfig>>,
    pub opacity: Option<u32>,
    pub chat_model: Option<String>,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub reasoning_language: Option<ReasoningLanguage>,
    pub pass_tool_reasoning: Option<bool>,
    pub zoom: Option<u32>,
    pub secondary_hotkey: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::BlueBlack,
            custom_accent_color: default_custom_accent_color(),
            language: AppLanguage::ZhCn,
            deepseek_api_key: String::new(),
            memory_enabled: default_memory_enabled(),
            mem0_api_key: String::new(),
            mem0_user_id: default_mem0_user_id(),
            mem0_base_url: default_mem0_base_url(),
            web_search_enabled: false,
            web_search_provider: WebSearchProvider::default(),
            serper_api_key: String::new(),
            tavily_api_key: String::new(),
            tool_approval_mode: ToolApprovalMode::default(),
            lsp_enabled: false,
            lsp_servers: default_lsp_servers(),
            mcp_servers: Vec::new(),
            opacity: 100,
            chat_model: default_chat_model(),
            reasoning_effort: ReasoningEffort::default(),
            reasoning_language: ReasoningLanguage::default(),
            pass_tool_reasoning: true,
            zoom: 100,
            secondary_hotkey: default_secondary_hotkey(),
        }
    }
}

fn default_lsp_servers() -> Vec<LspServerConfig> {
    vec![
        LspServerConfig {
            id: "rust".into(),
            languages: vec!["rust".into()],
            command: "rust-analyzer".into(),
            args: Vec::new(),
            enabled: true,
        },
        LspServerConfig {
            id: "typescript".into(),
            languages: vec!["typescript".into(), "javascript".into(), "tsx".into(), "jsx".into()],
            command: "typescript-language-server".into(),
            args: vec!["--stdio".into()],
            enabled: true,
        },
    ]
}

impl AppSettings {
    pub fn merge(&self, patch: AppSettingsPatch) -> Self {
        Self {
            color_scheme: patch.color_scheme.unwrap_or(self.color_scheme),
            custom_accent_color: patch
                .custom_accent_color
                .unwrap_or_else(|| self.custom_accent_color.clone()),
            language: patch.language.unwrap_or(self.language),
            deepseek_api_key: patch
                .deepseek_api_key
                .unwrap_or_else(|| self.deepseek_api_key.clone()),
            memory_enabled: patch.memory_enabled.unwrap_or(self.memory_enabled),
            mem0_api_key: patch
                .mem0_api_key
                .unwrap_or_else(|| self.mem0_api_key.clone()),
            mem0_user_id: patch
                .mem0_user_id
                .unwrap_or_else(|| self.mem0_user_id.clone()),
            mem0_base_url: patch
                .mem0_base_url
                .unwrap_or_else(|| self.mem0_base_url.clone()),
            web_search_enabled: patch
                .web_search_enabled
                .unwrap_or(self.web_search_enabled),
            web_search_provider: patch
                .web_search_provider
                .unwrap_or(self.web_search_provider),
            serper_api_key: patch
                .serper_api_key
                .unwrap_or_else(|| self.serper_api_key.clone()),
            tavily_api_key: patch
                .tavily_api_key
                .unwrap_or_else(|| self.tavily_api_key.clone()),
            tool_approval_mode: patch
                .tool_approval_mode
                .unwrap_or(self.tool_approval_mode),
            lsp_enabled: patch.lsp_enabled.unwrap_or(self.lsp_enabled),
            lsp_servers: patch
                .lsp_servers
                .unwrap_or_else(|| self.lsp_servers.clone()),
            mcp_servers: patch
                .mcp_servers
                .unwrap_or_else(|| self.mcp_servers.clone()),
            opacity: patch.opacity.unwrap_or(self.opacity),
            chat_model: patch.chat_model.unwrap_or_else(|| self.chat_model.clone()),
            reasoning_effort: patch.reasoning_effort.unwrap_or(self.reasoning_effort),
            reasoning_language: patch.reasoning_language.unwrap_or(self.reasoning_language),
            pass_tool_reasoning: patch
                .pass_tool_reasoning
                .unwrap_or(self.pass_tool_reasoning),
            zoom: patch.zoom.unwrap_or(self.zoom),
            secondary_hotkey: patch
                .secondary_hotkey
                .map(|value| crate::services::hotkey::normalize_hotkey(&value))
                .unwrap_or_else(|| self.secondary_hotkey.clone()),
        }
    }
}
