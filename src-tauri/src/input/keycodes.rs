//! Key naming uses the DOM `KeyboardEvent.code` vocabulary so the UI can
//! capture a hotkey without any translation table of its own.

use crate::macros::model::Modifier;

/// (code name, virtual key, needs KEYEVENTF_EXTENDEDKEY)
pub struct KeyDef {
    pub name: &'static str,
    pub vk: u16,
    pub extended: bool,
}

const fn k(name: &'static str, vk: u16) -> KeyDef {
    KeyDef { name, vk, extended: false }
}

const fn e(name: &'static str, vk: u16) -> KeyDef {
    KeyDef { name, vk, extended: true }
}

pub static KEYS: &[KeyDef] = &[
    k("KeyA", 0x41), k("KeyB", 0x42), k("KeyC", 0x43), k("KeyD", 0x44),
    k("KeyE", 0x45), k("KeyF", 0x46), k("KeyG", 0x47), k("KeyH", 0x48),
    k("KeyI", 0x49), k("KeyJ", 0x4A), k("KeyK", 0x4B), k("KeyL", 0x4C),
    k("KeyM", 0x4D), k("KeyN", 0x4E), k("KeyO", 0x4F), k("KeyP", 0x50),
    k("KeyQ", 0x51), k("KeyR", 0x52), k("KeyS", 0x53), k("KeyT", 0x54),
    k("KeyU", 0x55), k("KeyV", 0x56), k("KeyW", 0x57), k("KeyX", 0x58),
    k("KeyY", 0x59), k("KeyZ", 0x5A),

    k("Digit0", 0x30), k("Digit1", 0x31), k("Digit2", 0x32), k("Digit3", 0x33),
    k("Digit4", 0x34), k("Digit5", 0x35), k("Digit6", 0x36), k("Digit7", 0x37),
    k("Digit8", 0x38), k("Digit9", 0x39),

    k("F1", 0x70), k("F2", 0x71), k("F3", 0x72), k("F4", 0x73),
    k("F5", 0x74), k("F6", 0x75), k("F7", 0x76), k("F8", 0x77),
    k("F9", 0x78), k("F10", 0x79), k("F11", 0x7A), k("F12", 0x7B),
    k("F13", 0x7C), k("F14", 0x7D), k("F15", 0x7E), k("F16", 0x7F),
    k("F17", 0x80), k("F18", 0x81), k("F19", 0x82), k("F20", 0x83),
    k("F21", 0x84), k("F22", 0x85), k("F23", 0x86), k("F24", 0x87),

    k("Escape", 0x1B), k("Tab", 0x09), k("CapsLock", 0x14), k("Space", 0x20),
    k("Enter", 0x0D), k("Backspace", 0x08), k("Backquote", 0xC0),
    k("Minus", 0xBD), k("Equal", 0xBB), k("BracketLeft", 0xDB),
    k("BracketRight", 0xDD), k("Backslash", 0xDC), k("Semicolon", 0xBA),
    k("Quote", 0xDE), k("Comma", 0xBC), k("Period", 0xBE), k("Slash", 0xBF),
    k("IntlBackslash", 0xE2),

    k("ShiftLeft", 0xA0), e("ShiftRight", 0xA1),
    k("ControlLeft", 0xA2), e("ControlRight", 0xA3),
    k("AltLeft", 0xA4), e("AltRight", 0xA5),
    e("MetaLeft", 0x5B), e("MetaRight", 0x5C), e("ContextMenu", 0x5D),

    e("Insert", 0x2D), e("Delete", 0x2E), e("Home", 0x24), e("End", 0x23),
    e("PageUp", 0x21), e("PageDown", 0x22),
    e("ArrowLeft", 0x25), e("ArrowUp", 0x26), e("ArrowRight", 0x27), e("ArrowDown", 0x28),

    e("PrintScreen", 0x2C), k("ScrollLock", 0x91), k("Pause", 0x13), e("NumLock", 0x90),

    k("Numpad0", 0x60), k("Numpad1", 0x61), k("Numpad2", 0x62), k("Numpad3", 0x63),
    k("Numpad4", 0x64), k("Numpad5", 0x65), k("Numpad6", 0x66), k("Numpad7", 0x67),
    k("Numpad8", 0x68), k("Numpad9", 0x69),
    k("NumpadMultiply", 0x6A), k("NumpadAdd", 0x6B), k("NumpadSubtract", 0x6D),
    k("NumpadDecimal", 0x6E), e("NumpadDivide", 0x6F), e("NumpadEnter", 0x0D),
];

pub fn lookup(name: &str) -> Option<&'static KeyDef> {
    KEYS.iter().find(|def| def.name.eq_ignore_ascii_case(name))
}

/// Reverse lookup for the recorder. Extended keys win over their plain twin
/// only when the hook reported the extended flag.
pub fn name_for_vk(vk: u16, extended: bool) -> Option<&'static str> {
    KEYS.iter()
        .find(|def| def.vk == vk && def.extended == extended)
        .or_else(|| KEYS.iter().find(|def| def.vk == vk))
        .map(|def| def.name)
}

pub fn modifier_vk(modifier: Modifier) -> u16 {
    match modifier {
        Modifier::Ctrl => 0xA2,
        Modifier::Shift => 0xA0,
        Modifier::Alt => 0xA4,
        Modifier::Win => 0x5B,
    }
}

pub fn modifier_extended(modifier: Modifier) -> bool {
    matches!(modifier, Modifier::Win)
}

/// A key name is a modifier key itself — used to reject silly hotkeys.
pub fn is_modifier_key(name: &str) -> bool {
    matches!(
        name,
        "ShiftLeft"
            | "ShiftRight"
            | "ControlLeft"
            | "ControlRight"
            | "AltLeft"
            | "AltRight"
            | "MetaLeft"
            | "MetaRight"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_is_unique() {
        let mut names: Vec<&str> = KEYS.iter().map(|d| d.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "duplicate key name in table");
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(lookup("keya").map(|d| d.vk), Some(0x41));
        assert_eq!(lookup("F12").map(|d| d.vk), Some(0x7B));
        assert!(lookup("NotAKey").is_none());
    }

    #[test]
    fn numpad_enter_is_extended_enter() {
        let plain = lookup("Enter").expect("Enter");
        let numpad = lookup("NumpadEnter").expect("NumpadEnter");
        assert_eq!(plain.vk, numpad.vk);
        assert!(!plain.extended);
        assert!(numpad.extended);
    }

    #[test]
    fn reverse_lookup_prefers_matching_extended_flag() {
        assert_eq!(name_for_vk(0x0D, false), Some("Enter"));
        assert_eq!(name_for_vk(0x0D, true), Some("NumpadEnter"));
        assert_eq!(name_for_vk(0xA1, true), Some("ShiftRight"));
    }
}
