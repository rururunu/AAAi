use std::mem::size_of;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND, LPARAM, WPARAM};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
    IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Threading::AttachThreadInput;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTextPattern, UIA_TextPatternId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_INSERT, VK_LCONTROL,
    VK_LMENU, VK_LSHIFT, VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
    SendMessageW, SetForegroundWindow, GUITHREADINFO, WM_COPY,
};

use crate::core::context::image_capture::read_clipboard_image_data_url;
use crate::core::context::models::{CaptureError, CaptureSource, WindowInfo};
use crate::core::context::provider::{CaptureProvider, CaptureResult, PartialCapture};

const CF_UNICODETEXT: u32 = 13;
const CAPTURE_TIMEOUT: Duration = Duration::from_millis(140);
const POLL_INTERVAL: Duration = Duration::from_millis(6);
const FOCUS_SETTLE: Duration = Duration::from_millis(55);
const KEY_SETTLE: Duration = Duration::from_millis(22);
const CLIPBOARD_OPEN_RETRIES: usize = 8;
const UI_AUTOMATION_TIMEOUT: Duration = Duration::from_millis(100);
const UI_AUTOMATION_PARENT_LIMIT: usize = 6;

pub struct ClipboardProvider;

impl ClipboardProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClipboardProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureProvider for ClipboardProvider {
    fn capture(&self, window: &WindowInfo) -> CaptureResult {
        if should_try_ui_automation(window) {
            if let Some(text) = capture_selected_text_via_ui_automation() {
                return CaptureResult::Success(PartialCapture {
                    selected_text: Some(text),
                    selected_files: Vec::new(),
                    selected_images: Vec::new(),
                    source: CaptureSource::UiAutomation,
                });
            }
        }

        match capture_selected_content(window) {
            Ok(content) if content.has_content() => CaptureResult::Success(content.into_partial()),
            Ok(_) => CaptureResult::Empty,
            Err(error) => {
                tracing::warn!(provider = "clipboard", error = %error, "context provider failed");
                CaptureResult::Empty
            }
        }
    }
}

fn should_try_ui_automation(window: &WindowInfo) -> bool {
    // Monaco-based Electron editors do not expose their selection through
    // UI Automation. Going directly to the clipboard path saves a guaranteed
    // timeout on the primary Alt+Alt workflow.
    !matches!(
        window.process_name.to_ascii_lowercase().as_str(),
        "code.exe" | "code - insiders.exe" | "cursor.exe" | "vscodium.exe" | "windsurf.exe"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedClipboardContent {
    selected_text: Option<String>,
    selected_images: Vec<String>,
    source: CaptureSource,
}

impl CapturedClipboardContent {
    fn has_content(&self) -> bool {
        self.selected_text
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty())
            || !self.selected_images.is_empty()
    }

    fn into_partial(self) -> PartialCapture {
        PartialCapture {
            selected_text: self.selected_text,
            selected_files: Vec::new(),
            selected_images: self.selected_images,
            source: self.source,
        }
    }
}

fn capture_selected_text_via_ui_automation() -> Option<String> {
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let _ = tx.send(capture_selected_text_via_ui_automation_inner());
    });
    rx.recv_timeout(UI_AUTOMATION_TIMEOUT).ok().flatten()
}

fn capture_selected_text_via_ui_automation_inner() -> Option<String> {
    unsafe {
        let com_initialized = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        let result = read_ui_automation_selection();
        if com_initialized {
            CoUninitialize();
        }
        result
    }
}

unsafe fn read_ui_automation_selection() -> Option<String> {
    let automation: IUIAutomation =
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
    let walker = automation.ControlViewWalker().ok()?;
    let mut element = Some(automation.GetFocusedElement().ok()?);

    for _ in 0..UI_AUTOMATION_PARENT_LIMIT {
        let current = element?;
        if let Some(text) = read_text_pattern_selection(&current) {
            return Some(text);
        }
        element = walker.GetParentElement(&current).ok();
    }

    None
}

unsafe fn read_text_pattern_selection(element: &IUIAutomationElement) -> Option<String> {
    let pattern: IUIAutomationTextPattern = element.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
    let ranges = pattern.GetSelection().ok()?;
    let length = ranges.Length().ok()?;
    let mut selected = String::new();

    for index in 0..length {
        let text = ranges.GetElement(index).ok()?.GetText(-1).ok()?.to_string();
        if !text.trim().is_empty() {
            if !selected.is_empty() {
                selected.push('\n');
            }
            selected.push_str(&text);
        }
    }

    (!selected.trim().is_empty()).then_some(selected)
}

