use windows::Win32::Foundation::HWND;
use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

use crate::core::context::models::{CaptureError, WindowInfo};

pub struct WindowDetector;

impl WindowDetector {
    pub fn detect() -> Result<WindowInfo, CaptureError> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return Err(CaptureError::WindowDetectionFailed(
                    "no foreground window".into(),
                ));
            }

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return Err(CaptureError::WindowDetectionFailed(
                    "failed to read process id".into(),
                ));
            }

            let process_name = read_process_name(pid)?;
            let title = read_window_title(hwnd)?;

            Ok(WindowInfo {
                hwnd: hwnd.0 as isize,
                pid,
                process_name,
                title,
            })
        }
    }
}

unsafe fn read_process_name(pid: u32) -> Result<String, CaptureError> {
    let process =
        OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).map_err(|error| {
            CaptureError::WindowDetectionFailed(format!("OpenProcess failed: {error}"))
        })?;

    let mut buffer = [0u16; 260];
    let len = GetModuleBaseNameW(process, None, &mut buffer);
    if len == 0 {
        return Err(CaptureError::WindowDetectionFailed(
            "GetModuleBaseNameW returned zero".into(),
        ));
    }

    Ok(String::from_utf16_lossy(&buffer[..len as usize]))
}

unsafe fn read_window_title(hwnd: HWND) -> Result<String, CaptureError> {
    let mut buffer = [0u16; 512];
    let len = GetWindowTextW(hwnd, &mut buffer);
    if len == 0 {
        return Ok(String::new());
    }

    Ok(String::from_utf16_lossy(&buffer[..len as usize]))
}
