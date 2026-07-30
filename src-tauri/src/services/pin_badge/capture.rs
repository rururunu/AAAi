//! Capture a pin window into a JPEG data URL via screen BitBlt.

use std::mem::size_of;

use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use image::{ImageBuffer, Rgba};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

const JPEG_QUALITY: u8 = 85;
const MAX_EDGE: u32 = 4096;

pub fn capture_window_data_url(hwnd: HWND) -> Option<String> {
    let (width, height, pixels) = capture_window_bgra(hwnd)?;
    if width == 0 || height == 0 || pixels.is_empty() {
        return None;
    }

    // BGRA -> RGBA
    let mut rgba = Vec::with_capacity(pixels.len());
    for chunk in pixels.chunks_exact(4) {
        rgba.push(chunk[2]);
        rgba.push(chunk[1]);
        rgba.push(chunk[0]);
        rgba.push(chunk[3]);
    }

    let image: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, rgba)?;

    let image = if width > MAX_EDGE || height > MAX_EDGE {
        let scale = (MAX_EDGE as f32 / width.max(height) as f32).min(1.0);
        let new_w = ((width as f32) * scale).round().max(1.0) as u32;
        let new_h = ((height as f32) * scale).round().max(1.0) as u32;
        image::imageops::resize(&image, new_w, new_h, image::imageops::FilterType::Triangle)
    } else {
        image
    };

    // JPEG encoder expects RGB.
    let rgb = image::DynamicImage::ImageRgba8(image).to_rgb8();
    let mut jpeg_bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_bytes, JPEG_QUALITY);
    encoder.encode_image(&rgb).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(jpeg_bytes);
    Some(format!("data:image/jpeg;base64,{encoded}"))
}

fn capture_window_bgra(hwnd: HWND) -> Option<(u32, u32, Vec<u8>)> {
    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).ok()?;
        let width = (rect.right - rect.left).max(1);
        let height = (rect.bottom - rect.top).max(1);

        // Capture from the screen DC at the window's on-screen bounds.
        // Pin windows are topmost and visible, so this is reliable enough.
        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            return None;
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.0.is_null() {
            ReleaseDC(None, screen_dc);
            return None;
        }
        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.0.is_null() {
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
            return None;
        }

        let old = SelectObject(mem_dc, HGDIOBJ(bitmap.0));
        let blt_ok = BitBlt(
            mem_dc,
            0,
            0,
            width,
            height,
            screen_dc,
            rect.left,
            rect.top,
            SRCCOPY,
        )
        .is_ok();

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let lines = if blt_ok {
            GetDIBits(
                mem_dc,
                bitmap,
                0,
                height as u32,
                Some(pixels.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };

        SelectObject(mem_dc, old);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);

        if lines == 0 {
            return None;
        }

        for chunk in pixels.chunks_exact_mut(4) {
            if chunk[3] == 0 && (chunk[0] != 0 || chunk[1] != 0 || chunk[2] != 0) {
                chunk[3] = 255;
            } else if chunk[3] == 0 {
                chunk[3] = 255;
            }
        }

        Some((width as u32, height as u32, pixels))
    }
}
