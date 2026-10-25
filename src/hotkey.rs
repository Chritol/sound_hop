//! Hotkeys as text ("Ctrl+Alt+1") and as the numbers Windows wants.
//!
//! Nothing in here calls Windows, so it is all unit-tested.

/// Modifier flags in the format `RegisterHotKey` expects (`MOD_*`).
pub const MOD_ALT: u32 = 0x1;
pub const MOD_CONTROL: u32 = 0x2;
pub const MOD_SHIFT: u32 = 0x4;
pub const MOD_WIN: u32 = 0x8;

/// Modifier flags in the format the settings window's hotkey box uses
/// (`HOTKEYF_*`). Windows uses a different bit order here, so we convert.
const HOTKEYF_SHIFT: u16 = 0x1;
const HOTKEYF_CONTROL: u16 = 0x2;
const HOTKEYF_ALT: u16 = 0x4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub mods: u32,
    pub vk: u32,
}

/// Names for keys that are not a single letter or digit.
const NAMED_KEYS: &[(&str, u32)] = &[
    ("Space", 0x20),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("End", 0x23),
    ("Home", 0x24),
    ("Left", 0x25),
    ("Up", 0x26),
    ("Right", 0x27),
    ("Down", 0x28),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("Num0", 0x60),
    ("Num1", 0x61),
    ("Num2", 0x62),
    ("Num3", 0x63),
    ("Num4", 0x64),
    ("Num5", 0x65),
    ("Num6", 0x66),
    ("Num7", 0x67),
    ("Num8", 0x68),
    ("Num9", 0x69),
    ("NumMultiply", 0x6A),
    ("NumAdd", 0x6B),
    ("NumSubtract", 0x6D),
    ("NumDecimal", 0x6E),
    ("NumDivide", 0x6F),
    ("Pause", 0x13),
    ("ScrollLock", 0x91),
    ("VolumeMute", 0xAD),
    ("VolumeDown", 0xAE),
    ("VolumeUp", 0xAF),
    ("MediaNext", 0xB0),
    ("MediaPrev", 0xB1),
    ("MediaStop", 0xB2),
    ("MediaPlay", 0xB3),
    (";", 0xBA),
    ("=", 0xBB),
    (",", 0xBC),
    ("-", 0xBD),
    (".", 0xBE),
    ("/", 0xBF),
    ("`", 0xC0),
    ("[", 0xDB),
    ("\\", 0xDC),
    ("]", 0xDD),
    ("'", 0xDE),
];

fn key_name(vk: u32) -> String {
    match vk {
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(vk).unwrap().to_string(),
        0x70..=0x87 => format!("F{}", vk - 0x70 + 1),
        _ => NAMED_KEYS
            .iter()
            .find(|(_, code)| *code == vk)
            .map(|(name, _)| name.to_string())
            .unwrap_or_else(|| format!("VK{vk}")),
    }
}

fn key_code(name: &str) -> Option<u32> {
    let upper = name.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    if bytes.len() == 1 && (bytes[0].is_ascii_uppercase() || bytes[0].is_ascii_digit()) {
        return Some(bytes[0] as u32);
    }
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        if (1..=24).contains(&n) {
            return Some(0x70 + n - 1);
        }
    }
    if let Some(n) = upper.strip_prefix("VK").and_then(|n| n.parse::<u32>().ok()) {
        return Some(n);
    }
    NAMED_KEYS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, code)| *code)
}

impl Hotkey {
    /// Parses text like `Ctrl+Alt+1`. Returns `None` for empty or invalid
    /// text. (`Option<T>` is Rust's version of a nullable `T?`.)
    pub fn parse(text: &str) -> Option<Hotkey> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let parts: Vec<&str> = text.split('+').map(str::trim).collect();
        // A trailing "+" means the key itself is "+", e.g. "Ctrl++".
        let (mod_parts, key) = if text.ends_with("++") {
            (&parts[..parts.len() - 2], "+")
        } else {
            (&parts[..parts.len() - 1], *parts.last()?)
        };
        let mut mods = 0;
        for m in mod_parts {
            mods |= match m.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => MOD_CONTROL,
                "alt" => MOD_ALT,
                "shift" => MOD_SHIFT,
                "win" | "windows" => MOD_WIN,
                _ => return None,
            };
        }
        let vk = if key == "+" { 0xBB } else { key_code(key)? };
        Some(Hotkey { mods, vk })
    }

    pub fn to_text(self) -> String {
        let mut s = String::new();
        if self.mods & MOD_CONTROL != 0 {
            s.push_str("Ctrl+");
        }
        if self.mods & MOD_ALT != 0 {
            s.push_str("Alt+");
        }
        if self.mods & MOD_SHIFT != 0 {
            s.push_str("Shift+");
        }
        if self.mods & MOD_WIN != 0 {
            s.push_str("Win+");
        }
        s.push_str(&key_name(self.vk));
        s
    }
}

