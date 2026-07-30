use std::path::PathBuf;

use crate::core::context::models::{CaptureSource, WindowInfo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureResult {
    Success(PartialCapture),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialCapture {
    pub selected_text: Option<String>,
    pub selected_files: Vec<PathBuf>,
    pub selected_images: Vec<String>,
    pub source: CaptureSource,
}

pub trait CaptureProvider {
    fn capture(&self, window: &WindowInfo) -> CaptureResult;
}