fn capture_selected_content(window: &WindowInfo) -> Result<CapturedClipboardContent, CaptureError> {
    let backup = read_clipboard_text()?;
    let start_seq = unsafe { GetClipboardSequenceNumber() };

    with_target_focus(window, |target, target_thread| {
        let focus = unsafe { focused_control(target, target_thread) };
        copy_via_wm_message(focus)?;
        if !clipboard_changed(start_seq) && focus != target {
            copy_via_wm_message(target)?;
        }
        if !clipboard_changed(start_seq) {
            let _ = simulate_copy_ctrl_insert();
            thread::sleep(KEY_SETTLE);
        }
        Ok(())
    })?;

    wait_for_clipboard_update(start_seq);

    let end_seq = unsafe { GetClipboardSequenceNumber() };
    let captured_image = read_clipboard_image_data_url();
    let captured = read_clipboard_text()?;
    let backup_for_compare = backup.clone();
    restore_clipboard_text(backup)?;

    let selected_text = select_captured_text(backup_for_compare, captured, start_seq != end_seq);
    Ok(CapturedClipboardContent {
        selected_text,
        selected_images: captured_image.into_iter().collect(),
        source: CaptureSource::Clipboard,
    })
}

fn select_captured_text(
    backup: Option<String>,
    captured: Option<String>,
    sequence_changed: bool,
) -> Option<String> {
    let captured = captured.filter(|text| !text.trim().is_empty())?;

    if sequence_changed {
        return Some(captured);
    }

    if backup.as_ref() != Some(&captured) {
        return Some(captured);
    }

    None
}

fn simulate_copy_ctrl_insert() -> Result<(), CaptureError> {
    simulate_key_combo(VK_CONTROL, VK_INSERT, KEYEVENTF_EXTENDEDKEY)
}

fn simulate_key_combo(
    modifier: VIRTUAL_KEY,
    key: VIRTUAL_KEY,
    key_flags: KEYBD_EVENT_FLAGS,
) -> Result<(), CaptureError> {
    unsafe {
        let inputs = [
            key_event(modifier, KEYBD_EVENT_FLAGS(0)),
            key_event(key, key_flags),
            key_event(key, key_flags | KEYEVENTF_KEYUP),
            key_event(modifier, KEYEVENTF_KEYUP),
        ];
        let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            return Err(CaptureError::Clipboard(
                "SendInput failed to send all keys".into(),
            ));
        }
    }
    Ok(())
}

fn with_target_focus<F>(window: &WindowInfo, action: F) -> Result<(), CaptureError>
where
    F: FnOnce(HWND, u32) -> Result<(), CaptureError>,
{
    unsafe {
        let target = HWND(window.hwnd as *mut _);
        let foreground = GetForegroundWindow();

        let mut foreground_thread = 0u32;
        let mut target_thread = 0u32;
        GetWindowThreadProcessId(foreground, Some(&mut foreground_thread));
        GetWindowThreadProcessId(target, Some(&mut target_thread));

        if target_thread == 0 {
            return Err(CaptureError::Clipboard(
                "failed to resolve target thread".into(),
            ));
        }

        let attached = foreground_thread != 0 && foreground_thread != target_thread;
        if attached {
            let _ = AttachThreadInput(foreground_thread, target_thread, true);
        }

        let _ = SetForegroundWindow(target);
        let _ = BringWindowToTop(target);
        force_release_modifiers_for_capture();
        thread::sleep(FOCUS_SETTLE);

        let result = action(target, target_thread);

        if attached {
            let _ = AttachThreadInput(foreground_thread, target_thread, false);
        }

        result
    }
}

unsafe fn focused_control(target: HWND, target_thread: u32) -> HWND {
    let mut gui = GUITHREADINFO {
        cbSize: size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };

    if GetGUIThreadInfo(target_thread, &mut gui).is_ok() && !gui.hwndFocus.0.is_null() {
        return gui.hwndFocus;
    }

    if !gui.hwndActive.0.is_null() {
        return gui.hwndActive;
    }

    target
}

fn copy_via_wm_message(hwnd: HWND) -> Result<(), CaptureError> {
    unsafe {
        SendMessageW(hwnd, WM_COPY, WPARAM(0), LPARAM(0));
    }
    Ok(())
}

/// Force-clear Alt/Ctrl/Shift so double-Alt does not turn Ctrl+Insert into Alt+Ctrl+Insert.
/// Safe to call before clipboard capture from the hotkey path.
pub fn force_release_modifiers_for_capture() {
    for _ in 0..3 {
        release_modifier_keys();
        if !any_modifier_physically_down() {
            break;
        }
        thread::sleep(Duration::from_millis(12));
    }
    thread::sleep(Duration::from_millis(16));
}

fn any_modifier_physically_down() -> bool {
    let keys = [
        VK_MENU,
        VK_LMENU,
        VK_RMENU,
        VK_CONTROL,
        VK_LCONTROL,
        VK_RCONTROL,
        VK_SHIFT,
        VK_LSHIFT,
        VK_RSHIFT,
    ];
    keys.iter()
        .any(|key| unsafe { GetAsyncKeyState(key.0 as i32) } as u16 & 0x8000 != 0)
}

