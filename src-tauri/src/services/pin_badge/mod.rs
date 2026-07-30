//! Overlay an AI badge on PixPin / Snipaste pin windows.
//!
//! AAAi does not install plugins into those apps. It only detects their pin
//! windows and draws a small "AI" badge at the bottom-right corner. The badge
//! hides while the pin is selected or moved, then reappears after the pin is
//! idle again.

mod button;
mod capture;
mod detect;
mod theme;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tauri::AppHandle;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetCursorPos, IsWindow, PeekMessageW, TranslateMessage, WindowFromPoint, MSG,
    PM_REMOVE,
};

use crate::models::settings::AppSettings;
use crate::services::window::open_overlay_with_images;

use self::button::{
    create_button_for_pin, cursor_over_any_badge, destroy_button, hide_button, invalidate_button,
    position_button, process_pending_clicks, show_button,
};
use self::capture::capture_window_data_url;
use self::detect::{enumerate_pin_windows, PinHost, PinWindow};

/// How long a pin must stay still (and without mouse drag) before the badge returns.
const IDLE_SHOW_DELAY: Duration = Duration::from_millis(420);
/// Pixel movement that counts as "moving" the pin.
const MOVE_THRESHOLD: i32 = 2;
/// Size change that counts as resize interaction.
const RESIZE_THRESHOLD: i32 = 2;

static STARTED: OnceLock<()> = OnceLock::new();
static PIXPIN_ENABLED: AtomicBool = AtomicBool::new(true);
static SNIPASTE_ENABLED: AtomicBool = AtomicBool::new(true);
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

struct TrackedPin {
    badge: HWND,
    last_x: i32,
    last_y: i32,
    last_w: i32,
    last_h: i32,
    /// Last moment the pin was considered idle (not moving / not selected).
    idle_since: Instant,
    visible: bool,
}

/// Start the pin-badge monitor thread (idempotent).
pub fn start(app: AppHandle) {
    let _ = APP_HANDLE.set(app);
    if STARTED.set(()).is_err() {
        return;
    }

    std::thread::Builder::new()
        .name("aaai-pin-badge".into())
        .spawn(|| {
            if let Err(error) = button::register_class() {
                tracing::warn!(feature = "pin_badge", error = %error, "failed to register badge class");
                return;
            }
            run_loop();
        })
        .ok();
}

pub fn configure(pixpin_enabled: bool, snipaste_enabled: bool) {
    PIXPIN_ENABLED.store(pixpin_enabled, Ordering::Relaxed);
    SNIPASTE_ENABLED.store(snipaste_enabled, Ordering::Relaxed);
}

/// Apply pin-badge feature flags + current theme accent from settings.
pub fn configure_from_settings(settings: &AppSettings) {
    configure(
        settings.pixpin_pin_ai_enabled,
        settings.snipaste_pin_ai_enabled,
    );
    theme::configure_from_settings(settings);
}

fn host_enabled(host: PinHost) -> bool {
    match host {
        PinHost::PixPin => PIXPIN_ENABLED.load(Ordering::Relaxed),
        PinHost::Snipaste => SNIPASTE_ENABLED.load(Ordering::Relaxed),
    }
}

