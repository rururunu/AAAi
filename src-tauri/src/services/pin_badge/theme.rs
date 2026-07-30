//! Resolve the current AAAi accent color for the pin badge.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::commands::themes::load_vscode_theme;
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
    if let Some(from_theme) = accent_from_vscode_theme(&settings.vscode_theme) {
        return from_theme;
    }
    match settings.color_scheme {
        ColorScheme::Dark => DARK_ACCENT,
        ColorScheme::Light => LIGHT_ACCENT,
    }
}

fn accent_from_vscode_theme(theme_id: &str) -> Option<u32> {
    let id = theme_id.trim();
    if id.is_empty() {
        return None;
    }
    let theme = load_vscode_theme(id.to_string()).ok()?;
    let keys = [
        "focusBorder",
        "textLink.foreground",
        "button.background",
        "activityBarBadge.background",
        "statusBarItem.remoteBackground",
    ];
    for key in keys {
        if let Some(value) = theme.colors.get(key) {
            if let Some(rgb) = parse_css_color(value) {
                return Some(pack_rgb(rgb.0, rgb.1, rgb.2));
            }
        }
    }
    None
}

fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn parse_css_color(value: &str) -> Option<(u8, u8, u8)> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        let expanded = match hex.len() {
            3 | 4 => hex.chars().map(|c| format!("{c}{c}")).collect::<String>(),
            6 | 8 => hex.to_string(),
            _ => return None,
        };
        if expanded.len() < 6 {
            return None;
        }
        let r = u8::from_str_radix(&expanded[0..2], 16).ok()?;
        let g = u8::from_str_radix(&expanded[2..4], 16).ok()?;
        let b = u8::from_str_radix(&expanded[4..6], 16).ok()?;
        return Some((r, g, b));
    }

    // rgb(r,g,b) / rgba(r,g,b,a)
    let lower = value.to_ascii_lowercase();
    if let Some(rest) = lower
        .strip_prefix("rgba(")
        .or_else(|| lower.strip_prefix("rgb("))
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts: Vec<&str> = rest.split([',', ' ']).filter(|p| !p.is_empty()).collect();
        if parts.len() >= 3 {
            let r = parts[0].parse::<f32>().ok()?.clamp(0.0, 255.0) as u8;
            let g = parts[1].parse::<f32>().ok()?.clamp(0.0, 255.0) as u8;
            let b = parts[2].parse::<f32>().ok()?.clamp(0.0, 255.0) as u8;
            return Some((r, g, b));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_colors() {
        assert_eq!(parse_css_color("#3b8eea"), Some((0x3b, 0x8e, 0xea)));
        assert_eq!(parse_css_color("#06c"), Some((0x00, 0x66, 0xcc)));
    }
}
