const SELECTION_MARKER: &str = "\n\n<peek-selection lines=\"";

fn strip_attached_files(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(start) = rest.find("<peek-attached-file") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        if let Some(end) = after.find("/>") {
            // Prefer the earliest closer so a self-closing tag doesn't skip a later body tag.
            let body_close = after.find("</peek-attached-file>");
            match body_close {
                Some(close) if close < end => {
                    rest = &after[close + "</peek-attached-file>".len()..];
                }
                _ => {
                    rest = &after[end + 2..];
                }
            }
            continue;
        }
        if let Some(close) = after.find("</peek-attached-file>") {
            rest = &after[close + "</peek-attached-file>".len()..];
            continue;
        }
        // Malformed — keep the rest verbatim.
        out.push_str(after);
        return out;
    }
    out.push_str(rest);
    out
}

pub fn visible_user_text(content: &str) -> String {
    let without_selection = content
        .split_once(SELECTION_MARKER)
        .map(|(message, _)| message)
        .unwrap_or(content);
    strip_attached_files(without_selection).trim().to_string()
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
    fn hides_attached_file_payload_from_titles() {
        let content = "Please review\n\n<peek-attached-file name=\"README.md\" path=\"README.md\">\n# title\n</peek-attached-file>";
        assert_eq!(visible_user_text(content), "Please review");
    }

    #[test]
    fn preserves_regular_user_text() {
        assert_eq!(
            visible_user_text("  regular question  "),
            "regular question"
        );
    }
}