fn run_loop() {
    let mut tracked: HashMap<isize, TrackedPin> = HashMap::new();
    let mut msg = MSG::default();
    let mut last_accent_gen = theme::accent_generation();

    loop {
        unsafe {
            while PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        for pin_hwnd in process_pending_clicks() {
            handle_pin_click(pin_hwnd);
        }

        let any_enabled =
            PIXPIN_ENABLED.load(Ordering::Relaxed) || SNIPASTE_ENABLED.load(Ordering::Relaxed);

        if !any_enabled {
            for (_, state) in tracked.drain() {
                destroy_button(state.badge);
            }
            std::thread::sleep(Duration::from_millis(400));
            continue;
        }

        // Theme accent changed (settings / VS Code theme) → repaint visible badges.
        let accent_gen = theme::accent_generation();
        if accent_gen != last_accent_gen {
            last_accent_gen = accent_gen;
            for state in tracked.values() {
                invalidate_button(state.badge);
            }
        }

        let pins: Vec<PinWindow> = enumerate_pin_windows()
            .into_iter()
            .filter(|pin| host_enabled(pin.host))
            .collect();

        let active: HashSet<isize> = pins.iter().map(|pin| pin.hwnd).collect();

        // Drop badges for closed pins.
        let stale: Vec<isize> = tracked
            .keys()
            .copied()
            .filter(|hwnd| !active.contains(hwnd))
            .collect();
        for hwnd in stale {
            if let Some(state) = tracked.remove(&hwnd) {
                destroy_button(state.badge);
            }
        }

        // Badge hwnd map for "cursor over badge" checks.
        let badge_map: HashMap<isize, HWND> = tracked.iter().map(|(k, v)| (*k, v.badge)).collect();
        let over_badge = cursor_over_any_badge(&badge_map);
        let mouse_interacting = is_mouse_button_down();

        for pin in &pins {
            let pin_hwnd = HWND(pin.hwnd as *mut _);
            if !unsafe { IsWindow(pin_hwnd).as_bool() } {
                continue;
            }

            // Ensure badge window exists.
            if let std::collections::hash_map::Entry::Vacant(entry) = tracked.entry(pin.hwnd) {
                match create_button_for_pin(pin.hwnd) {
                    Ok(badge) => {
                        entry.insert(TrackedPin {
                            badge,
                            last_x: pin.x,
                            last_y: pin.y,
                            last_w: pin.width,
                            last_h: pin.height,
                            idle_since: Instant::now(),
                            visible: false,
                        });
                    }
                    Err(error) => {
                        tracing::debug!(feature = "pin_badge", error = %error, "create badge failed");
                        continue;
                    }
                }
            }

            let Some(state) = tracked.get_mut(&pin.hwnd) else {
                continue;
            };

            if !unsafe { IsWindow(state.badge).as_bool() } {
                // Badge window died; recreate next tick.
                tracked.remove(&pin.hwnd);
                continue;
            }

            let moved = (pin.x - state.last_x).abs() > MOVE_THRESHOLD
                || (pin.y - state.last_y).abs() > MOVE_THRESHOLD;
            let resized = (pin.width - state.last_w).abs() > RESIZE_THRESHOLD
                || (pin.height - state.last_h).abs() > RESIZE_THRESHOLD;

            let selected = is_pin_selected_or_dragged(pin.hwnd, pin, over_badge, mouse_interacting);
            let busy = moved || resized || selected;

            if busy {
                state.idle_since = Instant::now();
                if state.visible {
                    hide_button(state.badge);
                    state.visible = false;
                }
            } else if !state.visible && state.idle_since.elapsed() >= IDLE_SHOW_DELAY {
                // Reposition before showing so it never flashes at the old place.
                let rect = RECT {
                    left: pin.x,
                    top: pin.y,
                    right: pin.x + pin.width,
                    bottom: pin.y + pin.height,
                };
                position_button(state.badge, &rect);
                show_button(state.badge);
                state.visible = true;
            } else if state.visible {
                // Keep following the pin while idle.
                let rect = RECT {
                    left: pin.x,
                    top: pin.y,
                    right: pin.x + pin.width,
                    bottom: pin.y + pin.height,
                };
                position_button(state.badge, &rect);
            }

            state.last_x = pin.x;
            state.last_y = pin.y;
            state.last_w = pin.width;
            state.last_h = pin.height;
        }

        std::thread::sleep(Duration::from_millis(80));
    }
}

fn is_mouse_button_down() -> bool {
    unsafe {
        GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000 != 0
            || GetAsyncKeyState(VK_RBUTTON.0 as i32) as u16 & 0x8000 != 0
            || GetAsyncKeyState(VK_MBUTTON.0 as i32) as u16 & 0x8000 != 0
    }
}

/// True when the user is selecting / dragging the pin (not interacting with the badge).
fn is_pin_selected_or_dragged(
    pin_hwnd: isize,
    pin: &PinWindow,
    cursor_over_badge: bool,
    mouse_button_down: bool,
) -> bool {
    // Never treat badge clicks as pin interaction.
    if cursor_over_badge || !mouse_button_down {
        return false;
    }

    let mut pt = POINT::default();
    if unsafe { GetCursorPos(&mut pt) }.is_err() {
        return false;
    }

    let over_pin =
        pt.x >= pin.x && pt.x < pin.x + pin.width && pt.y >= pin.y && pt.y < pin.y + pin.height;

    // Pressing / dragging on the pin image → hide immediately.
    if over_pin {
        return true;
    }

    // Resize grips / chrome slightly outside the client rect, or child HWNDs.
    let under = unsafe { WindowFromPoint(pt) };
    if !under.0.is_null() && under.0 as isize == pin_hwnd {
        return true;
    }

    false
}

fn handle_pin_click(pin_hwnd: isize) {
    let hwnd = HWND(pin_hwnd as *mut _);
    if !unsafe { IsWindow(hwnd).as_bool() } {
        return;
    }

    let Some(data_url) = capture_window_data_url(hwnd) else {
        tracing::warn!(
            feature = "pin_badge",
            hwnd = pin_hwnd,
            "failed to capture pin image"
        );
        return;
    };

    let Some(app) = APP_HANDLE.get().cloned() else {
        return;
    };

    static LAST_CLICK: OnceLock<Mutex<std::time::Instant>> = OnceLock::new();
    {
        let guard = LAST_CLICK
            .get_or_init(|| Mutex::new(std::time::Instant::now() - Duration::from_secs(10)));
        if let Ok(mut last) = guard.lock() {
            if last.elapsed() < Duration::from_millis(500) {
                return;
            }
            *last = std::time::Instant::now();
        }
    }

    let app_for_overlay = app.clone();
    let _ = app.run_on_main_thread(move || {
        open_overlay_with_images(&app_for_overlay, vec![data_url]);
    });
}
