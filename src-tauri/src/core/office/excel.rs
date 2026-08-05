//! Microsoft Excel COM automation (via COM worker thread).

use windows::core::VARIANT;

use super::com::{ComDispatch, ComError, ComValue};
use super::worker;

pub const EXCEL_PROG_ID: &str = "Excel.Application";
const NO_ACTIVE_WORKBOOK_MSG: &str = "Excel has no active workbook";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExcelError {
    Com(ComError),
    NoActiveWorkbook,
}

impl std::fmt::Display for ExcelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Com(error) => write!(f, "{error}"),
            Self::NoActiveWorkbook => write!(f, "Excel has no active workbook"),
        }
    }
}

impl std::error::Error for ExcelError {}

impl From<ComError> for ExcelError {
    fn from(error: ComError) -> Self {
        if matches!(&error, ComError::Type(message) if message == NO_ACTIVE_WORKBOOK_MSG) {
            return Self::NoActiveWorkbook;
        }
        Self::Com(error)
    }
}

pub struct ExcelSnapshot {
    pub workbook_path: Option<String>,
    pub workbook_name: Option<String>,
    pub active_sheet: Option<String>,
    pub cell_address: Option<String>,
    pub selected_text: Option<String>,
}

pub fn excel_is_available() -> bool {
    worker::app_available(EXCEL_PROG_ID)
}

pub fn collect_excel_snapshot() -> Result<ExcelSnapshot, ExcelError> {
    worker::with_app_value(EXCEL_PROG_ID, collect_excel_snapshot_inner).map_err(ExcelError::from)
}

fn collect_excel_snapshot_inner(app: &ComDispatch) -> Result<ExcelSnapshot, ComError> {
    let workbook = active_workbook(app)?;
    let sheet = app.get("ActiveSheet").ok().and_then(|v| v.into_dispatch().ok());
    let selection = app.get("Selection").ok().and_then(|v| v.into_dispatch().ok());

    let workbook_name = optional_string(workbook.get("Name").ok());
    let workbook_path = optional_string(workbook.get("FullName").ok());
    let active_sheet = sheet
        .as_ref()
        .and_then(|s| optional_string(s.get("Name").ok()));
    let cell_address = selection
        .as_ref()
        .and_then(|s| optional_string(s.get("Address").ok()));
    let selected_text = selection
        .as_ref()
        .and_then(|s| cell_text(s).ok());

    Ok(ExcelSnapshot {
        workbook_path,
        workbook_name,
        active_sheet,
        cell_address,
        selected_text,
    })
}

pub fn get_selection_text() -> Result<String, ExcelError> {
    worker::with_app_value(EXCEL_PROG_ID, |app| {
        let selection = app.get("Selection")?.into_dispatch()?;
        cell_text(&selection)
    })
    .map_err(ExcelError::from)
}

pub fn get_used_range_text(max_chars: usize) -> Result<String, ExcelError> {
    worker::with_app_value(EXCEL_PROG_ID, move |app| {
        let sheet = app.get("ActiveSheet")?.into_dispatch()?;
        let used = sheet.get("UsedRange")?.into_dispatch()?;
        let text = cell_text(&used)?;
        Ok(truncate_chars(&text, max_chars))
    })
    .map_err(ExcelError::from)
}

pub fn set_selection_value(text: &str) -> Result<String, ExcelError> {
    let payload = text.to_string();
    worker::with_app_value(EXCEL_PROG_ID, move |app| {
        let selection = app.get("Selection")?.into_dispatch()?;
        selection.set("Value", VARIANT::from(payload.as_str()))?;
        Ok(format!("Updated selection ({} chars)", payload.chars().count()))
    })
    .map_err(ExcelError::from)
}

pub fn save_active_workbook() -> Result<String, ExcelError> {
    worker::with_app_value(EXCEL_PROG_ID, |app| {
        let workbook = active_workbook(app)?;
        workbook.call("Save", &[])?;
        let name = optional_string(workbook.get("Name").ok()).unwrap_or_else(|| "workbook".into());
        Ok(format!("Saved `{name}`"))
    })
    .map_err(ExcelError::from)
}

fn active_workbook(app: &ComDispatch) -> Result<ComDispatch, ComError> {
    match app.get("ActiveWorkbook") {
        Ok(value) => value.into_dispatch(),
        Err(ComError::Invoke(_, _)) => Err(ComError::Type(NO_ACTIVE_WORKBOOK_MSG.into())),
        Err(error) => Err(error),
    }
}

fn cell_text(selection: &ComDispatch) -> Result<String, ComError> {
    if let Ok(text) = selection.get("Text").and_then(|v| v.into_string()) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    selection.get("Value")?.into_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excel_operations_fail_gracefully_when_not_running() {
        if excel_is_available() {
            return;
        }
        assert!(get_selection_text().is_err());
        assert!(get_used_range_text(100).is_err());
        assert!(save_active_workbook().is_err());
    }
}
