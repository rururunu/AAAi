//! Native AI badge with the AAAi sparkle mark.
//!
//! The circle and centered four-pointed star are software-rendered with
//! antialiasing, a restrained surface gradient, and a soft ambient shadow.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::{Mutex, OnceLock};

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, CreateDIBSection, CreateEllipticRgn, DeleteDC, DeleteObject,
    EndPaint, GetDC, InvalidateRect, ReleaseDC, SelectObject, SetWindowRgn, UpdateWindow,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    HBRUSH, HGDIOBJ, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, GetWindowLongPtrW, GetWindowRect,
    LoadCursorW, RegisterClassW, SetWindowLongPtrW, SetWindowPos, ShowWindow, UpdateLayeredWindow,
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HICON, HWND_TOPMOST,
    IDC_HAND, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSENDCHANGING, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE,
    SW_SHOWNOACTIVATE, ULW_ALPHA, WM_DESTROY, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCCREATE, WM_PAINT,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use super::theme;

/// Compact badge dimensions in physical pixels.
pub const BUTTON_WIDTH: i32 = 40;
pub const BUTTON_HEIGHT: i32 = 40;
pub const BUTTON_MARGIN: i32 = 8;

const MAIN_CENTER_X: f32 = BUTTON_WIDTH as f32 / 2.0;
const MAIN_CENTER_Y: f32 = BUTTON_HEIGHT as f32 / 2.0 - 0.75;
const MAIN_RADIUS: f32 = 16.5;
const SHADOW_CENTER_Y: f32 = MAIN_CENTER_Y + 1.75;
const SHADOW_INNER_RADIUS: f32 = 16.75;
const SHADOW_OUTER_RADIUS: f32 = 19.25;
const STAR_OUTER_RADIUS: f32 = 9.0;
const STAR_INNER_RADIUS: f32 = 3.2;
const SUPERSAMPLE: usize = 4;

/// winuser.h WM_MOUSELEAVE
const WM_MOUSELEAVE: u32 = 0x02A3;

static CLASS_REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();
static PENDING_CLICKS: OnceLock<Mutex<Vec<isize>>> = OnceLock::new();
static HOVER_STATE: OnceLock<Mutex<HashMap<isize, bool>>> = OnceLock::new();

fn pending_clicks() -> &'static Mutex<Vec<isize>> {
    PENDING_CLICKS.get_or_init(|| Mutex::new(Vec::new()))
}

fn hover_state() -> &'static Mutex<HashMap<isize, bool>> {
    HOVER_STATE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn process_pending_clicks() -> Vec<isize> {
    pending_clicks()
        .lock()
        .map(|mut queue| std::mem::take(&mut *queue))
        .unwrap_or_default()
}

pub fn register_class() -> Result<(), String> {
    CLASS_REGISTERED
        .get_or_init(|| unsafe { register_class_inner() })
        .clone()
}

unsafe fn register_class_inner() -> Result<(), String> {
    let hinstance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
    let cursor = LoadCursorW(None, IDC_HAND).map_err(|e| e.to_string())?;

    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(button_wnd_proc),
        hInstance: hinstance.into(),
        hCursor: cursor,
        hbrBackground: HBRUSH::default(),
        lpszClassName: w!("AAAiPinAiButton"),
        hIcon: HICON::default(),
        ..Default::default()
    };

    let atom = RegisterClassW(&class);
    if atom == 0 {
        let err = windows::core::Error::from_win32();
        // ERROR_CLASS_ALREADY_EXISTS
        if err.code().0 as u32 == 1410 {
            return Ok(());
        }
        return Err(format!("RegisterClassW failed: {err}"));
    }
    Ok(())
}

