use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use image::ImageReader;
use windows::core::w;
use windows::Win32::Foundation::{HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

use crate::core::context::models::CaptureError;

const CF_DIB: u32 = 8;
const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
const JPEG_QUALITY: u8 = 85;

static CF_PNG: OnceLock<u32> = OnceLock::new();

pub fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "heic" | "heif"
            )
        })
        .unwrap_or(false)
}

pub fn partition_selected_files(files: Vec<PathBuf>) -> (Vec<PathBuf>, Vec<String>) {
    let mut selected_files = Vec::new();
    let mut selected_images = Vec::new();

    for path in files {
        if is_image_path(&path) {
            if let Ok(data_url) = file_to_data_url(&path) {
                selected_images.push(data_url);
                continue;
            }
        }
        selected_files.push(path);
    }

    (selected_files, selected_images)
}

pub fn file_to_data_url(path: &Path) -> Result<String, CaptureError> {
    let bytes = std::fs::read(path).map_err(|error| {
        CaptureError::ClipboardFailed(format!("read image file failed: {error}"))
    })?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(CaptureError::ClipboardFailed(
            "selected image is too large".into(),
        ));
    }

    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        _ => "image/jpeg",
    };

    if mime == "image/jpeg" || mime == "image/png" || mime == "image/gif" || mime == "image/webp" {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        return Ok(format!("data:{mime};base64,{encoded}"));
    }

    encode_image_bytes_as_jpeg_data_url(&bytes)
        .ok_or_else(|| CaptureError::ClipboardFailed("unsupported image file".into()))
}

/// Read an image currently on the clipboard and return a JPEG data URL.
pub fn read_open_clipboard_image_data_url() -> Option<String> {
    read_cf_png().or_else(read_cf_dib_as_jpeg_data_url)
}

pub fn read_clipboard_image_data_url() -> Option<String> {
    unsafe {
        if OpenClipboard(HWND::default()).is_err() {
            return None;
        }
        let image = read_open_clipboard_image_data_url();
        let _ = CloseClipboard();
        image
    }
}

fn png_format() -> u32 {
    *CF_PNG.get_or_init(|| unsafe { RegisterClipboardFormatW(w!("PNG")) })
}

fn read_cf_png() -> Option<String> {
    unsafe {
        if IsClipboardFormatAvailable(png_format()).is_err() {
            return None;
        }
        let handle = GetClipboardData(png_format()).ok()?;
        if handle.0.is_null() {
            return None;
        }
        let bytes = read_global_bytes(HGLOBAL(handle.0))?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return None;
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        Some(format!("data:image/png;base64,{encoded}"))
    }
}

fn read_cf_dib_as_jpeg_data_url() -> Option<String> {
    unsafe {
        if IsClipboardFormatAvailable(CF_DIB).is_err() {
            return None;
        }
        let handle = GetClipboardData(CF_DIB).ok()?;
        if handle.0.is_null() {
            return None;
        }
        let dib = read_global_bytes(HGLOBAL(handle.0))?;
        encode_dib_as_jpeg_data_url(&dib)
    }
}

unsafe fn read_global_bytes(global: HGLOBAL) -> Option<Vec<u8>> {
    let size = GlobalSize(global);
    if size == 0 {
        return None;
    }
    let ptr = GlobalLock(global);
    if ptr.is_null() {
        return None;
    }
    let bytes = std::slice::from_raw_parts(ptr as *const u8, size as usize).to_vec();
    let _ = GlobalUnlock(global);
    Some(bytes)
}

fn encode_dib_as_jpeg_data_url(dib: &[u8]) -> Option<String> {
    let bmp = wrap_dib_in_bmp(dib)?;
    encode_image_bytes_as_jpeg_data_url(&bmp)
}

fn wrap_dib_in_bmp(dib: &[u8]) -> Option<Vec<u8>> {
    if dib.len() < 40 {
        return None;
    }
    let header_size = u32::from_le_bytes(dib.get(0..4)?.try_into().ok()?) as usize;
    if header_size < 40 || dib.len() < header_size {
        return None;
    }

    let bit_count = u16::from_le_bytes(dib.get(14..16)?.try_into().ok()?);
    let mut offset = 14u32 + header_size as u32;
    if bit_count <= 8 {
        let colors_used = u32::from_le_bytes(dib.get(32..36)?.try_into().ok()?);
        let color_count = if colors_used == 0 {
            1u32 << bit_count
        } else {
            colors_used
        };
        offset += color_count * 4;
    }

    let mut bmp = Vec::with_capacity(14 + dib.len());
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((14 + dib.len()) as u32).to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&offset.to_le_bytes());
    bmp.extend_from_slice(dib);
    Some(bmp)
}

fn encode_image_bytes_as_jpeg_data_url(bytes: &[u8]) -> Option<String> {
    let image = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let mut jpeg_bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_bytes, JPEG_QUALITY);
    encoder.encode_image(&image).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(jpeg_bytes);
    Some(format!("data:image/jpeg;base64,{encoded}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_image_paths_by_extension() {
        assert!(is_image_path(Path::new("photo.PNG")));
        assert!(!is_image_path(Path::new("notes.txt")));
    }
}
