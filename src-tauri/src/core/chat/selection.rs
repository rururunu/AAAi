const SELECTION_MARKER: &str = "\n\n<peek-selection lines=\"";

pub fn visible_user_text(content: &str) -> &str {
    content
        .split_once(SELECTION_MARKER)
        .map(|(message, _)| message.trim())
        .unwrap_or_else(|| content.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_selection_attachment_from_rule_and_memory_text() {
        let content =
            "Explain this\n\n<peek-selection lines=\"2\">\nfirst\nsecond\n</peek-selection>";
        assert_eq!(visible_user_text(content), "Explain this");
    }

    #[test]
    fn preserves_regular_user_text() {
        assert_eq!(
            visible_user_text("  regular question  "),
            "regular question"
        );
    }
}
