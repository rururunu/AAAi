use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeDiffRequest {
    pub old_text: Option<String>,
    pub new_text: Option<String>,
    #[serde(default)]
    pub unified_diff: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeDiffDocument {
    pub rows: Vec<CodeDiffRow>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeDiffRow {
    pub left: Option<CodeDiffLine>,
    pub right: Option<CodeDiffLine>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeDiffLine {
    pub line_number: usize,
    pub text: String,
    pub kind: CodeDiffLineKind,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CodeDiffLineKind {
    Context,
    Addition,
    Deletion,
}

#[tauri::command]
pub fn build_code_diff(request: CodeDiffRequest) -> CodeDiffDocument {
    match (request.old_text, request.new_text) {
        (Some(old_text), Some(new_text)) if !old_text.is_empty() || !new_text.is_empty() => {
            diff_text(&old_text, &new_text)
        }
        _ => parse_unified_diff(&request.unified_diff),
    }
}

fn diff_text(old_text: &str, new_text: &str) -> CodeDiffDocument {
    let diff = TextDiff::from_lines(old_text, new_text);
    let mut rows = Vec::new();
    let mut deletions = Vec::new();
    let mut additions = Vec::new();

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                flush_changed_rows(&mut rows, &mut deletions, &mut additions);
                let line = CodeDiffLine {
                    line_number: change.old_index().unwrap_or_default() + 1,
                    text: line_text(change.value()),
                    kind: CodeDiffLineKind::Context,
                };
                rows.push(CodeDiffRow {
                    left: Some(line.clone()),
                    right: Some(CodeDiffLine {
                        line_number: change.new_index().unwrap_or_default() + 1,
                        ..line
                    }),
                });
            }
            ChangeTag::Delete => deletions.push(CodeDiffLine {
                line_number: change.old_index().unwrap_or_default() + 1,
                text: line_text(change.value()),
                kind: CodeDiffLineKind::Deletion,
            }),
            ChangeTag::Insert => additions.push(CodeDiffLine {
                line_number: change.new_index().unwrap_or_default() + 1,
                text: line_text(change.value()),
                kind: CodeDiffLineKind::Addition,
            }),
        }
    }

    flush_changed_rows(&mut rows, &mut deletions, &mut additions);
    CodeDiffDocument { rows }
}

fn parse_unified_diff(diff: &str) -> CodeDiffDocument {
    let mut rows = Vec::new();
    let mut deletions = Vec::new();
    let mut additions = Vec::new();
    let mut old_line = 0usize;
    let mut new_line = 0usize;

    for raw in diff.replace("\r\n", "\n").lines() {
        if let Some((old_start, new_start)) = hunk_starts(raw) {
            flush_changed_rows(&mut rows, &mut deletions, &mut additions);
            old_line = old_start;
            new_line = new_start;
            continue;
        }
        if raw.starts_with("--- ")
            || raw.starts_with("+++ ")
            || raw.starts_with("diff ")
            || raw.starts_with("index ")
        {
            continue;
        }
        if let Some(text) = raw.strip_prefix('-') {
            deletions.push(CodeDiffLine {
                line_number: old_line,
                text: text.to_owned(),
                kind: CodeDiffLineKind::Deletion,
            });
            old_line += 1;
            continue;
        }
        if let Some(text) = raw.strip_prefix('+') {
            additions.push(CodeDiffLine {
                line_number: new_line,
                text: text.to_owned(),
                kind: CodeDiffLineKind::Addition,
            });
            new_line += 1;
            continue;
        }
        if let Some(text) = raw.strip_prefix(' ') {
            flush_changed_rows(&mut rows, &mut deletions, &mut additions);
            rows.push(CodeDiffRow {
                left: Some(CodeDiffLine {
                    line_number: old_line,
                    text: text.to_owned(),
                    kind: CodeDiffLineKind::Context,
                }),
                right: Some(CodeDiffLine {
                    line_number: new_line,
                    text: text.to_owned(),
                    kind: CodeDiffLineKind::Context,
                }),
            });
            old_line += 1;
            new_line += 1;
        }
    }

    flush_changed_rows(&mut rows, &mut deletions, &mut additions);
    CodeDiffDocument { rows }
}

fn hunk_starts(line: &str) -> Option<(usize, usize)> {
    let mut parts = line.split_whitespace();
    if parts.next()? != "@@" {
        return None;
    }
    let old_start = parts
        .next()?
        .strip_prefix('-')?
        .split(',')
        .next()?
        .parse()
        .ok()?;
    let new_start = parts
        .next()?
        .strip_prefix('+')?
        .split(',')
        .next()?
        .parse()
        .ok()?;
    Some((old_start, new_start))
}

fn flush_changed_rows(
    rows: &mut Vec<CodeDiffRow>,
    deletions: &mut Vec<CodeDiffLine>,
    additions: &mut Vec<CodeDiffLine>,
) {
    let row_count = deletions.len().max(additions.len());
    rows.extend((0..row_count).map(|index| CodeDiffRow {
        left: deletions.get(index).cloned(),
        right: additions.get(index).cloned(),
    }));
    deletions.clear();
    additions.clear();
}

fn line_text(value: &str) -> String {
    value.trim_end_matches(['\r', '\n']).to_owned()
}

#[cfg(test)]
mod tests {
    use super::{build_code_diff, diff_text, parse_unified_diff, CodeDiffLineKind, CodeDiffRequest};

    #[test]
    fn aligns_replacements_with_similar() {
        let document = diff_text("one\nold\nthree\n", "one\nnew\nthree\n");
        assert_eq!(document.rows.len(), 3);
        assert_eq!(
            document.rows[1].left.as_ref().unwrap().kind,
            CodeDiffLineKind::Deletion
        );
        assert_eq!(
            document.rows[1].right.as_ref().unwrap().kind,
            CodeDiffLineKind::Addition
        );
        assert_eq!(document.rows[1].left.as_ref().unwrap().line_number, 2);
        assert_eq!(document.rows[1].right.as_ref().unwrap().line_number, 2);
    }

    #[test]
    fn keeps_empty_counterpart_for_insertions() {
        let document = diff_text("one\n", "one\ntwo\nthree\n");
        assert_eq!(document.rows.len(), 3);
        assert!(document.rows[1].left.is_none());
        assert_eq!(document.rows[1].right.as_ref().unwrap().line_number, 2);
    }

    #[test]
    fn parses_legacy_unified_diff() {
        let document =
            parse_unified_diff("--- a/file\n+++ b/file\n@@ -3,2 +3,2 @@\n old\n-old\n+new\n");
        assert_eq!(document.rows.len(), 2);
        assert_eq!(document.rows[0].left.as_ref().unwrap().line_number, 3);
        assert_eq!(
            document.rows[1].right.as_ref().unwrap().kind,
            CodeDiffLineKind::Addition
        );
    }

    #[test]
    fn falls_back_to_unified_diff_when_preview_text_is_empty() {
        let document = build_code_diff(CodeDiffRequest {
            old_text: Some(String::new()),
            new_text: Some(String::new()),
            unified_diff: "@@ -1 +1 @@\n-old\n+new\n".to_owned(),
        });

        assert_eq!(document.rows.len(), 1);
        assert_eq!(document.rows[0].left.as_ref().unwrap().text, "old");
        assert_eq!(document.rows[0].right.as_ref().unwrap().text, "new");
    }
}
