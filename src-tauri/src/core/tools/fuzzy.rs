//! Narrow fuzzy matching for file edits (ported from Reasonix encoding_helpers).
//! Exact match first; then tolerate trailing whitespace, tab/space indent, and
//! read_file line-number prefixes. Non-replace_all still requires a unique hit.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditApplyResult {
    pub updated: String,
    pub applied: usize,
    pub matches: usize,
    pub fuzzy: bool,
}

#[derive(Debug, Clone, Copy)]
struct EditRange {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
struct LineSegment {
    raw: String,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct FuzzyMode {
    strip_old_read_prefixes: bool,
    trim_trailing: bool,
    expand_tabs: bool,
    trim_leading: bool,
}

pub fn apply_old_string_edit(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> EditApplyResult {
    let (old, new_str) = match_line_endings(content, old_string, new_string);
    if replace_all {
        let count = content.matches(old.as_str()).count();
        if count > 0 {
            return EditApplyResult {
                updated: content.replace(&old, &new_str),
                applied: count,
                matches: count,
                fuzzy: false,
            };
        }
        let ranges = fuzzy_edit_ranges(content, &old);
        if ranges.is_empty() {
            return EditApplyResult {
                updated: content.to_string(),
                applied: 0,
                matches: 0,
                fuzzy: false,
            };
        }
        let replacement = match_replacement_line_endings(content, &new_str);
        return EditApplyResult {
            updated: replace_edit_ranges(content, &ranges, &replacement),
            applied: ranges.len(),
            matches: ranges.len(),
            fuzzy: true,
        };
    }

    match content.matches(old.as_str()).count() {
        0 => {
            let ranges = fuzzy_edit_ranges(content, &old);
            if ranges.len() != 1 {
                return EditApplyResult {
                    updated: content.to_string(),
                    applied: 0,
                    matches: ranges.len(),
                    fuzzy: false,
                };
            }
            let replacement = match_replacement_line_endings(content, &new_str);
            EditApplyResult {
                updated: replace_edit_ranges(content, &ranges, &replacement),
                applied: 1,
                matches: 1,
                fuzzy: true,
            }
        }
        1 => EditApplyResult {
            updated: content.replacen(&old, &new_str, 1),
            applied: 1,
            matches: 1,
            fuzzy: false,
        },
        count => EditApplyResult {
            updated: content.to_string(),
            applied: 0,
            matches: count,
            fuzzy: false,
        },
    }
}

fn match_line_endings(content: &str, old: &str, new: &str) -> (String, String) {
    let replacement = match_replacement_line_endings(content, new);
    if content.contains(old) || !content.contains("\r\n") {
        return (old.to_string(), replacement);
    }
    let old_crlf = to_crlf(old);
    if content.contains(&old_crlf) {
        return (old_crlf, replacement);
    }
    (old.to_string(), replacement)
}

fn to_crlf(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\n', "\r\n")
}

fn match_replacement_line_endings(content: &str, replacement: &str) -> String {
    if content.contains("\r\n") {
        to_crlf(replacement)
    } else {
        replacement.to_string()
    }
}

fn fuzzy_edit_ranges(content: &str, old: &str) -> Vec<EditRange> {
    if old.is_empty() || content.is_empty() {
        return Vec::new();
    }
    let content_lines = split_line_segments(content);
    let old_lines = split_line_segments(old);
    if old_lines.is_empty() || old_lines.len() > content_lines.len() {
        return Vec::new();
    }

    let old_has_read_prefixes = all_lines_have_read_file_prefix(&old_lines);
    let mut modes = vec![
        FuzzyMode {
            trim_trailing: true,
            ..Default::default()
        },
        FuzzyMode {
            trim_trailing: true,
            expand_tabs: true,
            ..Default::default()
        },
    ];
    if old_has_read_prefixes {
        modes.push(FuzzyMode {
            strip_old_read_prefixes: true,
            trim_trailing: true,
            ..Default::default()
        });
        modes.push(FuzzyMode {
            strip_old_read_prefixes: true,
            trim_trailing: true,
            expand_tabs: true,
            ..Default::default()
        });
    }

    for mode in modes {
        let norm_old: Vec<String> = old_lines
            .iter()
            .map(|line| {
                normalize_fuzzy_line(
                    &line.raw,
                    line_has_newline(&line.raw),
                    mode,
                    mode.strip_old_read_prefixes,
                )
            })
            .collect();
        let mut ranges = Vec::new();
        let mut i = 0;
        while i <= content_lines.len() - old_lines.len() {
            if fuzzy_window_matches(
                &content_lines[i..i + old_lines.len()],
                &old_lines,
                &norm_old,
                mode,
            ) {
                ranges.push(EditRange {
                    start: content_lines[i].start,
                    end: fuzzy_window_end(
                        &content_lines[i + old_lines.len() - 1],
                        &old_lines[old_lines.len() - 1],
                    ),
                });
                i += old_lines.len();
            } else {
                i += 1;
            }
        }
        if !ranges.is_empty() {
            return ranges;
        }
    }
    Vec::new()
}

fn fuzzy_window_matches(
    content_window: &[LineSegment],
    old_lines: &[LineSegment],
    norm_old: &[String],
    mode: FuzzyMode,
) -> bool {
    for (i, content_line) in content_window.iter().enumerate() {
        let old_has_newline = line_has_newline(&old_lines[i].raw);
        if old_has_newline && !line_has_newline(&content_line.raw) {
            return false;
        }
        let got = normalize_fuzzy_line(&content_line.raw, old_has_newline, mode, false);
        if got != norm_old[i] {
            return false;
        }
    }
    true
}

fn split_line_segments(s: &str) -> Vec<LineSegment> {
    if s.is_empty() {
        return Vec::new();
    }
    let bytes = s.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            let end = i + 1;
            lines.push(LineSegment {
                raw: s[start..end].to_string(),
                start,
                end,
            });
            start = end;
        }
    }
    if start < s.len() {
        lines.push(LineSegment {
            raw: s[start..].to_string(),
            start,
            end: s.len(),
        });
    }
    lines
}

fn line_has_newline(line: &str) -> bool {
    line.ends_with('\n')
}

fn fuzzy_window_end(content_last: &LineSegment, old_last: &LineSegment) -> usize {
    if line_has_newline(&old_last.raw) || !line_has_newline(&content_last.raw) {
        return content_last.end;
    }
    let mut end = content_last.end.saturating_sub(1);
    let raw = content_last.raw.as_bytes();
    if end > content_last.start && raw.len() >= 2 && raw[raw.len() - 2] == b'\r' {
        end -= 1;
    }
    end
}

fn normalize_fuzzy_line(
    line: &str,
    include_newline: bool,
    mode: FuzzyMode,
    strip_read: bool,
) -> String {
    let mut body = line.strip_suffix('\n').unwrap_or(line).to_string();
    if strip_read {
        if let Some(stripped) = strip_read_file_line_prefix(&body) {
            body = stripped;
        }
    }
    if mode.trim_trailing {
        while body.ends_with(' ') || body.ends_with('\t') || body.ends_with('\r') {
            body.pop();
        }
    }
    if mode.expand_tabs {
        body = body.replace('\t', "    ");
    }
    if mode.trim_leading {
        body = body.trim_start_matches([' ', '\t']).to_string();
    }
    if include_newline {
        body.push('\n');
    }
    body
}

fn all_lines_have_read_file_prefix(lines: &[LineSegment]) -> bool {
    !lines.is_empty()
        && lines.iter().all(|line| {
            let body = line.raw.strip_suffix('\n').unwrap_or(&line.raw);
            strip_read_file_line_prefix(body).is_some()
        })
}

/// Strip `   12→` style prefixes from read_file numbered lines.
fn strip_read_file_line_prefix(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    let j_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == j_start {
        return None;
    }
    // U+2192 RIGHTWARDS ARROW as UTF-8: E2 86 92
    if i + 3 <= bytes.len() && bytes[i] == 0xE2 && bytes[i + 1] == 0x86 && bytes[i + 2] == 0x92 {
        return Some(line[i + 3..].to_string());
    }
    None
}

fn replace_edit_ranges(content: &str, ranges: &[EditRange], replacement: &str) -> String {
    let mut updated = content.to_string();
    for range in ranges.iter().rev() {
        updated.replace_range(range.start..range.end, replacement);
    }
    updated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_preferred() {
        let result = apply_old_string_edit("hello world", "world", "there", false);
        assert!(!result.fuzzy);
        assert_eq!(result.updated, "hello there");
        assert_eq!(result.applied, 1);
    }

    #[test]
    fn fuzzy_trailing_whitespace() {
        let content = "fn main() {  \n";
        let result =
            apply_old_string_edit(content, "fn main() {\n", "fn main() {\n    // hi\n", false);
        assert!(result.fuzzy);
        assert_eq!(result.applied, 1);
        assert!(result.updated.contains("// hi"));
    }

    #[test]
    fn fuzzy_rejects_ambiguous() {
        let content = "aa  \naa  \n";
        let result = apply_old_string_edit(content, "aa\n", "bb\n", false);
        assert_eq!(result.applied, 0);
        assert!(result.matches >= 2);
    }

    #[test]
    fn exact_replacement_preserves_crlf_for_inserted_lines() {
        let content = "before\r\ntarget\r\nafter\r\n";
        let result = apply_old_string_edit(content, "target", "first\nsecond", false);
        assert_eq!(result.updated, "before\r\nfirst\r\nsecond\r\nafter\r\n");
        assert!(!result.updated.replace("\r\n", "").contains('\n'));
    }
}
