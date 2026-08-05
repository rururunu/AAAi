//! Microsoft PowerPoint COM automation (via COM worker thread).

use windows::core::VARIANT;

use super::com::{ComDispatch, ComError, ComValue};
use super::worker;

pub const POWERPOINT_PROG_ID: &str = "PowerPoint.Application";
const NO_ACTIVE_PRESENTATION_MSG: &str = "PowerPoint has no active presentation";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerPointError {
    Com(ComError),
    NoActivePresentation,
}

impl std::fmt::Display for PowerPointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Com(error) => write!(f, "{error}"),
            Self::NoActivePresentation => write!(f, "PowerPoint has no active presentation"),
        }
    }
}

impl std::error::Error for PowerPointError {}

impl From<ComError> for PowerPointError {
    fn from(error: ComError) -> Self {
        if matches!(&error, ComError::Type(message) if message == NO_ACTIVE_PRESENTATION_MSG) {
            return Self::NoActivePresentation;
        }
        Self::Com(error)
    }
}

pub struct PowerPointSnapshot {
    pub presentation_path: Option<String>,
    pub presentation_name: Option<String>,
    pub slide_index: Option<i32>,
    pub slide_count: Option<i32>,
    pub selected_text: Option<String>,
}

pub fn powerpoint_is_available() -> bool {
    worker::app_available(POWERPOINT_PROG_ID)
}

pub fn collect_powerpoint_snapshot() -> Result<PowerPointSnapshot, PowerPointError> {
    worker::with_app_value(POWERPOINT_PROG_ID, collect_powerpoint_snapshot_inner)
        .map_err(PowerPointError::from)
}

fn collect_powerpoint_snapshot_inner(app: &ComDispatch) -> Result<PowerPointSnapshot, ComError> {
    let presentation = active_presentation(app)?;
    let window = app.get("ActiveWindow").ok().and_then(|v| v.into_dispatch().ok());
    let selection = window
        .as_ref()
        .and_then(|w| w.get("Selection").ok())
        .and_then(|v| v.into_dispatch().ok());

    let presentation_name = optional_string(presentation.get("Name").ok());
    let presentation_path = optional_string(presentation.get("FullName").ok());
    let slide_count = presentation.get("Slides").ok().and_then(|slides| {
        slides
            .into_dispatch()
            .ok()?
            .get("Count")
            .ok()?
            .into_int()
            .ok()
    });
    let slide_index = window.and_then(|w| {
        w.get("View")
            .ok()?
            .into_dispatch()
            .ok()?
            .get("Slide")
            .ok()?
            .into_dispatch()
            .ok()?
            .get("SlideIndex")
            .ok()?
            .into_int()
            .ok()
    });
    let selected_text = selection.and_then(|s| optional_string(s.get("Text").ok()));

    Ok(PowerPointSnapshot {
        presentation_path,
        presentation_name,
        slide_index,
        slide_count,
        selected_text,
    })
}

pub fn get_selection_text() -> Result<String, PowerPointError> {
    worker::with_app_value(POWERPOINT_PROG_ID, |app| {
        let window = app.get("ActiveWindow")?.into_dispatch()?;
        let selection = window.get("Selection")?.into_dispatch()?;
        selection.get("Text")?.into_string()
    })
    .map_err(PowerPointError::from)
}

pub fn get_slide_text(max_chars: usize) -> Result<String, PowerPointError> {
    worker::with_app_value(POWERPOINT_PROG_ID, move |app| {
        let window = app.get("ActiveWindow")?.into_dispatch()?;
        let slide = window
            .get("View")?
            .into_dispatch()?
            .get("Slide")?
            .into_dispatch()?;
        let shapes = slide.get("Shapes")?.into_dispatch()?;
        let count = shapes.get("Count")?.into_int()?;
        let mut parts = Vec::new();
        for index in 1..=count {
            let shape = shapes
                .call("Item", &[VARIANT::from(index)])?
                .into_dispatch()?;
            if let Ok(text_frame) = shape.get("TextFrame") {
                if let Ok(has_text) = text_frame
                    .into_dispatch()?
                    .get("HasText")
                    .and_then(|v| v.into_bool())
                {
                    if has_text {
                        if let Ok(text) = shape
                            .get("TextFrame")?
                            .into_dispatch()?
                            .get("TextRange")?
                            .into_dispatch()?
                            .get("Text")
                            .and_then(|v| v.into_string())
                        {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                parts.push(trimmed.to_string());
                            }
                        }
                    }
                }
            }
        }
        let combined = parts.join("\n\n");
        Ok(truncate_chars(&combined, max_chars))
    })
    .map_err(PowerPointError::from)
}

pub fn replace_selection_text(text: &str) -> Result<String, PowerPointError> {
    let payload = text.to_string();
    worker::with_app_value(POWERPOINT_PROG_ID, move |app| {
        let window = app.get("ActiveWindow")?.into_dispatch()?;
        let selection = window.get("Selection")?.into_dispatch()?;
        selection.set("Text", VARIANT::from(payload.as_str()))?;
        Ok(format!("Replaced selection ({} chars)", payload.chars().count()))
    })
    .map_err(PowerPointError::from)
}

pub fn insert_text_at_cursor(text: &str) -> Result<String, PowerPointError> {
    let payload = text.to_string();
    worker::with_app_value(POWERPOINT_PROG_ID, move |app| {
        let window = app.get("ActiveWindow")?.into_dispatch()?;
        let selection = window.get("Selection")?.into_dispatch()?;
        selection.set("Text", VARIANT::from(payload.as_str()))?;
        Ok(format!("Inserted text ({} chars)", payload.chars().count()))
    })
    .map_err(PowerPointError::from)
}

pub fn save_active_presentation() -> Result<String, PowerPointError> {
    worker::with_app_value(POWERPOINT_PROG_ID, |app| {
        let presentation = active_presentation(app)?;
        presentation.call("Save", &[])?;
        let name =
            optional_string(presentation.get("Name").ok()).unwrap_or_else(|| "presentation".into());
        Ok(format!("Saved `{name}`"))
    })
    .map_err(PowerPointError::from)
}

fn active_presentation(app: &ComDispatch) -> Result<ComDispatch, ComError> {
    match app.get("ActivePresentation") {
        Ok(value) => value.into_dispatch(),
        Err(ComError::Invoke(_, _)) => Err(ComError::Type(NO_ACTIVE_PRESENTATION_MSG.into())),
        Err(error) => Err(error),
    }
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
    fn powerpoint_operations_fail_gracefully_when_not_running() {
        if powerpoint_is_available() {
            return;
        }
        assert!(get_selection_text().is_err());
        assert!(get_slide_text(100).is_err());
        assert!(save_active_presentation().is_err());
    }
}
