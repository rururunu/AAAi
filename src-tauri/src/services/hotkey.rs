//! Configurable global hotkeys for overlay activation.

use std::sync::{Mutex, MutexGuard, OnceLock};

use rdev::Key;

pub const DEFAULT_SECONDARY_HOTKEY: &str = "Ctrl+Alt+Space";
/// Primary overlay gesture — double-tap `Alt` (configurable modifier).
pub const DEFAULT_PRIMARY_HOTKEY: &str = "Alt";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PrimaryHotkey {
    #[default]
    Alt,
    Ctrl,
    Shift,
    Meta,
}

impl PrimaryHotkey {
    pub fn matches(self, key: Key) -> bool {
        match self {
            Self::Alt => is_alt_key(key),
            Self::Ctrl => is_ctrl_key(key),
            Self::Shift => is_shift_key(key),
            Self::Meta => is_meta_key(key),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Alt => "Alt",
            Self::Ctrl => "Ctrl",
            Self::Shift => "Shift",
            Self::Meta => "Meta",
        }
    }
}

pub fn parse_primary_hotkey(raw: &str) -> Result<PrimaryHotkey, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "alt" | "option" => Ok(PrimaryHotkey::Alt),
        "ctrl" | "control" => Ok(PrimaryHotkey::Ctrl),
        "shift" => Ok(PrimaryHotkey::Shift),
        "meta" | "win" | "super" | "cmd" | "command" => Ok(PrimaryHotkey::Meta),
        _ => Err(format!("unsupported primary hotkey: {raw}")),
    }
}

