//! Detect PixPin / Snipaste pin windows.

use std::collections::HashSet;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowLongW, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, GWL_EXSTYLE, GWL_STYLE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinHost {
    PixPin,
    Snipaste,
}

#[derive(Debug, Clone)]
pub struct PinWindow {
    pub hwnd: isize,
    pub host: PinHost,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

const MIN_PIN_SIZE: i32 = 48;
const MAX_PIN_EDGE: i32 = 8_000;

struct EnumState {
    pins: Vec<PinWindow>,
    seen: HashSet<isize>,
}

pub fn enumerate_pin_windows() -> Vec<PinWindow> {
    let mut state = EnumState {
        pins: Vec::new(),
        seen: HashSet::new(),
    };

    unsafe {
        let _ = EnumWindows(
            Some(enum_proc),
            LPARAM(&mut state as *mut EnumState as isize),
        );
    }

    state.pins
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state = &mut *(lparam.0 as *mut EnumState);

    if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
        return BOOL(1);
    }

    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect).is_err() {
        return BOOL(1);
    }

    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width < MIN_PIN_SIZE
        || height < MIN_PIN_SIZE
        || width > MAX_PIN_EDGE
        || height > MAX_PIN_EDGE
    {
        return BOOL(1);
    }

    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return BOOL(1);
    }

    let process_name = match read_process_name(pid) {
        Some(name) => name,
        None => return BOOL(1),
    };
    let process_lower = process_name.to_ascii_lowercase();

    let host = if process_lower == "pixpin.exe" {
        PinHost::PixPin
    } else if process_lower == "snipaste.exe" {
        PinHost::Snipaste
    } else {
        return BOOL(1);
    };

    let class_name = read_class_name(hwnd);
    let title = read_window_title(hwnd);
    let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

    if !is_pin_window(host, &class_name, &title, style, ex_style, width, height) {
        return BOOL(1);
    }

    let key = hwnd.0 as isize;
    if !state.seen.insert(key) {
        return BOOL(1);
    }

    state.pins.push(PinWindow {
        hwnd: key,
        host,
        x: rect.left,
        y: rect.top,
        width,
        height,
    });

    BOOL(1)
}

fn is_pin_window(
    host: PinHost,
    class_name: &str,
    title: &str,
    style: u32,
    ex_style: u32,
    width: i32,
    height: i32,
) -> bool {
    // Skip our own AI buttons if they ever match a process name (they don't).
    if class_name == button_class_name() {
        return false;
    }

    match host {
        PinHost::PixPin => is_pixpin_pin(class_name, title, style, ex_style, width, height),
        PinHost::Snipaste => is_snipaste_pin(class_name, title, style, ex_style, width, height),
    }
}

fn is_pixpin_pin(
    class_name: &str,
    title: &str,
    _style: u32,
    ex_style: u32,
    width: i32,
    height: i32,
) -> bool {
    // Qt top-level windows (versioned class like Qt51513QWindowIcon).
    let is_qt = class_name.contains("QWindow");
    if !is_qt {
        return false;
    }

    // Tiny tray / helper windows.
    if width <= 8 || height <= 8 {
        return false;
    }

    let title_lower = title.to_ascii_lowercase();
    // Main config / about windows usually have a non-empty product title and are not topmost pins.
    if title_lower == "pixpin"
        || title_lower.contains("配置")
        || title_lower.contains("settings")
        || title_lower.contains("preference")
    {
        // Config dialogs are typically larger and not tool windows.
        let topmost = ex_style & WS_EX_TOPMOST.0 != 0;
        let tool = ex_style & WS_EX_TOOLWINDOW.0 != 0;
        if !topmost && !tool {
            return false;
        }
        // A topmost "PixPin" title with large size is still likely the main UI.
        if width > 400 && height > 300 && !tool {
            return false;
        }
    }

    // Prefer topmost pin-like windows; also accept popup tool windows.
    let topmost = ex_style & WS_EX_TOPMOST.0 != 0;
    let tool = ex_style & WS_EX_TOOLWINDOW.0 != 0;
    topmost || tool || title.is_empty()
}

fn is_snipaste_pin(
    class_name: &str,
    title: &str,
    style: u32,
    ex_style: u32,
    width: i32,
    height: i32,
) -> bool {
    if width <= 8 || height <= 8 {
        return false;
    }

    let title_lower = title.to_ascii_lowercase();
    if title_lower.contains("snipaste")
        && (title_lower.contains("preference")
            || title_lower.contains("设置")
            || title_lower.contains("option")
            || title_lower.contains("about"))
    {
        return false;
    }

    // Snipaste pin windows are usually frameless popup + topmost.
    let popup = style & WS_POPUP.0 != 0;
    let visible = style & WS_VISIBLE.0 != 0;
    let topmost = ex_style & WS_EX_TOPMOST.0 != 0;
    let tool = ex_style & WS_EX_TOOLWINDOW.0 != 0;

    if !visible {
        return false;
    }

    // Accept common Snipaste pin signatures.
    if topmost || tool || popup {
        return true;
    }

    // Fallback: empty-title floating windows under Snipaste process.
    title.is_empty() || class_name.to_ascii_lowercase().contains("snip")
}

fn button_class_name() -> &'static str {
    "AAAiPinAiButton"
}

unsafe fn read_process_name(pid: u32) -> Option<String> {
    let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;
    let mut buffer = [0u16; 260];
    let len = GetModuleBaseNameW(process, None, &mut buffer);
    if len == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..len as usize]))
}

unsafe fn read_class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let len = GetClassNameW(hwnd, &mut buffer);
    if len == 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..len as usize])
}

unsafe fn read_window_title(hwnd: HWND) -> String {
    let mut buffer = [0u16; 512];
    let len = GetWindowTextW(hwnd, &mut buffer);
    if len == 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..len as usize])
}
