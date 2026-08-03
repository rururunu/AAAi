use crate::models::settings::{AppLanguage, AppSettings, ReasoningLanguage, ResponseTone};

#[derive(Debug, Clone, Default)]
pub struct SendPreferences {
    pub reasoning_language: ReasoningLanguage,
    pub app_language: AppLanguage,
    pub response_tone: ResponseTone,
}

impl From<&AppSettings> for SendPreferences {
    fn from(settings: &AppSettings) -> Self {
        Self {
            reasoning_language: settings.reasoning_language,
            app_language: settings.language,
            response_tone: settings.response_tone,
        }
    }
}
