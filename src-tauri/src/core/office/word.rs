//! Microsoft Word COM automation (via COM worker thread).

use windows::core::VARIANT;

use super::com::{ComDispatch, ComError, ComValue};
use super::worker;

pub const WORD_PROG_ID: &str = "Word.Application";
const WD_STATISTIC_PAGES: i32 = 2;
const NO_ACTIVE_DOCUMENT_MSG: &str = "Word has no active document";
const EMPTY_SELECTION_WITH_RANGE_HINT: &str = "Word selection is empty (often cleared when Anya steals focus). Reselect text in Word, or pass start/end from the earlier Office context.";
const EMPTY_SELECTION_NO_RANGE_HINT: &str = "Word selection is empty (often cleared when Anya steals focus). Reselect the text in Word, then retry.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordError {
    Com(ComError),
    NoActiveDocument,
    Operation(String),
}

impl std::fmt::Display for WordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Com(error) => write!(f, "{error}"),
            Self::NoActiveDocument => write!(f, "Word has no active document"),
            Self::Operation(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for WordError {}

impl From<ComError> for WordError {
    fn from(error: ComError) -> Self {
        match &error {
            ComError::Type(message) if message == NO_ACTIVE_DOCUMENT_MSG => Self::NoActiveDocument,
            ComError::Type(message)
                if message == EMPTY_SELECTION_WITH_RANGE_HINT
                    || message == EMPTY_SELECTION_NO_RANGE_HINT =>
            {
                Self::Operation(message.clone())
            }
            _ => Self::Com(error),
        }
    }
}

pub struct WordSnapshot {
    pub document_path: Option<String>,
    pub document_name: Option<String>,
    pub selected_text: Option<String>,
    pub selection_start: Option<i32>,
    pub selection_end: Option<i32>,
    pub document_title: Option<String>,
    pub page_count: Option<i32>,
    pub track_changes_enabled: Option<bool>,
    pub pending_revisions: Option<i32>,
}

pub fn word_is_available() -> bool {
    worker::app_available(WORD_PROG_ID)
}

pub fn collect_word_snapshot() -> Result<WordSnapshot, WordError> {
    worker::with_app_value(WORD_PROG_ID, collect_word_snapshot_inner).map_err(WordError::from)
}

fn collect_word_snapshot_inner(app: &ComDispatch) -> Result<WordSnapshot, ComError> {
    let document = active_document(app)?;
    let selection = app
        .get("Selection")
        .ok()
        .and_then(|value| value.into_dispatch().ok());

    let document_name = optional_string(document.get("Name").ok());
    let document_path = optional_string(document.get("FullName").ok());
    let document_title = builtin_property(&document, "Title");
    let page_count = document
        .call("ComputeStatistics", &[VARIANT::from(WD_STATISTIC_PAGES)])
        .ok()
        .and_then(|value| value.into_int().ok());
    let track_changes_enabled = document
        .get("TrackRevisions")
        .ok()
        .and_then(|value| value.into_bool().ok());
    let pending_revisions = document
        .get("Revisions")
        .ok()
        .and_then(|value| value.into_dispatch().ok())
        .and_then(|revs| revs.get("Count").ok())
        .and_then(|value| value.into_int().ok());

    let (selected_text, selection_start, selection_end) = selection
        .as_ref()
        .map(|sel| {
            let text = optional_string(sel.get("Text").ok());
            let start = sel.get("Start").ok().and_then(|v| v.into_int().ok());
            let end = sel.get("End").ok().and_then(|v| v.into_int().ok());
            (text, start, end)
        })
        .unwrap_or((None, None, None));

    Ok(WordSnapshot {
        document_path,
        document_name,
        selected_text,
        selection_start,
        selection_end,
        document_title,
        page_count,
        track_changes_enabled,
        pending_revisions,
    })
}

pub fn get_document_text(max_chars: usize) -> Result<String, WordError> {
    get_document_range(None, None, max_chars)
}

pub fn get_document_range(
    start_char: Option<i32>,
    end_char: Option<i32>,
    max_chars: usize,
) -> Result<String, WordError> {
    worker::with_app_value(WORD_PROG_ID, move |app| {
        let document = active_document(app)?;
        let text = if let (Some(start), Some(end)) = (start_char, end_char) {
            read_range(&document, start, end)?
        } else {
            let content = document.get("Content")?.into_dispatch()?;
            content.get("Text")?.into_string()?
        };
        Ok(truncate_chars(&text, max_chars))
    })
    .map_err(WordError::from)
}

pub fn get_document_paragraphs(
    start_para: i32,
    count: i32,
    max_chars: usize,
) -> Result<String, WordError> {
    worker::with_app_value(WORD_PROG_ID, move |app| {
        let document = active_document(app)?;
        let paragraphs = document.get("Paragraphs")?.into_dispatch()?;
        let total = paragraphs.get("Count")?.into_int()?;
        if start_para < 1 || start_para > total {
            return Err(ComError::Type(format!(
                "paragraph start {start_para} out of range (1..={total})"
            )));
        }
        let end_para = (start_para + count - 1).min(total);
        let start_range = paragraphs
            .call("Item", &[VARIANT::from(start_para)])?
            .into_dispatch()?
            .get("Range")?
            .into_dispatch()?;
        let end_range = paragraphs
            .call("Item", &[VARIANT::from(end_para)])?
            .into_dispatch()?
            .get("Range")?
            .into_dispatch()?;
        let start = start_range.get("Start")?.into_int()?;
        let end = end_range.get("End")?.into_int()?;
        let text = read_range(&document, start, end)?;
        Ok(format!(
            "Paragraphs {start_para}..{end_para} of {total}\n\n{}",
            truncate_chars(&text, max_chars)
        ))
    })
    .map_err(WordError::from)
}

pub fn get_selection_text() -> Result<String, WordError> {
    worker::with_app_value(WORD_PROG_ID, |app| {
        let selection = app.get("Selection")?.into_dispatch()?;
        selection.get("Text")?.into_string()
    })
    .map_err(WordError::from)
}

/// Replace the current selection, or a previously captured range when focus stole the selection.
pub fn replace_selection_or_range(
    text: &str,
    start: Option<i32>,
    end: Option<i32>,
) -> Result<String, WordError> {
    let payload = text.to_string();
    worker::with_app_value(WORD_PROG_ID, move |app| {
        let document = active_document(app)?;
        let selection = app.get("Selection")?.into_dispatch()?;
        let current_start = selection.get("Start")?.into_int()?;
        let current_end = selection.get("End")?.into_int()?;
        let (range_start, range_end) = if current_start != current_end {
            (current_start, current_end)
        } else if let (Some(start), Some(end)) = (start, end) {
            if start == end {
                return Err(ComError::Type(EMPTY_SELECTION_WITH_RANGE_HINT.into()));
            }
            (start.min(end), start.max(end))
        } else {
            return Err(ComError::Type(EMPTY_SELECTION_NO_RANGE_HINT.into()));
        };

        let range = document
            .call(
                "Range",
                &[VARIANT::from(range_start), VARIANT::from(range_end)],
            )?
            .into_dispatch()?;
        range.set("Text", VARIANT::from(payload.as_str()))?;
        Ok(format!(
            "Replaced range {range_start}..{range_end} ({} chars)",
            payload.chars().count()
        ))
    })
    .map_err(WordError::from)
}

pub fn insert_text_at_cursor(text: &str) -> Result<String, WordError> {
    let payload = text.to_string();
    worker::with_app_value(WORD_PROG_ID, move |app| {
        let selection = app.get("Selection")?.into_dispatch()?;
        selection.call("TypeText", &[VARIANT::from(payload.as_str())])?;
        Ok(format!("Inserted text ({} chars)", payload.chars().count()))
    })
    .map_err(WordError::from)
}

/// Insert a Word table at the current selection.
///
/// `rows` / `cols` are the full grid size (including header). `cells` is row-major
/// flat text (`len == rows * cols`). Prefer python-docx for whole technical bids;
/// this COM helper is for small live edits in an already-open document.
pub fn insert_table_at_selection(
    rows: i32,
    cols: i32,
    cells: &[String],
) -> Result<String, WordError> {
    if rows < 1 || cols < 1 {
        return Err(WordError::Operation(
            "insert_table requires rows >= 1 and cols >= 1".into(),
        ));
    }
    let expected = (rows as usize).saturating_mul(cols as usize);
    if cells.len() != expected {
        return Err(WordError::Operation(format!(
            "insert_table expected {expected} cells (rows*cols), got {}",
            cells.len()
        )));
    }
    let payload = cells.to_vec();
    worker::with_app_value(WORD_PROG_ID, move |app| {
        let selection = app.get("Selection")?.into_dispatch()?;
        let range = selection.get("Range")?.into_dispatch()?;
        // Word: Selection.Tables.Add(Range, NumRows, NumColumns)
        let tables = selection.get("Tables")?.into_dispatch()?;
        let table = tables
            .call(
                "Add",
                &[range.to_variant(), VARIANT::from(rows), VARIANT::from(cols)],
            )?
            .into_dispatch()?;

        for r in 1..=rows {
            for c in 1..=cols {
                let idx = ((r - 1) * cols + (c - 1)) as usize;
                let cell = table
                    .call("Cell", &[VARIANT::from(r), VARIANT::from(c)])?
                    .into_dispatch()?;
                let cell_range = cell.get("Range")?.into_dispatch()?;
                // Range.Text in a cell includes the end-of-cell marker; assign plain text.
                cell_range.set("Text", VARIANT::from(payload[idx].as_str()))?;
            }
        }
        Ok(format!("Inserted table {rows}x{cols}"))
    })
    .map_err(WordError::from)
}

/// Normalize font name/size across the current selection (or a captured range).
///
/// Mitigates mixed run sizes after plain `Range.Text` replacement.
pub fn apply_font_to_selection_or_range(
    font_name: &str,
    size_pt: f64,
    start: Option<i32>,
    end: Option<i32>,
) -> Result<String, WordError> {
    if !(size_pt > 0.0) {
        return Err(WordError::Operation("font size_pt must be > 0".into()));
    }
    let name = font_name.to_string();
    // Word Font.Size is points as f32/f64 via VARIANT; we pass i32 half? Actually Size is float points.
    // Our COM layer mainly uses i32/string; use string-free path via Int for whole points when possible.
    let size_int = size_pt.round() as i32;
    worker::with_app_value(WORD_PROG_ID, move |app| {
        let document = active_document(app)?;
        let selection = app.get("Selection")?.into_dispatch()?;
        let current_start = selection.get("Start")?.into_int()?;
        let current_end = selection.get("End")?.into_int()?;
        let (range_start, range_end) = if current_start != current_end {
            (current_start, current_end)
        } else if let (Some(start), Some(end)) = (start, end) {
            if start == end {
                return Err(ComError::Type(EMPTY_SELECTION_WITH_RANGE_HINT.into()));
            }
            (start.min(end), start.max(end))
        } else {
            return Err(ComError::Type(EMPTY_SELECTION_NO_RANGE_HINT.into()));
        };

        let range = document
            .call(
                "Range",
                &[VARIANT::from(range_start), VARIANT::from(range_end)],
            )?
            .into_dispatch()?;
        let font = range.get("Font")?.into_dispatch()?;
        if !name.trim().is_empty() {
            font.set("Name", VARIANT::from(name.as_str()))?;
            // East Asian face often mirrors NameFarEast on Word COM.
            let _ = font.set("NameFarEast", VARIANT::from(name.as_str()));
        }
        font.set("Size", VARIANT::from(size_int))?;
        Ok(format!(
            "Applied font name=`{name}` size={size_int}pt to range {range_start}..{range_end}"
        ))
    })
    .map_err(WordError::from)
}

pub fn save_active_document() -> Result<String, WordError> {
    worker::with_app_value(WORD_PROG_ID, |app| {
        let document = active_document(app)?;
        document.call("Save", &[])?;
        let name =
            optional_string(document.get("Name").ok()).unwrap_or_else(|| "document".to_string());
        Ok(format!("Saved `{name}`"))
    })
    .map_err(WordError::from)
}

pub fn list_comments(max_items: usize) -> Result<String, WordError> {
    worker::with_app_value(WORD_PROG_ID, move |app| {
        let document = active_document(app)?;
        let comments = document.get("Comments")?.into_dispatch()?;
        let count = comments.get("Count")?.into_int()?;
        let limit = count.min(max_items as i32);
        let mut lines = vec![format!("Comments: {count} total (showing {limit})")];
        for index in 1..=limit {
            let comment = comments
                .call("Item", &[VARIANT::from(index)])?
                .into_dispatch()?;
            let author = optional_string(comment.get("Author").ok()).unwrap_or_else(|| "?".into());
            let text = comment
                .get("Range")?
                .into_dispatch()?
                .get("Text")?
                .into_string()
                .unwrap_or_default()
                .trim()
                .to_string();
            let preview = truncate_chars(&text, 240);
            lines.push(format!("{index}. [{author}] {preview}"));
        }
        Ok(lines.join("\n"))
    })
    .map_err(WordError::from)
}

pub fn add_comment(text: &str, use_selection: bool) -> Result<String, WordError> {
    let payload = text.to_string();
    worker::with_app_value(WORD_PROG_ID, move |app| {
        let document = active_document(app)?;
        let range = if use_selection {
            app.get("Selection")?
                .into_dispatch()?
                .get("Range")?
                .into_dispatch()?
        } else {
            document.get("Content")?.into_dispatch()?
        };
        document.get("Comments")?.into_dispatch()?.call(
            "Add",
            &[range.to_variant(), VARIANT::from(payload.as_str())],
        )?;
        Ok(format!("Added comment ({} chars)", payload.chars().count()))
    })
    .map_err(WordError::from)
}

pub fn accept_all_revisions() -> Result<String, WordError> {
    worker::with_app_value(WORD_PROG_ID, |app| {
        let document = active_document(app)?;
        let count = document
            .get("Revisions")
            .ok()
            .and_then(|value| value.into_dispatch().ok())
            .and_then(|revs| revs.get("Count").ok())
            .and_then(|value| value.into_int().ok())
            .unwrap_or(0);
        if count == 0 {
            return Ok("No revisions to accept".to_string());
        }
        accept_or_reject_all(&document, true)?;
        Ok(format!("Accepted {count} revision(s)"))
    })
    .map_err(WordError::from)
}

pub fn reject_all_revisions() -> Result<String, WordError> {
    worker::with_app_value(WORD_PROG_ID, |app| {
        let document = active_document(app)?;
        let count = document
            .get("Revisions")
            .ok()
            .and_then(|value| value.into_dispatch().ok())
            .and_then(|revs| revs.get("Count").ok())
            .and_then(|value| value.into_int().ok())
            .unwrap_or(0);
        if count == 0 {
            return Ok("No revisions to reject".to_string());
        }
        accept_or_reject_all(&document, false)?;
        Ok(format!("Rejected {count} revision(s)"))
    })
    .map_err(WordError::from)
}

fn accept_or_reject_all(document: &ComDispatch, accept: bool) -> Result<(), ComError> {
    let revisions = document.get("Revisions")?.into_dispatch()?;
    let count = revisions.get("Count")?.into_int()?;
    for index in (1..=count).rev() {
        let revision = revisions
            .call("Item", &[VARIANT::from(index)])?
            .into_dispatch()?;
        if accept {
            revision.call("Accept", &[])?;
        } else {
            revision.call("Reject", &[])?;
        }
    }
    Ok(())
}

fn active_document(app: &ComDispatch) -> Result<ComDispatch, ComError> {
    match app.get("ActiveDocument") {
        Ok(value) => value.into_dispatch(),
        Err(ComError::Invoke(_, _)) => Err(ComError::Type(NO_ACTIVE_DOCUMENT_MSG.into())),
        Err(error) => Err(error),
    }
}

fn read_range(document: &ComDispatch, start: i32, end: i32) -> Result<String, ComError> {
    let range = document
        .call("Range", &[VARIANT::from(start), VARIANT::from(end)])?
        .into_dispatch()?;
    range.get("Text")?.into_string()
}

fn builtin_property(document: &ComDispatch, name: &str) -> Option<String> {
    let properties = document.get("BuiltInDocumentProperties").ok()?;
    let properties = properties.into_dispatch().ok()?;
    let item = properties
        .call("Item", &[VARIANT::from(name)])
        .ok()?
        .into_dispatch()
        .ok()?;
    optional_string(item.get("Value").ok())
}

fn optional_string(value: Option<ComValue>) -> Option<String> {
    let text = value?.into_string().ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max_chars).collect();
    format!("{truncated}…")
}

trait ComValueExt {
    fn into_bool(self) -> Result<bool, ComError>;
}

impl ComValueExt for ComValue {
    fn into_bool(self) -> Result<bool, ComError> {
        match self {
            Self::Bool(value) => Ok(value),
            Self::Int(value) => Ok(value != 0),
            other => Err(ComError::Type(format!("expected bool, got {other:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_operations_fail_gracefully_when_not_running() {
        if word_is_available() {
            return;
        }
        assert!(get_document_text(100).is_err());
        assert!(get_selection_text().is_err());
        assert!(replace_selection_or_range("x", None, None).is_err());
        assert!(insert_text_at_cursor("x").is_err());
        assert!(save_active_document().is_err());
        assert!(accept_all_revisions().is_err());
    }
}
