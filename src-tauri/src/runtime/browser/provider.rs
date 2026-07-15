use crate::runtime::tool::ToolError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserDocument {
    pub url: String,
    pub markdown: String,
    pub truncated: bool,
}

pub trait BrowserProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn read(&self, url: &str) -> Result<BrowserDocument, ToolError>;
}
