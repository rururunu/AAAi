use crate::models::settings::{AppLanguage, ReasoningLanguage};

use super::PromptPreferences;

pub(super) fn inject_language_blocks(content: &str, preferences: &PromptPreferences) -> String {
    let mut blocks = Vec::new();

    if let Some(block) = reasoning_language_block(preferences, content) {
        blocks.push(block);
    }
    if let Some(block) = response_language_block(preferences, content) {
        blocks.push(block);
    }

    if blocks.is_empty() {
        return content.to_string();
    }

    format!("{}\n\n{}", blocks.join("\n\n"), content)
}

fn reasoning_language_block(preferences: &PromptPreferences, user_content: &str) -> Option<String> {
    let mode = match preferences.reasoning_language {
        ReasoningLanguage::Auto => {
            if infer_chinese_user_text(user_content) {
                "zh"
            } else {
                return None;
            }
        }
        ReasoningLanguage::Zh => "zh",
        ReasoningLanguage::En => "en",
    };

    Some(match mode {
        "zh" => "<reasoning-language>\n可见推理/思考文本偏好：当模型服务暴露可见推理或思考文本时，请使用简体中文。代码、标识符、文件路径、shell 命令和未翻译的技术术语保持原文。此偏好不会覆盖用户对最终回答语言的明确要求。\n</reasoning-language>".to_string(),
        _ => "<reasoning-language>\nVisible reasoning/thinking text preference: use English when the provider exposes reasoning text. Keep code, identifiers, file paths, shell commands, and untranslated technical terms in their original form. This preference does not override an explicit user request for the final answer language.\n</reasoning-language>".to_string(),
    })
}

fn response_language_block(
    preferences: &PromptPreferences,
    user_content: &str,
) -> Option<String> {
    // Auto: follow the user's latest message language first (matches system policy).
    // Only fall back to UI/app language when the message has no clear language signal.
    let mode = match preferences.reasoning_language {
        ReasoningLanguage::Auto => {
            if infer_chinese_user_text(user_content) {
                "zh"
            } else if infer_mostly_latin_user_text(user_content) {
                "en"
            } else {
                match preferences.app_language {
                    AppLanguage::ZhCn => "zh",
                    AppLanguage::EnUs => "en",
                    AppLanguage::JaJp => "ja",
                    AppLanguage::RuRu => "ru",
                    AppLanguage::DeDe => "de",
                    AppLanguage::FrFr => "fr",
                    AppLanguage::KoKr => "ko",
                }
            }
        }
        ReasoningLanguage::Zh => "zh",
        ReasoningLanguage::En => "en",
    };

    Some(match mode {
        "zh" => "<response-language>\n最终回答语言：必须使用简体中文回复用户可见内容（除非用户明确要求其他语言）。代码、标识符、文件路径、shell 命令与未翻译技术术语保持原文。不要因为系统提示或工具输出是英文就改用英文回答。\n</response-language>".to_string(),
        "ja" => response_language_instruction("Japanese"),
        "ru" => response_language_instruction("Russian"),
        "de" => response_language_instruction("German"),
        "fr" => response_language_instruction("French"),
        "ko" => response_language_instruction("Korean"),
        _ => response_language_instruction("English"),
    })
}

fn response_language_instruction(language: &str) -> String {
    format!("<response-language>\nFinal answer language preference: use {language} for user-facing replies unless the user explicitly asks for another language. Keep code, identifiers, file paths, shell commands, and untranslated technical terms in their original form.\n</response-language>")
}

fn infer_chinese_user_text(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(ch,
            '\u{4E00}'..='\u{9FFF}'
                | '\u{3400}'..='\u{4DBF}'
                | '\u{F900}'..='\u{FAFF}'
        )
    })
}

/// True when the message looks like ordinary English / Latin-script text with no CJK.
fn infer_mostly_latin_user_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || infer_chinese_user_text(trimmed) {
        return false;
    }
    let letters: Vec<char> = trimmed.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return false;
    }
    let latin = letters
        .iter()
        .filter(|c| c.is_ascii_alphabetic())
        .count();
    latin * 100 / letters.len() >= 80
}
