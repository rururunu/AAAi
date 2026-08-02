#[cfg(windows)]
mod imp {
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, OnceLock};

    use tauri::WebviewWindow;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, IsIconic, IsWindowVisible, SetWindowLongPtrW, SetWindowPos, ShowWindow,
        GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE, SW_MINIMIZE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    };

    static OVERLAY_NATIVE_MINIMIZED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    static OVERLAY_MINIMIZE_PENDING: AtomicBool = AtomicBool::new(false);

    fn minimized_labels() -> std::sync::MutexGuard<'static, HashSet<String>> {
        OVERLAY_NATIVE_MINIMIZED
            .get_or_init(|| Mutex::new(HashSet::new()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn mark_overlay_native_minimized(label: &str) {
        minimized_labels().insert(label.to_string());
    }

    pub fn is_overlay_native_minimized(label: &str) -> bool {
        minimized_labels().contains(label)
    }

    pub fn clear_overlay_native_minimized(label: &str) {
        minimized_labels().remove(label);
    }

    pub fn mark_minimize_pending() {
        OVERLAY_MINIMIZE_PENDING.store(true, Ordering::Release);
    }

    pub fn clear_minimize_pending() {
        OVERLAY_MINIMIZE_PENDING.store(false, Ordering::Release);
    }

    pub fn is_minimize_pending() -> bool {
        OVERLAY_MINIMIZE_PENDING.load(Ordering::Acquire)
    }

    fn local_hwnd(window: &WebviewWindow) -> Result<HWND, String> {
        let raw = window.hwnd().map_err(|error| error.to_string())?.0;
        Ok(HWND(raw))
    }

    pub fn minimize_window(window: &WebviewWindow) -> Result<(), String> {
        let hwnd = local_hwnd(window)?;
        unsafe { minimize_hwnd(hwnd) }
    }

    pub fn reapply_toolwindow_style(window: &WebviewWindow) {
        let Ok(hwnd) = local_hwnd(window) else {
            return;
        };
        unsafe {
            apply_toolwindow_style(hwnd);
        }
    }

    unsafe fn minimize_hwnd(hwnd: HWND) -> Result<(), String> {
        if IsIconic(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
            return Ok(());
        }

        // Tool windows minimize to nowhere; register as a normal app window first.
        apply_appwindow_style(hwnd);
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
        Ok(())
    }

    unsafe fn apply_appwindow_style(hwnd: HWND) {
        let _ = SetWindowPos(
            hwnd,
            HWND_NOTOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let next_style = (ex_style | WS_EX_APPWINDOW.0) & !WS_EX_TOOLWINDOW.0;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_style as isize);

        // Force shell to pick up taskbar eligibility before minimizing.
        let _ = SetWindowPos(
            hwnd,
            HWND_NOTOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }

    unsafe fn apply_toolwindow_style(hwnd: HWND) {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let next_style = (ex_style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_style as isize);
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

#[cfg(windows)]
pub use imp::*;

#[cfg(not(windows))]
mod imp {
    use tauri::WebviewWindow;

    pub fn mark_overlay_native_minimized(_label: &str) {}
    pub fn is_overlay_native_minimized(_label: &str) -> bool {
        false
    }
    pub fn clear_overlay_native_minimized(_label: &str) {}
    pub fn mark_minimize_pending() {}
    pub fn clear_minimize_pending() {}
    pub fn is_minimize_pending() -> bool {
        false
    }
    pub fn reapply_toolwindow_style(_window: &WebviewWindow) {}

    pub fn minimize_window(window: &WebviewWindow) -> Result<(), String> {
        window.minimize().map_err(|error| error.to_string())
    }
}

#[cfg(not(windows))]
pub use imp::*;
