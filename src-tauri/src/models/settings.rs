use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ColorScheme {
    #[serde(
        rename = "dark",
        alias = "system",
        alias = "auto",
        alias = "default",
        alias = "nocturne",
        alias = "blue-black",
        alias = "dark",
        alias = "midnight",
        alias = "forest",
        alias = "rose",
        alias = "ocean",
        alias = "graphite",
        alias = "ember",
        alias = "teal",
        alias = "ghost-pastel"
    )]
    Dark,
    #[serde(rename = "light", alias = "paper", alias = "cream", alias = "frost")]
    Light,
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

/// Chat interaction mode: Agent can mutate; Ask is read-only tools only.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ChatMode {
    #[default]
    Agent,
    Ask,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    /// Comma-separated or newline-separated model IDs (stored as raw text).
    pub models: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GeminiOAuthSettings {
    #[serde(default = "default_gemini_oauth_client_id")]
    pub client_id: String,
    #[serde(default = "default_gemini_oauth_client_secret")]
    pub client_secret: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    /// Unix timestamp (seconds) when `access_token` expires.
    #[serde(default)]
    pub expires_at: i64,
    #[serde(default)]
    pub email: String,
    /// Cloud Code companion project from `loadCodeAssist`.
    #[serde(default)]
    pub project_id: String,
}

impl Default for GeminiOAuthSettings {
    fn default() -> Self {
        Self {
            client_id: default_gemini_oauth_client_id(),
            client_secret: default_gemini_oauth_client_secret(),
            access_token: String::new(),
            refresh_token: String::new(),
            expires_at: 0,
            email: String::new(),
            project_id: String::new(),
        }
    }
}

impl GeminiOAuthSettings {
    pub fn is_logged_in(&self) -> bool {
        !self.access_token.trim().is_empty() || !self.refresh_token.trim().is_empty()
    }
}

fn default_gemini_oauth_client_id() -> String {
    String::new()
}

fn default_gemini_oauth_client_secret() -> String {
    String::new()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub color_scheme: ColorScheme,
    pub language: AppLanguage,
    #[serde(default)]
    pub deepseek_api_key: String,
    #[serde(default)]
    pub gemini_oauth: GeminiOAuthSettings,
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
    pub chat_mode: ChatMode,
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
    pub chat_model_provider: String,
    #[serde(default = "default_multimodal_model")]
    pub multimodal_model: String,
    #[serde(default)]
    pub multimodal_model_provider: String,
    #[serde(default)]
    pub reasoning_effort: ReasoningEffort,
    #[serde(default)]
    pub reasoning_language: ReasoningLanguage,
    /// When true, assistant turns that contain `tool_calls` include `reasoning_content`
    /// in subsequent API history (required by DeepSeek thinking + tools).
    #[serde(default = "default_true")]
    pub pass_tool_reasoning: bool,
    /// Controls whether reasoning supplied by the model is rendered in chat.
    #[serde(default = "default_true")]
    pub show_reasoning: bool,
    /// Allow the main agent to delegate work to user-selected models.
    #[serde(default)]
    pub multi_model_collaboration: bool,
    #[serde(default)]
    pub collaboration_models: Vec<String>,
    #[serde(default = "default_true")]
    pub multimodal_split_analysis: bool,
    /// When true, use a 1M-token context window for compaction / turn budgets.
    #[serde(default = "default_true")]
    pub large_context_enabled: bool,
    #[serde(default = "default_zoom")]
    pub zoom: u32,
    /// Primary overlay shortcut modifier, activated by a double tap.
    #[serde(default = "default_primary_hotkey")]
    pub primary_hotkey: String,
    /// Secondary overlay shortcut, e.g. `Ctrl+Alt+Space` (recorded in Settings).
    #[serde(default = "default_secondary_hotkey")]
    pub secondary_hotkey: String,
    #[serde(default)]
    pub custom_providers: Vec<CustomProviderConfig>,
    /// Show an AI button on PixPin pin windows (bottom-right).
    #[serde(default = "default_true")]
    pub pixpin_pin_ai_enabled: bool,
    /// Show an AI button on Snipaste pin windows (bottom-right).
    #[serde(default = "default_true")]
    pub snipaste_pin_ai_enabled: bool,
}

fn default_chat_model() -> String {
    String::new()
}