pub fn normalize_primary_hotkey(raw: &str) -> String {
    parse_primary_hotkey(raw)
        .map(|hotkey| hotkey.label().to_string())
        .unwrap_or_else(|_| DEFAULT_PRIMARY_HOTKEY.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordKey {
    Space,
    Tab,
    Enter,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Right,
    Up,
    Down,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    BackSlash,
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    BackQuote,
    F(u8),
    Digit(u8),
    Letter(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedChord {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
    pub key: ChordKey,
}

impl Default for ParsedChord {
    fn default() -> Self {
        parse_hotkey(DEFAULT_SECONDARY_HOTKEY).expect("default hotkey parses")
    }
}

pub fn parse_hotkey(raw: &str) -> Result<ParsedChord, String> {
    let parts: Vec<&str> = raw
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return Err("empty hotkey".into());
    }

    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut meta = false;
    let mut key: Option<ChordKey> = None;

    for part in parts {
        let lower = part.to_ascii_lowercase();
        match lower.as_str() {
            "ctrl" | "control" | "controlleft" | "controlright" => ctrl = true,
            "shift" | "shiftleft" | "shiftright" => shift = true,
            "alt" | "option" | "altgr" | "altleft" | "altright" => alt = true,
            "meta" | "win" | "super" | "cmd" | "command" => meta = true,
            other => {
                if key.is_some() {
                    return Err(format!("multiple primary keys in hotkey: {raw}"));
                }
                key = Some(parse_primary_key(other)?);
            }
        }
    }

    let key = key.ok_or_else(|| "hotkey must include a primary key".to_string())?;
    if !ctrl && !shift && !alt && !meta {
        return Err("hotkey must include at least one modifier".into());
    }
    // Bare Alt+? with no other mod is allowed; plain Alt alone is already rejected.
    Ok(ParsedChord {
        ctrl,
        shift,
        alt,
        meta,
        key,
    })
}

pub fn format_hotkey(chord: &ParsedChord) -> String {
    let mut parts = Vec::new();
    if chord.ctrl {
        parts.push("Ctrl");
    }
    if chord.shift {
        parts.push("Shift");
    }
    if chord.alt {
        parts.push("Alt");
    }
    if chord.meta {
        parts.push("Meta");
    }
    parts.push(primary_key_label(chord.key));
    parts.join("+")
}

pub fn normalize_hotkey(raw: &str) -> String {
    parse_hotkey(raw)
        .map(|chord| format_hotkey(&chord))
        .unwrap_or_else(|_| DEFAULT_SECONDARY_HOTKEY.to_string())
}

fn parse_primary_key(token: &str) -> Result<ChordKey, String> {
    let lower = token.to_ascii_lowercase();
    Ok(match lower.as_str() {
        "space" => ChordKey::Space,
        "tab" => ChordKey::Tab,
        "enter" | "return" => ChordKey::Enter,
        "backspace" => ChordKey::Backspace,
        "delete" | "del" => ChordKey::Delete,
        "insert" | "ins" => ChordKey::Insert,
        "home" => ChordKey::Home,
        "end" => ChordKey::End,
        "pageup" | "pgup" => ChordKey::PageUp,
        "pagedown" | "pgdn" => ChordKey::PageDown,
        "left" | "arrowleft" => ChordKey::Left,
        "right" | "arrowright" => ChordKey::Right,
        "up" | "arrowup" => ChordKey::Up,
        "down" | "arrowdown" => ChordKey::Down,
        "semicolon" | ";" => ChordKey::Semicolon,
        "quote" | "'" => ChordKey::Quote,
        "comma" | "," => ChordKey::Comma,
        "period" | "." | "dot" => ChordKey::Period,
        "slash" | "/" => ChordKey::Slash,
        "backslash" | "\\" => ChordKey::BackSlash,
        "minus" | "-" => ChordKey::Minus,
        "equal" | "equals" | "=" => ChordKey::Equal,
        "leftbracket" | "[" => ChordKey::LeftBracket,
        "rightbracket" | "]" => ChordKey::RightBracket,
        "backquote" | "`" | "grave" => ChordKey::BackQuote,
        other if other.len() == 1 => {
            let ch = other.chars().next().unwrap();
            if ch.is_ascii_digit() {
                ChordKey::Digit(ch as u8 - b'0')
            } else if ch.is_ascii_alphabetic() {
                ChordKey::Letter(ch.to_ascii_lowercase())
            } else {
                return Err(format!("unsupported key: {token}"));
            }
        }
        other if other.starts_with('f') && other.len() <= 3 => {
            let num: u8 = other[1..]
                .parse()
                .map_err(|_| format!("unsupported key: {token}"))?;
            if (1..=24).contains(&num) {
                ChordKey::F(num)
            } else {
                return Err(format!("unsupported key: {token}"));
            }
        }
        _ => return Err(format!("unsupported key: {token}")),
    })
}

fn primary_key_label(key: ChordKey) -> &'static str {
    match key {
        ChordKey::Space => "Space",
        ChordKey::Tab => "Tab",
        ChordKey::Enter => "Enter",
        ChordKey::Backspace => "Backspace",
        ChordKey::Delete => "Delete",
        ChordKey::Insert => "Insert",
        ChordKey::Home => "Home",
        ChordKey::End => "End",
        ChordKey::PageUp => "PageUp",
        ChordKey::PageDown => "PageDown",
        ChordKey::Left => "Left",
        ChordKey::Right => "Right",
        ChordKey::Up => "Up",
        ChordKey::Down => "Down",
        ChordKey::Semicolon => ";",
        ChordKey::Quote => "'",
        ChordKey::Comma => ",",
        ChordKey::Period => ".",
        ChordKey::Slash => "/",
        ChordKey::BackSlash => "\\",
        ChordKey::Minus => "-",
        ChordKey::Equal => "=",
        ChordKey::LeftBracket => "[",
        ChordKey::RightBracket => "]",
        ChordKey::BackQuote => "`",
        ChordKey::F(n) => match n {
            1 => "F1",
            2 => "F2",
            3 => "F3",
            4 => "F4",
            5 => "F5",
            6 => "F6",
            7 => "F7",
            8 => "F8",
            9 => "F9",
            10 => "F10",
            11 => "F11",
            12 => "F12",
            _ => "F1",
        },
        ChordKey::Digit(n) => match n {
            0 => "0",
            1 => "1",
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            6 => "6",
            7 => "7",
            8 => "8",
            9 => "9",
            _ => "0",
        },
        ChordKey::Letter(ch) => match ch {
            'a' => "A",
            'b' => "B",
            'c' => "C",
            'd' => "D",
            'e' => "E",
            'f' => "F",
            'g' => "G",
            'h' => "H",
            'i' => "I",
            'j' => "J",
            'k' => "K",
            'l' => "L",
            'm' => "M",
            'n' => "N",
            'o' => "O",
            'p' => "P",
            'q' => "Q",
            'r' => "R",
            's' => "S",
            't' => "T",
            'u' => "U",
            'v' => "V",
            'w' => "W",
            'x' => "X",
            'y' => "Y",
            'z' => "Z",
            _ => "A",
        },
    }
}

fn key_matches(chord_key: ChordKey, key: Key) -> bool {
    matches!(
        (chord_key, key),
        (ChordKey::Space, Key::Space)
            | (ChordKey::Tab, Key::Tab)
            | (ChordKey::Enter, Key::Return)
            | (ChordKey::Backspace, Key::Backspace)
            | (ChordKey::Delete, Key::Delete)
            | (ChordKey::Insert, Key::Insert)
            | (ChordKey::Home, Key::Home)
            | (ChordKey::End, Key::End)
            | (ChordKey::PageUp, Key::PageUp)
            | (ChordKey::PageDown, Key::PageDown)
            | (ChordKey::Left, Key::LeftArrow)
            | (ChordKey::Right, Key::RightArrow)
            | (ChordKey::Up, Key::UpArrow)
            | (ChordKey::Down, Key::DownArrow)
            | (ChordKey::Semicolon, Key::SemiColon)
            | (ChordKey::Quote, Key::Quote)
            | (ChordKey::Comma, Key::Comma)
            | (ChordKey::Period, Key::Dot)
            | (ChordKey::Slash, Key::Slash)
            | (ChordKey::BackSlash, Key::BackSlash)
            | (ChordKey::Minus, Key::Minus)
            | (ChordKey::Equal, Key::Equal)
            | (ChordKey::LeftBracket, Key::LeftBracket)
            | (ChordKey::RightBracket, Key::RightBracket)
            | (ChordKey::BackQuote, Key::BackQuote)
            | (ChordKey::F(1), Key::F1)
            | (ChordKey::F(2), Key::F2)
            | (ChordKey::F(3), Key::F3)
            | (ChordKey::F(4), Key::F4)
            | (ChordKey::F(5), Key::F5)
            | (ChordKey::F(6), Key::F6)
            | (ChordKey::F(7), Key::F7)
            | (ChordKey::F(8), Key::F8)
            | (ChordKey::F(9), Key::F9)
            | (ChordKey::F(10), Key::F10)
            | (ChordKey::F(11), Key::F11)
            | (ChordKey::F(12), Key::F12)
            | (ChordKey::Digit(0), Key::Num0)
            | (ChordKey::Digit(1), Key::Num1)
            | (ChordKey::Digit(2), Key::Num2)
            | (ChordKey::Digit(3), Key::Num3)
            | (ChordKey::Digit(4), Key::Num4)
            | (ChordKey::Digit(5), Key::Num5)
            | (ChordKey::Digit(6), Key::Num6)
            | (ChordKey::Digit(7), Key::Num7)
            | (ChordKey::Digit(8), Key::Num8)
            | (ChordKey::Digit(9), Key::Num9)
            | (ChordKey::Letter('a'), Key::KeyA)
            | (ChordKey::Letter('b'), Key::KeyB)
            | (ChordKey::Letter('c'), Key::KeyC)
            | (ChordKey::Letter('d'), Key::KeyD)
            | (ChordKey::Letter('e'), Key::KeyE)
            | (ChordKey::Letter('f'), Key::KeyF)
            | (ChordKey::Letter('g'), Key::KeyG)
            | (ChordKey::Letter('h'), Key::KeyH)
            | (ChordKey::Letter('i'), Key::KeyI)
            | (ChordKey::Letter('j'), Key::KeyJ)
            | (ChordKey::Letter('k'), Key::KeyK)
            | (ChordKey::Letter('l'), Key::KeyL)
            | (ChordKey::Letter('m'), Key::KeyM)
            | (ChordKey::Letter('n'), Key::KeyN)
            | (ChordKey::Letter('o'), Key::KeyO)
            | (ChordKey::Letter('p'), Key::KeyP)
            | (ChordKey::Letter('q'), Key::KeyQ)
            | (ChordKey::Letter('r'), Key::KeyR)
            | (ChordKey::Letter('s'), Key::KeyS)
            | (ChordKey::Letter('t'), Key::KeyT)
            | (ChordKey::Letter('u'), Key::KeyU)
            | (ChordKey::Letter('v'), Key::KeyV)
            | (ChordKey::Letter('w'), Key::KeyW)
            | (ChordKey::Letter('x'), Key::KeyX)
            | (ChordKey::Letter('y'), Key::KeyY)
            | (ChordKey::Letter('z'), Key::KeyZ)
    )
}

fn is_ctrl_key(key: Key) -> bool {
    matches!(key, Key::ControlLeft | Key::ControlRight)
}

fn is_shift_key(key: Key) -> bool {
    matches!(key, Key::ShiftLeft | Key::ShiftRight)
}

fn is_alt_key(key: Key) -> bool {
    matches!(key, Key::Alt | Key::AltGr)
}

fn is_meta_key(key: Key) -> bool {
    matches!(key, Key::MetaLeft | Key::MetaRight)
}

#[derive(Debug, Default)]
pub struct SecondaryHotkeyDetector {
    ctrl: bool,
    shift: bool,
    alt: bool,
    meta: bool,
    armed: bool,
}

impl SecondaryHotkeyDetector {
    pub fn key_press(&mut self, key: Key, chord: &ParsedChord) {
        if is_ctrl_key(key) {
            self.ctrl = true;
            return;
        }
        if is_shift_key(key) {
            self.shift = true;
            return;
        }
        if is_alt_key(key) {
            self.alt = true;
            return;
        }
        if is_meta_key(key) {
            self.meta = true;
            return;
        }
        if modifiers_match(self, chord) && key_matches(chord.key, key) {
            self.armed = true;
        }
    }

    pub fn key_release(&mut self, key: Key, chord: &ParsedChord) -> bool {
        if is_ctrl_key(key) {
            self.ctrl = false;
            self.armed = false;
            return false;
        }
        if is_shift_key(key) {
            self.shift = false;
            self.armed = false;
            return false;
        }
        if is_alt_key(key) {
            self.alt = false;
            self.armed = false;
            return false;
        }
        if is_meta_key(key) {
            self.meta = false;
            self.armed = false;
            return false;
        }
        if self.armed && key_matches(chord.key, key) {
            self.armed = false;
            return true;
        }
        false
    }
}

fn modifiers_match(state: &SecondaryHotkeyDetector, chord: &ParsedChord) -> bool {
    state.ctrl == chord.ctrl
        && state.shift == chord.shift
        && state.alt == chord.alt
        && state.meta == chord.meta
}

pub fn shared_secondary_hotkey() -> &'static Mutex<ParsedChord> {
    static HOTKEY: OnceLock<Mutex<ParsedChord>> = OnceLock::new();
    HOTKEY.get_or_init(|| Mutex::new(ParsedChord::default()))
}

pub fn configure_secondary_hotkey(raw: &str) {
    let chord = parse_hotkey(raw).unwrap_or_default();
    *lock_recover(shared_secondary_hotkey()) = chord;
}

pub fn current_secondary_hotkey() -> ParsedChord {
    *lock_recover(shared_secondary_hotkey())
}

pub fn shared_secondary_hotkey_enabled() -> &'static Mutex<bool> {
    static ENABLED: OnceLock<Mutex<bool>> = OnceLock::new();
    ENABLED.get_or_init(|| Mutex::new(true))
}

