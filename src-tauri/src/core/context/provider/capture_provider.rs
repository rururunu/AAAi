use std::path::PathBuf;

use crate::core::context::models::{CaptureError, CaptureSource, WindowInfo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureResult {
    Success(PartialCapture),
    Empty,
    Failed(CaptureError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialCapture {
    pub selected_text: Option<String>,
    pub selected_files: Vec<PathBuf>,
    pub source: CaptureSource,
}

pub trait CaptureProvider {
    fn capture(&self, window: &WindowInfo) -> CaptureResult;
}