fn default_multimodal_model() -> String {
    "gpt-4o".to_string()
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

fn default_primary_hotkey() -> String {
    crate::services::hotkey::DEFAULT_PRIMARY_HOTKEY.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsPatch {
    pub color_scheme: Option<ColorScheme>,
    pub language: Option<AppLanguage>,
    pub deepseek_api_key: Option<String>,
    pub gemini_oauth: Option<GeminiOAuthSettings>,
    pub memory_enabled: Option<bool>,
    pub mem0_api_key: Option<String>,
    pub mem0_user_id: Option<String>,
    pub mem0_base_url: Option<String>,
    pub web_search_enabled: Option<bool>,
    pub web_search_provider: Option<WebSearchProvider>,
    pub serper_api_key: Option<String>,
    pub tavily_api_key: Option<String>,
    pub tool_approval_mode: Option<ToolApprovalMode>,
    pub chat_mode: Option<ChatMode>,
    pub lsp_enabled: Option<bool>,
    pub lsp_servers: Option<Vec<LspServerConfig>>,
    pub mcp_servers: Option<Vec<McpServerConfig>>,
    pub opacity: Option<u32>,
    pub chat_model: Option<String>,
    pub chat_model_provider: Option<String>,
    pub multimodal_model: Option<String>,
    pub multimodal_model_provider: Option<String>,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub reasoning_language: Option<ReasoningLanguage>,
    pub pass_tool_reasoning: Option<bool>,
    pub show_reasoning: Option<bool>,
    pub multi_model_collaboration: Option<bool>,
    pub collaboration_models: Option<Vec<String>>,
    pub multimodal_split_analysis: Option<bool>,
    pub large_context_enabled: Option<bool>,
    pub zoom: Option<u32>,
    pub primary_hotkey: Option<String>,
    pub secondary_hotkey: Option<String>,
    pub custom_providers: Option<Vec<CustomProviderConfig>>,
    pub pixpin_pin_ai_enabled: Option<bool>,
    pub snipaste_pin_ai_enabled: Option<bool>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::Dark,
            language: AppLanguage::ZhCn,
            deepseek_api_key: String::new(),
            gemini_oauth: GeminiOAuthSettings::default(),
            memory_enabled: default_memory_enabled(),
            mem0_api_key: String::new(),
            mem0_user_id: default_mem0_user_id(),
            mem0_base_url: default_mem0_base_url(),
            web_search_enabled: false,
            web_search_provider: WebSearchProvider::default(),
            serper_api_key: String::new(),
            tavily_api_key: String::new(),
            tool_approval_mode: ToolApprovalMode::default(),
            chat_mode: ChatMode::default(),
            lsp_enabled: false,
            lsp_servers: default_lsp_servers(),
            mcp_servers: Vec::new(),
            opacity: 100,
            chat_model: default_chat_model(),
            chat_model_provider: String::new(),
            multimodal_model: default_multimodal_model(),
            multimodal_model_provider: String::new(),
            reasoning_effort: ReasoningEffort::default(),
            reasoning_language: ReasoningLanguage::default(),
            pass_tool_reasoning: true,
            show_reasoning: true,
            multi_model_collaboration: false,
            collaboration_models: Vec::new(),
            multimodal_split_analysis: true,
            large_context_enabled: true,
            zoom: 100,
            primary_hotkey: default_primary_hotkey(),
            secondary_hotkey: default_secondary_hotkey(),
            custom_providers: Vec::new(),
            pixpin_pin_ai_enabled: true,
            snipaste_pin_ai_enabled: true,
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
            languages: vec![
                "typescript".into(),
                "javascript".into(),
                "tsx".into(),
                "jsx".into(),
            ],
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
            language: patch.language.unwrap_or(self.language),
            deepseek_api_key: patch
                .deepseek_api_key
                .unwrap_or_else(|| self.deepseek_api_key.clone()),
            gemini_oauth: patch
                .gemini_oauth
                .unwrap_or_else(|| self.gemini_oauth.clone()),
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
            web_search_enabled: patch.web_search_enabled.unwrap_or(self.web_search_enabled),
            web_search_provider: patch
                .web_search_provider
                .unwrap_or(self.web_search_provider),
            serper_api_key: patch
                .serper_api_key
                .unwrap_or_else(|| self.serper_api_key.clone()),
            tavily_api_key: patch
                .tavily_api_key
                .unwrap_or_else(|| self.tavily_api_key.clone()),
            tool_approval_mode: patch.tool_approval_mode.unwrap_or(self.tool_approval_mode),
            chat_mode: patch.chat_mode.unwrap_or(self.chat_mode),
            lsp_enabled: patch.lsp_enabled.unwrap_or(self.lsp_enabled),
            lsp_servers: patch
                .lsp_servers
                .unwrap_or_else(|| self.lsp_servers.clone()),
            mcp_servers: patch
                .mcp_servers
                .unwrap_or_else(|| self.mcp_servers.clone()),
            opacity: patch.opacity.unwrap_or(self.opacity),
            chat_model: patch.chat_model.unwrap_or_else(|| self.chat_model.clone()),
            chat_model_provider: patch
                .chat_model_provider
                .unwrap_or_else(|| self.chat_model_provider.clone()),
            multimodal_model: patch
                .multimodal_model
                .unwrap_or_else(|| self.multimodal_model.clone()),
            multimodal_model_provider: patch
                .multimodal_model_provider
                .unwrap_or_else(|| self.multimodal_model_provider.clone()),
            reasoning_effort: patch.reasoning_effort.unwrap_or(self.reasoning_effort),
            reasoning_language: patch.reasoning_language.unwrap_or(self.reasoning_language),
            pass_tool_reasoning: patch
                .pass_tool_reasoning
                .unwrap_or(self.pass_tool_reasoning),
            show_reasoning: patch.show_reasoning.unwrap_or(self.show_reasoning),
            multi_model_collaboration: patch
                .multi_model_collaboration
                .unwrap_or(self.multi_model_collaboration),
            collaboration_models: patch
                .collaboration_models
                .unwrap_or_else(|| self.collaboration_models.clone()),
            multimodal_split_analysis: patch
                .multimodal_split_analysis
                .unwrap_or(self.multimodal_split_analysis),
            large_context_enabled: patch
                .large_context_enabled
                .unwrap_or(self.large_context_enabled),
            zoom: patch.zoom.unwrap_or(self.zoom),
            primary_hotkey: patch
                .primary_hotkey
                .map(|value| crate::services::hotkey::normalize_primary_hotkey(&value))
                .unwrap_or_else(|| self.primary_hotkey.clone()),
            secondary_hotkey: patch
                .secondary_hotkey
                .map(|value| crate::services::hotkey::normalize_hotkey(&value))
                .unwrap_or_else(|| self.secondary_hotkey.clone()),
            custom_providers: patch
                .custom_providers
                .unwrap_or_else(|| self.custom_providers.clone()),
            pixpin_pin_ai_enabled: patch
                .pixpin_pin_ai_enabled
                .unwrap_or(self.pixpin_pin_ai_enabled),
            snipaste_pin_ai_enabled: patch
                .snipaste_pin_ai_enabled
                .unwrap_or(self.snipaste_pin_ai_enabled),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, AppSettingsPatch, ColorScheme};

    #[test]
    fn legacy_settings_default_to_showing_reasoning() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "colorScheme": "blue-black",
            "language": "zh-CN"
        }))
        .expect("legacy settings should deserialize");

        assert_eq!(settings.color_scheme, ColorScheme::Dark);
        assert!(settings.show_reasoning);
        assert!(!settings.multi_model_collaboration);
        assert!(settings.collaboration_models.is_empty());
        assert!(settings.chat_model_provider.is_empty());
        assert!(settings.multimodal_model_provider.is_empty());
    }

    #[test]
    fn legacy_light_palette_migrates_without_custom_accent() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "colorScheme": "frost",
            "customAccentColor": "#ff00ff",
            "language": "zh-CN"
        }))
        .expect("legacy palette should deserialize");

        assert_eq!(settings.color_scheme, ColorScheme::Light);
        let serialized = serde_json::to_value(settings).expect("settings should serialize");
        assert_eq!(serialized["colorScheme"], "light");
        assert!(serialized.get("customAccentColor").is_none());
    }

    #[test]
    fn legacy_system_theme_migrates_to_dark() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "colorScheme": "system",
            "language": "zh-CN"
        }))
        .expect("legacy system theme should deserialize");

        assert_eq!(settings.color_scheme, ColorScheme::Dark);
        let serialized = serde_json::to_value(settings).expect("settings should serialize");
        assert_eq!(serialized["colorScheme"], "dark");
    }

    #[test]
    fn show_reasoning_patch_is_optional_and_mergeable() {
        let settings = AppSettings::default();
        assert!(settings.merge(AppSettingsPatch::default()).show_reasoning);

        let patch = AppSettingsPatch {
            show_reasoning: Some(false),
            ..AppSettingsPatch::default()
        };
        assert!(!settings.merge(patch).show_reasoning);
    }

    #[test]
    fn model_provider_patch_is_optional_and_mergeable() {
        let settings = AppSettings::default();
        let patch = AppSettingsPatch {
            chat_model_provider: Some("provider-a".into()),
            multimodal_model_provider: Some("provider-b".into()),
            ..AppSettingsPatch::default()
        };
        let merged = settings.merge(patch);
        assert_eq!(merged.chat_model_provider, "provider-a");
        assert_eq!(merged.multimodal_model_provider, "provider-b");
    }
}