pub fn configure_secondary_hotkey_enabled(enabled: bool) {
    *lock_recover(shared_secondary_hotkey_enabled()) = enabled;
}

pub fn secondary_hotkey_enabled() -> bool {
    *lock_recover(shared_secondary_hotkey_enabled())
}

pub fn shared_primary_hotkey() -> &'static Mutex<PrimaryHotkey> {
    static HOTKEY: OnceLock<Mutex<PrimaryHotkey>> = OnceLock::new();
    HOTKEY.get_or_init(|| Mutex::new(PrimaryHotkey::default()))
}

pub fn configure_primary_hotkey(raw: &str) {
    let hotkey = parse_primary_hotkey(&normalize_primary_hotkey(raw)).unwrap_or_default();
    *lock_recover(shared_primary_hotkey()) = hotkey;
}

pub fn current_primary_hotkey() -> PrimaryHotkey {
    *lock_recover(shared_primary_hotkey())
}

pub fn shared_primary_hotkey_enabled() -> &'static Mutex<bool> {
    static ENABLED: OnceLock<Mutex<bool>> = OnceLock::new();
    ENABLED.get_or_init(|| Mutex::new(true))
}

pub fn configure_primary_hotkey_enabled(enabled: bool) {
    *lock_recover(shared_primary_hotkey_enabled()) = enabled;
}

