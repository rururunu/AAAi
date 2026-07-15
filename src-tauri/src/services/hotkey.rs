//! Secondary global hotkey: parse `Ctrl+Alt+Space`-style chords and match them via rdev.

use std::sync::{Mutex, MutexGuard, OnceLock};

use rdev::Key;

pub const DEFAULT_SECONDARY_HOTKEY: &str = "Ctrl+Alt+Space";

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
    match (chord_key, key) {
        (ChordKey::Space, Key::Space) => true,
        (ChordKey::Tab, Key::Tab) => true,
        (ChordKey::Enter, Key::Return) => true,
        (ChordKey::Backspace, Key::Backspace) => true,
        (ChordKey::Delete, Key::Delete) => true,
        (ChordKey::Insert, Key::Insert) => true,
        (ChordKey::Home, Key::Home) => true,
        (ChordKey::End, Key::End) => true,
        (ChordKey::PageUp, Key::PageUp) => true,
        (ChordKey::PageDown, Key::PageDown) => true,
        (ChordKey::Left, Key::LeftArrow) => true,
        (ChordKey::Right, Key::RightArrow) => true,
        (ChordKey::Up, Key::UpArrow) => true,
        (ChordKey::Down, Key::DownArrow) => true,
        (ChordKey::Semicolon, Key::SemiColon) => true,
        (ChordKey::Quote, Key::Quote) => true,
        (ChordKey::Comma, Key::Comma) => true,
        (ChordKey::Period, Key::Dot) => true,
        (ChordKey::Slash, Key::Slash) => true,
        (ChordKey::BackSlash, Key::BackSlash) => true,
        (ChordKey::Minus, Key::Minus) => true,
        (ChordKey::Equal, Key::Equal) => true,
        (ChordKey::LeftBracket, Key::LeftBracket) => true,
        (ChordKey::RightBracket, Key::RightBracket) => true,
        (ChordKey::BackQuote, Key::BackQuote) => true,
        (ChordKey::F(n), Key::F1) if n == 1 => true,
        (ChordKey::F(n), Key::F2) if n == 2 => true,
        (ChordKey::F(n), Key::F3) if n == 3 => true,
        (ChordKey::F(n), Key::F4) if n == 4 => true,
        (ChordKey::F(n), Key::F5) if n == 5 => true,
        (ChordKey::F(n), Key::F6) if n == 6 => true,
        (ChordKey::F(n), Key::F7) if n == 7 => true,
        (ChordKey::F(n), Key::F8) if n == 8 => true,
        (ChordKey::F(n), Key::F9) if n == 9 => true,
        (ChordKey::F(n), Key::F10) if n == 10 => true,
        (ChordKey::F(n), Key::F11) if n == 11 => true,
        (ChordKey::F(n), Key::F12) if n == 12 => true,
        (ChordKey::Digit(n), Key::Num0) if n == 0 => true,
        (ChordKey::Digit(n), Key::Num1) if n == 1 => true,
        (ChordKey::Digit(n), Key::Num2) if n == 2 => true,
        (ChordKey::Digit(n), Key::Num3) if n == 3 => true,
        (ChordKey::Digit(n), Key::Num4) if n == 4 => true,
        (ChordKey::Digit(n), Key::Num5) if n == 5 => true,
        (ChordKey::Digit(n), Key::Num6) if n == 6 => true,
        (ChordKey::Digit(n), Key::Num7) if n == 7 => true,
        (ChordKey::Digit(n), Key::Num8) if n == 8 => true,
        (ChordKey::Digit(n), Key::Num9) if n == 9 => true,
        (ChordKey::Letter('a'), Key::KeyA) => true,
        (ChordKey::Letter('b'), Key::KeyB) => true,
        (ChordKey::Letter('c'), Key::KeyC) => true,
        (ChordKey::Letter('d'), Key::KeyD) => true,
        (ChordKey::Letter('e'), Key::KeyE) => true,
        (ChordKey::Letter('f'), Key::KeyF) => true,
        (ChordKey::Letter('g'), Key::KeyG) => true,
        (ChordKey::Letter('h'), Key::KeyH) => true,
        (ChordKey::Letter('i'), Key::KeyI) => true,
        (ChordKey::Letter('j'), Key::KeyJ) => true,
        (ChordKey::Letter('k'), Key::KeyK) => true,
        (ChordKey::Letter('l'), Key::KeyL) => true,
        (ChordKey::Letter('m'), Key::KeyM) => true,
        (ChordKey::Letter('n'), Key::KeyN) => true,
        (ChordKey::Letter('o'), Key::KeyO) => true,
        (ChordKey::Letter('p'), Key::KeyP) => true,
        (ChordKey::Letter('q'), Key::KeyQ) => true,
        (ChordKey::Letter('r'), Key::KeyR) => true,
        (ChordKey::Letter('s'), Key::KeyS) => true,
        (ChordKey::Letter('t'), Key::KeyT) => true,
        (ChordKey::Letter('u'), Key::KeyU) => true,
        (ChordKey::Letter('v'), Key::KeyV) => true,
        (ChordKey::Letter('w'), Key::KeyW) => true,
        (ChordKey::Letter('x'), Key::KeyX) => true,
        (ChordKey::Letter('y'), Key::KeyY) => true,
        (ChordKey::Letter('z'), Key::KeyZ) => true,
        _ => false,
    }
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
}