fn release_modifier_keys() {
    // Right Alt / Ctrl / Shift are extended keys on the numeric / right side.
    let modifiers = [
        (VK_MENU, KEYBD_EVENT_FLAGS(0)),
        (VK_LMENU, KEYBD_EVENT_FLAGS(0)),
        (VK_RMENU, KEYEVENTF_EXTENDEDKEY),
        (VK_CONTROL, KEYBD_EVENT_FLAGS(0)),
        (VK_LCONTROL, KEYBD_EVENT_FLAGS(0)),
        (VK_RCONTROL, KEYEVENTF_EXTENDEDKEY),
        (VK_SHIFT, KEYBD_EVENT_FLAGS(0)),
        (VK_LSHIFT, KEYBD_EVENT_FLAGS(0)),
        (VK_RSHIFT, KEYEVENTF_EXTENDEDKEY),
    ];

    for (key, flags) in modifiers {
        let input = [key_event(key, flags | KEYEVENTF_KEYUP)];
        let _ = unsafe { SendInput(&input, size_of::<INPUT>() as i32) };
    }
}

fn clipboard_changed(start_seq: u32) -> bool {
    unsafe { GetClipboardSequenceNumber() != start_seq }
}

fn key_event(key: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn wait_for_clipboard_update(start_seq: u32) {
    let deadline = Instant::now() + CAPTURE_TIMEOUT;
    while Instant::now() < deadline {
        if unsafe { GetClipboardSequenceNumber() } != start_seq {
            return;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn read_clipboard_text() -> Result<Option<String>, CaptureError> {
    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
            return Ok(None);
        }

        open_clipboard_with_retry()?;

        let result = (|| {
            let handle = GetClipboardData(CF_UNICODETEXT).map_err(|error| {
                CaptureError::Clipboard(format!("GetClipboardData failed: {error}"))
            })?;
            if handle.0.is_null() {
                return Ok(None);
            }

            let global = HGLOBAL(handle.0);
            let ptr = GlobalLock(global);
            if ptr.is_null() {
                return Ok(None);
            }

            let text = read_wide_string(ptr as *const u16);
            let _ = GlobalUnlock(global);
            Ok(Some(text))
        })();

        let _ = CloseClipboard();
        result
    }
}

fn open_clipboard_with_retry() -> Result<(), CaptureError> {
    for attempt in 0..CLIPBOARD_OPEN_RETRIES {
        if unsafe { OpenClipboard(HWND::default()) }.is_ok() {
            return Ok(());
        }
        if attempt + 1 < CLIPBOARD_OPEN_RETRIES {
            thread::sleep(Duration::from_millis(10));
        }
    }

    Err(CaptureError::Clipboard(
        "OpenClipboard failed after retries".into(),
    ))
}

unsafe fn read_wide_string(ptr: *const u16) -> String {
    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}

fn restore_clipboard_text(backup: Option<String>) -> Result<(), CaptureError> {
    unsafe {
        open_clipboard_with_retry()?;

        let result = (|| {
            EmptyClipboard().map_err(|error| {
                CaptureError::Clipboard(format!("EmptyClipboard failed: {error}"))
            })?;

            if let Some(text) = backup {
                write_clipboard_text(&text)?;
            }

            Ok(())
        })();

        let _ = CloseClipboard();
        result
    }
}

fn write_clipboard_text(text: &str) -> Result<(), CaptureError> {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_len = wide.len() * 2;

    unsafe {
        let global = GlobalAlloc(GMEM_MOVEABLE, byte_len)
            .map_err(|error| CaptureError::Clipboard(format!("GlobalAlloc failed: {error}")))?;
        if global.0.is_null() {
            return Err(CaptureError::Clipboard("GlobalAlloc returned null".into()));
        }

        let ptr = GlobalLock(global);
        if ptr.is_null() {
            return Err(CaptureError::Clipboard("GlobalLock returned null".into()));
        }

        std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, byte_len);
        let _ = GlobalUnlock(global);

        SetClipboardData(CF_UNICODETEXT, HANDLE(global.0)).map_err(|error| {
            CaptureError::Clipboard(format!("SetClipboardData failed: {error}"))
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_captured_text_when_sequence_changes() {
        let selected = select_captured_text(None, Some("hello".into()), true);
        assert_eq!(selected, Some("hello".into()));
    }

    #[test]
    fn accepts_captured_text_when_it_differs_from_backup() {
        let selected = select_captured_text(Some("old".into()), Some("new".into()), false);
        assert_eq!(selected, Some("new".into()));
    }

    #[test]
    fn rejects_unchanged_clipboard_without_sequence_change() {
        let selected = select_captured_text(Some("same".into()), Some("same".into()), false);
        assert_eq!(selected, None);
    }

    #[test]
    fn skips_slow_ui_automation_for_monaco_editors() {
        let vscode = WindowInfo {
            hwnd: 0,
            pid: 1,
            process_name: "Code.exe".into(),
            title: "editor".into(),
        };
        let notepad = WindowInfo {
            process_name: "notepad.exe".into(),
            ..vscode.clone()
        };
        assert!(!should_try_ui_automation(&vscode));
        assert!(should_try_ui_automation(&notepad));
    }
}