pub fn primary_hotkey_enabled() -> bool {
    *lock_recover(shared_primary_hotkey_enabled())
}

/// Max gap between first tap release and second tap press (must feel intentional).
pub const DOUBLE_TAP_GAP_MAX_MS: u64 = 300;
/// Min gap — ignore bounce / OS duplicate events from a single physical tap.
pub const DOUBLE_TAP_GAP_MIN_MS: u64 = 50;
/// Each press must be a short tap; long hold (Alt menu / Alt+Tab prep) cancels.
pub const TAP_MAX_HOLD_MS: u64 = 200;

/// Detect a deliberate double-tap of the primary modifier (default: Alt).
///
/// Fires only when the user completes two short, clean taps in quick succession:
/// press→release→press→release, each hold ≤ [`TAP_MAX_HOLD_MS`], gap in
/// [`DOUBLE_TAP_GAP_MIN_MS`, `DOUBLE_TAP_GAP_MAX_MS`]. Any other key, a long hold,
/// or a slow second tap resets the sequence.
#[derive(Debug, Default)]
pub struct DoubleModifierDetector {
    modifier: Option<PrimaryHotkey>,
    modifier_down: bool,
    chorded: bool,
    /// Timestamp of the current modifier KeyPress.
    press_ms: Option<u64>,
    /// Timestamp of the previous clean tap's KeyRelease.
    last_tap_release_ms: Option<u64>,
    /// Second press landed inside the double-tap window; fire on its KeyRelease
    /// so the modifier is up before clipboard capture simulates Ctrl+Insert.
    pending_trigger: bool,
}

