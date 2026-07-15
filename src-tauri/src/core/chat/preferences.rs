use crate::models::settings::{AppLanguage, AppSettings, ReasoningLanguage};

#[derive(Debug, Clone)]
pub struct SendPreferences {
    pub reasoning_language: ReasoningLanguage,
    pub app_language: AppLanguage,
}

impl Default for SendPreferences {
    fn default() -> Self {
        Self {
            reasoning_language: ReasoningLanguage::default(),
            app_language: AppLanguage::default(),
        }
    }
}

impl From<&AppSettings> for SendPreferences {
    fn from(settings: &AppSettings) -> Self {
        Self {
            reasoning_language: settings.reasoning_language,
            app_language: settings.language,
        }
    }
}
