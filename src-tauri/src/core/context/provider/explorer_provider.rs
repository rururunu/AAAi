use std::path::PathBuf;

use windows::core::{Interface, VARIANT};
use windows::Win32::Foundation::{FALSE, HWND};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_LOCAL_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    IFolderView, IFolderView2, IShellBrowser, IShellItemArray, IShellView, IShellWindows,
    IWebBrowserApp, SID_STopLevelBrowser, ShellWindows, SIGDN_FILESYSPATH, SVGIO_SELECTION,
};

use crate::core::context::image_capture::partition_selected_files;
use crate::core::context::models::{CaptureError, CaptureSource, WindowInfo};
use crate::core::context::provider::{CaptureProvider, CaptureResult, PartialCapture};

pub struct ExplorerProvider;

impl ExplorerProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExplorerProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureProvider for ExplorerProvider {
    fn capture(&self, window: &WindowInfo) -> CaptureResult {
        match capture_selected_files(HWND(window.hwnd as *mut _)) {
            Ok(files) if !files.is_empty() => {
                let (selected_files, selected_images) = partition_selected_files(files);
                if selected_files.is_empty() && selected_images.is_empty() {
                    CaptureResult::Empty
                } else {
                    CaptureResult::Success(PartialCapture {
                        selected_text: None,
                        selected_files,
                        selected_images,
                        source: CaptureSource::Explorer,
                    })
                }
            }
            Ok(_) => CaptureResult::Empty,
            Err(error) => {
                tracing::warn!(provider = "explorer", error = %error, "context provider failed");
                CaptureResult::Empty
            }
        }
    }
}

fn capture_selected_files(target_hwnd: HWND) -> Result<Vec<PathBuf>, CaptureError> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let com_initialized = hr.is_ok() || hr == windows::core::HRESULT(0x80010106u32 as i32);
        if !com_initialized {
            return Err(CaptureError::ComInitFailed(format!("{hr:?}")));
        }

        let result = capture_selected_files_inner(target_hwnd);

        if hr.is_ok() {
            CoUninitialize();
        }

        result
    }
}

unsafe fn capture_selected_files_inner(target_hwnd: HWND) -> Result<Vec<PathBuf>, CaptureError> {
    let shell_windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)
        .map_err(|error| {
        CaptureError::ExplorerFailed(format!("CoCreateInstance ShellWindows failed: {error}"))
    })?;

    let count = shell_windows
        .Count()
        .map_err(|error| CaptureError::ExplorerFailed(format!("Count failed: {error}")))?;

    for index in 0..count {
        let item = shell_windows
            .Item(&VARIANT::from(index))
            .map_err(|error| CaptureError::ExplorerFailed(format!("Item failed: {error}")))?;

        let browser: IWebBrowserApp = item.cast().map_err(|error| {
            CaptureError::ExplorerFailed(format!("cast browser failed: {error}"))
        })?;

        let hwnd_ptr = browser
            .HWND()
            .map_err(|error| CaptureError::ExplorerFailed(format!("HWND failed: {error}")))?;
        let browser_hwnd = HWND(hwnd_ptr.0 as *mut _);

        if !hwnd_matches_explorer(browser_hwnd, target_hwnd) {
            continue;
        }

        let service_provider = browser
            .cast::<windows::Win32::System::Com::IServiceProvider>()
            .map_err(|error| {
                CaptureError::ExplorerFailed(format!("cast service provider failed: {error}"))
            })?;

        let shell_browser: IShellBrowser = service_provider
            .QueryService(&SID_STopLevelBrowser)
            .map_err(|error| {
                CaptureError::ExplorerFailed(format!("QueryService failed: {error}"))
            })?;

        let shell_view: IShellView = shell_browser.QueryActiveShellView().map_err(|error| {
            CaptureError::ExplorerFailed(format!("QueryActiveShellView failed: {error}"))
        })?;

        let folder_view: IFolderView = shell_view.cast().map_err(|error| {
            CaptureError::ExplorerFailed(format!("cast folder view failed: {error}"))
        })?;

        return read_selected_paths(&folder_view);
    }

    Ok(Vec::new())
}

/// Foreground HWND may be the Shell view child, not the top-level Explorer frame.
unsafe fn hwnd_matches_explorer(browser_hwnd: HWND, target_hwnd: HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, IsChild, GA_ROOT};

    if browser_hwnd == target_hwnd {
        return true;
    }
    if IsChild(browser_hwnd, target_hwnd).as_bool() {
        return true;
    }
    let target_root = GetAncestor(target_hwnd, GA_ROOT);
    !target_root.0.is_null() && target_root == browser_hwnd
}

/// `IFolderView::Item(i)` is the *folder view* index, not the selection ordinal.
/// Using `Item(0..selectionCount)` returns the first N files in the view — wrong paths.
/// Always resolve selection via `GetSelection` / `Items(SVGIO_SELECTION)`.
unsafe fn read_selected_paths(folder_view: &IFolderView) -> Result<Vec<PathBuf>, CaptureError> {
    if let Ok(view2) = folder_view.cast::<IFolderView2>() {
        if let Ok(items) = view2.GetSelection(FALSE) {
            return shell_item_array_paths(&items);
        }
    }

    if let Ok(items) = folder_view.Items::<IShellItemArray>(SVGIO_SELECTION) {
        return shell_item_array_paths(&items);
    }

    Ok(Vec::new())
}

unsafe fn shell_item_array_paths(items: &IShellItemArray) -> Result<Vec<PathBuf>, CaptureError> {
    let count = items
        .GetCount()
        .map_err(|error| CaptureError::ExplorerFailed(format!("GetCount failed: {error}")))?;

    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        let item = items.GetItemAt(index).map_err(|error| {
            CaptureError::ExplorerFailed(format!("GetItemAt({index}) failed: {error}"))
        })?;
        let name = match item.GetDisplayName(SIGDN_FILESYSPATH) {
            Ok(name) => name,
            Err(_) => continue,
        };
        let path = name.to_string().unwrap_or_default();
        CoTaskMemFree(Some(name.0 as *const _));
        if !path.is_empty() {
            paths.push(PathBuf::from(path));
        }
    }

    Ok(paths)
}
