use std::sync::OnceLock;

use tiktoken_rs::{bpe_for_model, cl100k_base, CoreBPE};
use tokenizers::Tokenizer as HuggingFaceTokenizer;

use super::{TokenAccuracy, TokenCount};

pub trait Tokenizer: Send + Sync {
    fn name(&self) -> &'static str;
    fn encode(&self, text: &str) -> Result<Vec<u32>, String>;

    fn count(&self, text: &str) -> TokenCount {
        match self.encode(text) {
            Ok(ids) => TokenCount {
                tokens: ids.len(),
                accuracy: TokenAccuracy::Exact,
                tokenizer: self.name().to_string(),
            },
            Err(_) => EstimatedTokenizer.count(text),
        }
    }
}

pub struct EstimatedTokenizer;

impl Tokenizer for EstimatedTokenizer {
    fn name(&self) -> &'static str {
        "unicode-chars/4"
    }

    fn encode(&self, text: &str) -> Result<Vec<u32>, String> {
        let count = text.chars().count().div_ceil(4);
        Ok((0..count).map(|value| value as u32).collect())
    }

    fn count(&self, text: &str) -> TokenCount {
        TokenCount {
            tokens: text.chars().count().div_ceil(4),
            accuracy: TokenAccuracy::Estimated,
            tokenizer: self.name().to_string(),
        }
    }
}

pub struct DeepSeekTokenizer;

impl DeepSeekTokenizer {
    fn inner() -> Result<&'static HuggingFaceTokenizer, String> {
        static TOKENIZER: OnceLock<Result<HuggingFaceTokenizer, String>> = OnceLock::new();
        TOKENIZER
            .get_or_init(|| {
                HuggingFaceTokenizer::from_bytes(include_bytes!(
                    "assets/deepseek-v3-tokenizer.json"
                ))
                .map_err(|error| error.to_string())
            })
            .as_ref()
            .map_err(Clone::clone)
    }
}

pub struct DeepSeekV4Tokenizer;

impl DeepSeekV4Tokenizer {
    fn inner() -> Result<&'static HuggingFaceTokenizer, String> {
        static TOKENIZER: OnceLock<Result<HuggingFaceTokenizer, String>> = OnceLock::new();
        TOKENIZER
            .get_or_init(|| {
                HuggingFaceTokenizer::from_bytes(include_bytes!(
                    "assets/deepseek-v4-tokenizer.json"
                ))
                .map_err(|error| error.to_string())
            })
            .as_ref()
            .map_err(Clone::clone)
    }
}

impl Tokenizer for DeepSeekV4Tokenizer {
    fn name(&self) -> &'static str {
        "deepseek-ai/DeepSeek-V4"
    }

    fn encode(&self, text: &str) -> Result<Vec<u32>, String> {
        Self::inner()?
            .encode(text, false)
            .map(|encoding| encoding.get_ids().to_vec())
            .map_err(|error| error.to_string())
    }
}

impl Tokenizer for DeepSeekTokenizer {
    fn name(&self) -> &'static str {
        "deepseek-ai/DeepSeek-V3"
    }

    fn encode(&self, text: &str) -> Result<Vec<u32>, String> {
        Self::inner()?
            .encode(text, false)
            .map(|encoding| encoding.get_ids().to_vec())
            .map_err(|error| error.to_string())
    }
}

pub struct OpenAiTokenizer {
    model: String,
}

impl OpenAiTokenizer {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }

    fn bpe(&self) -> Result<&CoreBPE, String> {
        if let Ok(bpe) = bpe_for_model(&self.model) {
            return Ok(bpe);
        }
        static FALLBACK: OnceLock<Result<CoreBPE, String>> = OnceLock::new();
        FALLBACK
            .get_or_init(|| cl100k_base().map_err(|error| error.to_string()))
            .as_ref()
            .map_err(Clone::clone)
    }
}

impl Tokenizer for OpenAiTokenizer {
    fn name(&self) -> &'static str {
        "openai/tiktoken"
    }

    fn encode(&self, text: &str) -> Result<Vec<u32>, String> {
        Ok(self
            .bpe()?
            .encode_ordinary(text)
            .into_iter()
            .map(|token| token as u32)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepseek_official_tokenizer_encodes_known_text_consistently() {
        let tokenizer = DeepSeekTokenizer;
        let ids = tokenizer.encode("Hello, world!").unwrap();
        assert!(!ids.is_empty());
        assert_eq!(ids, tokenizer.encode("Hello, world!").unwrap());
        assert_eq!(
            tokenizer.count("Hello, world!").accuracy,
            TokenAccuracy::Exact
        );
    }

    #[test]
    fn deepseek_v4_official_tokenizer_is_exact() {
        let tokenizer = DeepSeekV4Tokenizer;
        let ids = tokenizer
            .encode("DeepSeek V4 supports a 1M context window.")
            .unwrap();
        assert!(!ids.is_empty());
        assert_eq!(
            tokenizer
                .count("DeepSeek V4 supports a 1M context window.")
                .accuracy,
            TokenAccuracy::Exact
        );
    }

    #[test]
    fn estimated_tokenizer_is_marked_estimated() {
        let count = EstimatedTokenizer.count("12345678");
        assert_eq!(count.tokens, 2);
        assert_eq!(count.accuracy, TokenAccuracy::Estimated);
    }
}