impl Hotkey {
    /// Converts from the value of a Windows hotkey box (`HKM_GETHOTKEY`).
    pub fn from_control(value: u16) -> Option<Hotkey> {
        let vk = (value & 0xFF) as u32;
        if vk == 0 {
            return None;
        }
        let f = value >> 8;
        let mut mods = 0;
        if f & HOTKEYF_SHIFT != 0 {
            mods |= MOD_SHIFT;
        }
        if f & HOTKEYF_CONTROL != 0 {
            mods |= MOD_CONTROL;
        }
        if f & HOTKEYF_ALT != 0 {
            mods |= MOD_ALT;
        }
        Some(Hotkey { mods, vk })
    }

    /// Converts to the value a Windows hotkey box takes (`HKM_SETHOTKEY`).
    /// The box cannot show the Windows key, so that modifier is dropped.
    pub fn to_control(self) -> u16 {
        let mut f = 0;
        if self.mods & MOD_SHIFT != 0 {
            f |= HOTKEYF_SHIFT;
        }
        if self.mods & MOD_CONTROL != 0 {
            f |= HOTKEYF_CONTROL;
        }
        if self.mods & MOD_ALT != 0 {
            f |= HOTKEYF_ALT;
        }
        (f << 8) | (self.vk as u16 & 0xFF)
    }
}

/// The first unused default hotkey for a newly added device:
/// `Ctrl+Alt+1..9` for outputs, `Ctrl+Alt+Shift+1..9` for inputs.
/// Returns an empty string if all nine are taken.
pub fn free_default(for_output: bool, used: &[Hotkey]) -> String {
    let prefix = if for_output { "Ctrl+Alt+" } else { "Ctrl+Alt+Shift+" };
    (1..=9)
        .map(|n| format!("{prefix}{n}"))
        .find(|t| Hotkey::parse(t).is_some_and(|h| !used.contains(&h)))
        .unwrap_or_default()
}

/// Normalizes hotkey text, e.g. `ctrl+alt+1` becomes `Ctrl+Alt+1`.
/// Invalid text is returned unchanged so it can be reported.
pub fn normalize(text: &str) -> String {
    Hotkey::parse(text).map(Hotkey::to_text).unwrap_or_else(|| text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_format() {
        let h = Hotkey::parse("ctrl+alt+1").unwrap();
        assert_eq!(h, Hotkey { mods: MOD_CONTROL | MOD_ALT, vk: 0x31 });
        assert_eq!(h.to_text(), "Ctrl+Alt+1");
        assert_eq!(Hotkey::parse("Win+Shift+F13").unwrap().to_text(), "Shift+Win+F13");
        assert_eq!(Hotkey::parse("Ctrl+Num5").unwrap().vk, 0x65);
        assert_eq!(Hotkey::parse("Ctrl++").unwrap().vk, 0xBB);
        assert!(Hotkey::parse("").is_none());
        assert!(Hotkey::parse("Ctrl+Foo").is_none());
        assert!(Hotkey::parse("Hyper+A").is_none());
    }

    #[test]
    fn control_round_trip() {
        let h = Hotkey::parse("Ctrl+Shift+M").unwrap();
        assert_eq!(Hotkey::from_control(h.to_control()), Some(h));
        assert_eq!(Hotkey::from_control(0), None);
    }

    #[test]
    fn free_default_skips_used() {
        let used = [Hotkey::parse("Ctrl+Alt+1").unwrap(), Hotkey::parse("Ctrl+Alt+2").unwrap()];
        assert_eq!(free_default(true, &used), "Ctrl+Alt+3");
        assert_eq!(free_default(false, &used), "Ctrl+Alt+Shift+1");
        let all: Vec<Hotkey> = (1..=9).map(|n| Hotkey::parse(&format!("Ctrl+Alt+{n}")).unwrap()).collect();
        assert_eq!(free_default(true, &all), "");
    }

    #[test]
    fn every_formatted_key_parses_back() {
        for vk in 1..=0xFE {
            let h = Hotkey { mods: MOD_ALT, vk };
            assert_eq!(Hotkey::parse(&h.to_text()), Some(h), "vk {vk:#x}");
        }
    }
}