impl DoubleModifierDetector {
    pub fn sync_modifier(&mut self, modifier: PrimaryHotkey) {
        if self.modifier != Some(modifier) {
            *self = Self {
                modifier: Some(modifier),
                ..Self::default()
            };
        }
    }

    pub fn key_press(&mut self, key: Key, now: u64, modifier: PrimaryHotkey) {
        self.sync_modifier(modifier);
        if modifier.matches(key) {
            // Ignore OS key-repeat while the modifier is held.
            if self.modifier_down {
                return;
            }
            self.modifier_down = true;
            self.chorded = false;
            self.press_ms = Some(now);
            self.pending_trigger = self.last_tap_release_ms.is_some_and(|last| {
                let gap = now.saturating_sub(last);
                (DOUBLE_TAP_GAP_MIN_MS..=DOUBLE_TAP_GAP_MAX_MS).contains(&gap)
            });
            return;
        }

        // Any other key aborts — both mid-hold chords and between-tap typing.
        self.chorded = self.modifier_down;
        self.last_tap_release_ms = None;
        self.pending_trigger = false;
    }

    pub fn key_release(&mut self, key: Key, now: u64, modifier: PrimaryHotkey) -> bool {
        self.sync_modifier(modifier);
        if !modifier.matches(key) || !self.modifier_down {
            return false;
        }

        self.modifier_down = false;
        let hold_ms = self
            .press_ms
            .map(|press| now.saturating_sub(press))
            .unwrap_or(u64::MAX);
        self.press_ms = None;

        if self.chorded || hold_ms > TAP_MAX_HOLD_MS {
            self.chorded = false;
            self.last_tap_release_ms = None;
            self.pending_trigger = false;
            return false;
        }

        if self.pending_trigger {
            self.pending_trigger = false;
            self.last_tap_release_ms = None;
            return true;
        }

        self.last_tap_release_ms = Some(now);
        false
    }
}

fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_chord() {
        let chord = parse_hotkey("Ctrl+Alt+Space").unwrap();
        assert!(chord.ctrl && chord.alt && !chord.shift);
        assert_eq!(chord.key, ChordKey::Space);
        assert_eq!(format_hotkey(&chord), "Ctrl+Alt+Space");
    }

    #[test]
    fn normalizes_primary_double_tap_modifier() {
        assert_eq!(normalize_primary_hotkey("control"), "Ctrl");
        assert_eq!(normalize_primary_hotkey("Alt+;"), "Alt");
        assert_eq!(normalize_primary_hotkey("Ctrl+`"), "Alt");
        assert_eq!(normalize_primary_hotkey("invalid"), DEFAULT_PRIMARY_HOTKEY);
    }

    #[test]
    fn primary_hotkey_detection() {
        assert!(parse_primary_hotkey("Alt").is_ok());
        assert!(parse_primary_hotkey("ctrl").is_ok());
        assert!(parse_primary_hotkey("Alt+;").is_err());
    }

    #[test]
    fn rejects_modifier_only() {
        assert!(parse_hotkey("Ctrl+Alt").is_err());
    }

    #[test]
    fn detector_triggers_on_matching_release() {
        let chord = parse_hotkey("Ctrl+Shift+P").unwrap();
        let mut detector = SecondaryHotkeyDetector::default();
        detector.key_press(Key::ControlLeft, &chord);
        detector.key_press(Key::ShiftLeft, &chord);
        detector.key_press(Key::KeyP, &chord);
        assert!(detector.key_release(Key::KeyP, &chord));
    }

    #[test]
    fn detector_ignores_wrong_modifiers() {
        let chord = parse_hotkey("Ctrl+Alt+Space").unwrap();
        let mut detector = SecondaryHotkeyDetector::default();
        detector.key_press(Key::ControlLeft, &chord);
        detector.key_press(Key::Space, &chord);
        assert!(!detector.key_release(Key::Space, &chord));
    }

    #[test]
    fn double_alt_requires_two_quick_short_taps() {
        let mut d = DoubleModifierDetector::default();
        let m = PrimaryHotkey::Alt;
        // Tap 1
        d.key_press(Key::Alt, 1_000, m);
        assert!(!d.key_release(Key::Alt, 1_080, m));
        // Tap 2 within window
        d.key_press(Key::Alt, 1_200, m);
        assert!(d.key_release(Key::Alt, 1_280, m));
    }

    #[test]
    fn double_alt_ignores_slow_second_tap() {
        let mut d = DoubleModifierDetector::default();
        let m = PrimaryHotkey::Alt;
        d.key_press(Key::Alt, 1_000, m);
        assert!(!d.key_release(Key::Alt, 1_080, m));
        let second = 1_080 + DOUBLE_TAP_GAP_MAX_MS + 1;
        d.key_press(Key::Alt, second, m);
        assert!(!d.key_release(Key::Alt, second + 80, m));
    }

    #[test]
    fn double_alt_ignores_long_hold() {
        let mut d = DoubleModifierDetector::default();
        let m = PrimaryHotkey::Alt;
        d.key_press(Key::Alt, 1_000, m);
        assert!(!d.key_release(Key::Alt, 1_000 + TAP_MAX_HOLD_MS + 1, m));
        d.key_press(Key::Alt, 1_250, m);
        assert!(!d.key_release(Key::Alt, 1_320, m));
    }

    #[test]
    fn double_alt_cancels_when_other_key_between_taps() {
        let mut d = DoubleModifierDetector::default();
        let m = PrimaryHotkey::Alt;
        d.key_press(Key::Alt, 1_000, m);
        assert!(!d.key_release(Key::Alt, 1_080, m));
        d.key_press(Key::KeyA, 1_120, m);
        d.key_press(Key::Alt, 1_200, m);
        assert!(!d.key_release(Key::Alt, 1_280, m));
    }

    #[test]
    fn double_alt_cancels_chord_like_alt_tab() {
        let mut d = DoubleModifierDetector::default();
        let m = PrimaryHotkey::Alt;
        d.key_press(Key::Alt, 1_000, m);
        d.key_press(Key::Tab, 1_050, m);
        assert!(!d.key_release(Key::Alt, 1_100, m));
    }

    #[test]
    fn double_alt_ignores_bounce_gap() {
        let mut d = DoubleModifierDetector::default();
        let m = PrimaryHotkey::Alt;
        d.key_press(Key::Alt, 1_000, m);
        assert!(!d.key_release(Key::Alt, 1_080, m));
        // Second press too soon after release — treat as bounce, not a new tap.
        d.key_press(Key::Alt, 1_080 + DOUBLE_TAP_GAP_MIN_MS - 1, m);
        assert!(!d.key_release(Key::Alt, 1_200, m));
    }
}
