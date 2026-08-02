//! Resolve the current AAAi accent color for the pin badge.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::models::settings::{AppSettings, ColorScheme};

/// Packed 0x00RRGGBB accent color (sRGB).
static ACCENT_RGB: AtomicU32 = AtomicU32::new(0x00_3B_8E_EA);
/// Bumped whenever accent changes so badges can repaint.
static ACCENT_GENERATION: AtomicU32 = AtomicU32::new(1);

const DARK_ACCENT: u32 = 0x00_3B_8E_EA; // #3b8eea — themes.css dark
const LIGHT_ACCENT: u32 = 0x00_00_67_C0; // #0067c0 — themes.css light

pub fn accent_generation() -> u32 {
    ACCENT_GENERATION.load(Ordering::Relaxed)
}

/// Returns (r, g, b) of the current theme accent.
pub fn accent_rgb() -> (u8, u8, u8) {
    let packed = ACCENT_RGB.load(Ordering::Relaxed);
    (
        ((packed >> 16) & 0xFF) as u8,
        ((packed >> 8) & 0xFF) as u8,
        (packed & 0xFF) as u8,
    )
}

pub fn configure_from_settings(settings: &AppSettings) {
    let packed = resolve_accent(settings);
    let prev = ACCENT_RGB.swap(packed, Ordering::Relaxed);
    if prev != packed {
        ACCENT_GENERATION.fetch_add(1, Ordering::Relaxed);
    }
}

fn resolve_accent(settings: &AppSettings) -> u32 {
    match settings.color_scheme {
        ColorScheme::Dark => DARK_ACCENT,
        ColorScheme::Light => LIGHT_ACCENT,
    }
}