pub fn create_button_for_pin(pin_hwnd: isize) -> Result<HWND, String> {
    register_class()?;

    unsafe {
        let hinstance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
            w!("AAAiPinAiButton"),
            w!("AAAi"),
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            BUTTON_WIDTH,
            BUTTON_HEIGHT,
            None,
            None,
            hinstance,
            Some(pin_hwnd as *const std::ffi::c_void),
        )
        .map_err(|e| e.to_string())?;

        if hwnd.0.is_null() {
            return Err("CreateWindowExW returned null".into());
        }

        apply_badge_region(hwnd);
        let _ = ShowWindow(hwnd, SW_HIDE);
        let _ = UpdateWindow(hwnd);
        Ok(hwnd)
    }
}

pub fn destroy_button(hwnd: HWND) {
    unsafe {
        if !hwnd.0.is_null() {
            let key = hwnd.0 as isize;
            if let Ok(mut map) = hover_state().lock() {
                map.remove(&key);
            }
            let _ = ShowWindow(hwnd, SW_HIDE);
            let _ = DestroyWindow(hwnd);
        }
    }
}

pub fn hide_button(hwnd: HWND) {
    unsafe {
        if !hwnd.0.is_null() {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

pub fn show_button(hwnd: HWND) {
    unsafe {
        if !hwnd.0.is_null() {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_NOSENDCHANGING,
            );
            // Ensure latest theme paint after show.
            let _ = InvalidateRect(hwnd, None, true);
        }
    }
}

pub fn position_button(button: HWND, pin_rect: &RECT) {
    let x = pin_rect.right - BUTTON_WIDTH - BUTTON_MARGIN;
    let y = pin_rect.bottom - BUTTON_HEIGHT - BUTTON_MARGIN;
    unsafe {
        let _ = SetWindowPos(
            button,
            HWND_TOPMOST,
            x,
            y,
            BUTTON_WIDTH,
            BUTTON_HEIGHT,
            SWP_NOACTIVATE | SWP_NOSENDCHANGING,
        );
        apply_badge_region(button);
    }
}

pub fn invalidate_button(button: HWND) {
    unsafe {
        if !button.0.is_null() {
            let _ = InvalidateRect(button, None, true);
        }
    }
}

/// Returns true if the cursor is currently over any AAAi pin badge window.
pub fn cursor_over_any_badge(badges: &HashMap<isize, HWND>) -> bool {
    let mut pt = POINT::default();
    if unsafe { GetCursorPos(&mut pt) }.is_err() {
        return false;
    }
    for badge in badges.values() {
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(*badge, &mut rect) }.is_err() {
            continue;
        }
        if pt.x >= rect.left && pt.x < rect.right && pt.y >= rect.top && pt.y < rect.bottom {
            return true;
        }
    }
    false
}

unsafe fn apply_badge_region(hwnd: HWND) {
    let rgn = CreateEllipticRgn(0, 0, BUTTON_WIDTH, BUTTON_HEIGHT);
    let _ = SetWindowRgn(hwnd, rgn, true);
}

fn mix_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8;
    (mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f32 {
    let channel = |c: u8| {
        let n = c as f32 / 255.0;
        if n <= 0.03928 {
            n / 12.92
        } else {
            ((n + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

unsafe extern "system" fn button_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let create = lparam.0 as *const CREATESTRUCTW;
            if create.is_null() {
                return LRESULT(0);
            }
            let pin_hwnd = (*create).lpCreateParams as isize;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, pin_hwnd);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_PAINT => {
            paint_button(hwnd);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let key = hwnd.0 as isize;
            let mut entered = false;
            if let Ok(mut map) = hover_state().lock() {
                entered = !*map.get(&key).unwrap_or(&false);
                map.insert(key, true);
            }
            if entered {
                let _ = InvalidateRect(hwnd, None, true);
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                let _ = TrackMouseEvent(&mut tme);
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            let key = hwnd.0 as isize;
            if let Ok(mut map) = hover_state().lock() {
                map.insert(key, false);
            }
            let _ = InvalidateRect(hwnd, None, true);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let pin_hwnd = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if pin_hwnd != 0 {
                if let Ok(mut queue) = pending_clicks().lock() {
                    queue.push(pin_hwnd);
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let key = hwnd.0 as isize;
            if let Ok(mut map) = hover_state().lock() {
                map.remove(&key);
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn paint_button(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let paint_dc = BeginPaint(hwnd, &mut ps);
    if paint_dc.0.is_null() {
        return;
    }

    let hovered = hover_state()
        .lock()
        .ok()
        .and_then(|map| map.get(&(hwnd.0 as isize)).copied())
        .unwrap_or(false);

    update_layered_badge(hwnd, theme::accent_rgb(), hovered);
    let _ = EndPaint(hwnd, &ps);
}

unsafe fn update_layered_badge(hwnd: HWND, accent: (u8, u8, u8), hovered: bool) {
    let pixels = render_badge_pixels(accent, hovered);
    let screen_dc = GetDC(None);
    if screen_dc.0.is_null() {
        return;
    }

    let memory_dc = CreateCompatibleDC(screen_dc);
    if memory_dc.0.is_null() {
        ReleaseDC(None, screen_dc);
        return;
    }

    let bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: BUTTON_WIDTH,
            biHeight: -BUTTON_HEIGHT,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let bitmap = match CreateDIBSection(screen_dc, &bitmap_info, DIB_RGB_COLORS, &mut bits, None, 0)
    {
        Ok(bitmap) => bitmap,
        Err(_) => {
            let _ = DeleteDC(memory_dc);
            ReleaseDC(None, screen_dc);
            return;
        }
    };

    std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len());
    let old_bitmap = SelectObject(memory_dc, HGDIOBJ(bitmap.0));

    let mut window_rect = RECT::default();
    if GetWindowRect(hwnd, &mut window_rect).is_ok() {
        let destination = POINT {
            x: window_rect.left,
            y: window_rect.top,
        };
        let source = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: BUTTON_WIDTH,
            cy: BUTTON_HEIGHT,
        };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let _ = UpdateLayeredWindow(
            hwnd,
            screen_dc,
            Some(&destination),
            Some(&size),
            memory_dc,
            Some(&source),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );
    }

    SelectObject(memory_dc, old_bitmap);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(memory_dc);
    ReleaseDC(None, screen_dc);
}

fn render_badge_pixels(accent: (u8, u8, u8), hovered: bool) -> Vec<u8> {
    let mut pixels = vec![0u8; (BUTTON_WIDTH * BUTTON_HEIGHT * 4) as usize];
    let base = if hovered {
        mix_rgb(accent, (255, 255, 255), 0.11)
    } else {
        accent
    };
    let star = if relative_luminance(base.0, base.1, base.2) > 0.62 {
        (24, 27, 31)
    } else {
        (255, 255, 255)
    };
    let radius = MAIN_RADIUS + if hovered { 0.45 } else { 0.0 };
    let star_points = four_point_star_points(
        MAIN_CENTER_X,
        MAIN_CENTER_Y,
        STAR_OUTER_RADIUS,
        STAR_INNER_RADIUS,
    );
    let sample_count = (SUPERSAMPLE * SUPERSAMPLE) as f32;

    for py in 0..BUTTON_HEIGHT as usize {
        for px in 0..BUTTON_WIDTH as usize {
            let mut sum = [0.0f32; 4];
            for sy in 0..SUPERSAMPLE {
                for sx in 0..SUPERSAMPLE {
                    let x = px as f32 + (sx as f32 + 0.5) / SUPERSAMPLE as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / SUPERSAMPLE as f32;
                    let mut sample = [0.0f32; 4];

                    let shadow_distance =
                        ((x - MAIN_CENTER_X).powi(2) + (y - SHADOW_CENTER_Y).powi(2)).sqrt();
                    let shadow_alpha = ((SHADOW_OUTER_RADIUS - shadow_distance)
                        / (SHADOW_OUTER_RADIUS - SHADOW_INNER_RADIUS))
                        .clamp(0.0, 1.0)
                        * if hovered { 0.22 } else { 0.28 };
                    composite(&mut sample, (10, 14, 20), shadow_alpha);

                    let main_distance =
                        ((x - MAIN_CENTER_X).powi(2) + (y - MAIN_CENTER_Y).powi(2)).sqrt();
                    if main_distance <= radius {
                        let vertical =
                            ((y - (MAIN_CENTER_Y - radius)) / (radius * 2.0)).clamp(0.0, 1.0);
                        let surface = if vertical < 0.5 {
                            mix_rgb(base, (255, 255, 255), (0.5 - vertical) * 0.16)
                        } else {
                            mix_rgb(base, (0, 0, 0), (vertical - 0.5) * 0.12)
                        };
                        composite(&mut sample, surface, 1.0);

                        if point_in_polygon(x, y, &star_points) {
                            composite(&mut sample, star, 1.0);
                        }
                    }

                    for channel in 0..4 {
                        sum[channel] += sample[channel];
                    }
                }
            }

            let offset = (py * BUTTON_WIDTH as usize + px) * 4;
            let red = (sum[0] / sample_count * 255.0).round() as u8;
            let green = (sum[1] / sample_count * 255.0).round() as u8;
            let blue = (sum[2] / sample_count * 255.0).round() as u8;
            let alpha = (sum[3] / sample_count * 255.0).round() as u8;
            pixels[offset] = blue;
            pixels[offset + 1] = green;
            pixels[offset + 2] = red;
            pixels[offset + 3] = alpha;
        }
    }

    pixels
}

fn composite(target: &mut [f32; 4], color: (u8, u8, u8), alpha: f32) {
    let inverse = 1.0 - alpha;
    target[0] = color.0 as f32 / 255.0 * alpha + target[0] * inverse;
    target[1] = color.1 as f32 / 255.0 * alpha + target[1] * inverse;
    target[2] = color.2 as f32 / 255.0 * alpha + target[2] * inverse;
    target[3] = alpha + target[3] * inverse;
}

fn point_in_polygon(x: f32, y: f32, points: &[(f32, f32); 8]) -> bool {
    let mut inside = false;
    let mut previous = points.len() - 1;
    for current in 0..points.len() {
        let (x1, y1) = points[current];
        let (x2, y2) = points[previous];
        if (y1 > y) != (y2 > y) && x < (x2 - x1) * (y - y1) / (y2 - y1) + x1 {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

/// Cardinal four-point star (N/E/S/W tips) with concave sides — AAAi sparkle mark.
fn four_point_star_points(cx: f32, cy: f32, outer_r: f32, inner_r: f32) -> [(f32, f32); 8] {
    let mut points = [(0.0, 0.0); 8];
    for (i, point) in points.iter_mut().enumerate() {
        let angle = -PI as f32 / 2.0 + i as f32 * (PI as f32 / 4.0);
        let r = if i % 2 == 0 { outer_r } else { inner_r };
        *point = (cx + r * angle.cos(), cy + r * angle.sin());
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_badge_has_clean_antialiased_transparency() {
        let pixels = render_badge_pixels((59, 142, 234), false);
        assert_eq!(pixels.len(), (BUTTON_WIDTH * BUTTON_HEIGHT * 4) as usize);

        assert_eq!(&pixels[0..4], &[0, 0, 0, 0]);
        assert!(pixels.chunks_exact(4).any(|pixel| {
            let alpha = pixel[3];
            alpha > 0 && alpha < 255
        }));
        assert!(pixels
            .chunks_exact(4)
            .all(|pixel| pixel[0] <= pixel[3] && pixel[1] <= pixel[3] && pixel[2] <= pixel[3]));
    }

    #[test]
    fn original_star_remains_centered_and_high_contrast() {
        let pixels = render_badge_pixels((59, 142, 234), false);
        let center = (MAIN_CENTER_Y as usize * BUTTON_WIDTH as usize + MAIN_CENTER_X as usize) * 4;
        let pixel = &pixels[center..center + 4];

        assert_eq!(pixel[3], 255);
        assert!(pixel[0] > 240 && pixel[1] > 240 && pixel[2] > 240);
    }
}
