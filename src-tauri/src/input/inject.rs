//! `SendInput` wrapper (§2.1).
//!
//! Keyboard events are sent as **scan codes**, not virtual keys: DirectInput
//! and RawInput based games ignore VK-only injection but do see scan codes.
//! Literal text uses `KEYEVENTF_UNICODE` instead, which is layout independent.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE,
    KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC_EX, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL,
    MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN,
};

use crate::input::keycodes;
use crate::macros::model::{Modifier, MouseButton, ScrollDirection};

/// Stamped into `dwExtraInfo` on every event we inject, so the global hook can
/// tell *our* playback apart from everything else.
///
/// The `LLKHF_INJECTED` flag is not good enough for that job: it is set for
/// any synthetic input, so filtering on it also throws away keystrokes coming
/// from PowerToys, AutoHotkey, a gaming keyboard's driver or an on-screen
/// keyboard — for those users the hotkeys would simply never fire. A private
/// signature keeps the self-injection guard exact.
pub const SIGNATURE: usize = 0x464D_5230;

const XBUTTON1: i32 = 0x0001;
const XBUTTON2: i32 = 0x0002;
const WHEEL_DELTA: i32 = 120;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InjectError {
    #[error("Unknown key: {0}")]
    UnknownKey(String),
    #[error("Input was blocked by Windows. The focused window probably runs as administrator.")]
    Blocked,
    #[error("Windows rejected the input event")]
    Rejected,
}

pub type InjectResult = Result<(), InjectError>;

fn send(inputs: &[INPUT]) -> InjectResult {
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else if sent == 0 {
        // UIPI: a lower integrity process cannot inject into an elevated window.
        Err(InjectError::Blocked)
    } else {
        Err(InjectError::Rejected)
    }
}

fn scan_code(vk: u16) -> u16 {
    let mapped = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC_EX) };
    (mapped & 0xFF) as u16
}

fn key_input(vk: u16, extended: bool, up: bool) -> INPUT {
    let scan = scan_code(vk);
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if scan != 0 {
        flags |= KEYEVENTF_SCANCODE;
    }
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if up {
        flags |= KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                // With KEYEVENTF_SCANCODE the virtual key is derived from wScan.
                wVk: if scan == 0 { VIRTUAL_KEY(vk) } else { VIRTUAL_KEY(0) },
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SIGNATURE,
            },
        },
    }
}

fn unicode_input(unit: u16, up: bool) -> INPUT {
    let mut flags = KEYEVENTF_UNICODE;
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SIGNATURE,
            },
        },
    }
}

fn mouse_input(flags: MOUSE_EVENT_FLAGS, dx: i32, dy: i32, data: i32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: data as u32,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SIGNATURE,
            },
        },
    }
}

// ---------------------------------------------------------------- keyboard

pub fn key_down(code: &str) -> InjectResult {
    let def = keycodes::lookup(code).ok_or_else(|| InjectError::UnknownKey(code.to_owned()))?;
    send(&[key_input(def.vk, def.extended, false)])
}

pub fn key_up(code: &str) -> InjectResult {
    let def = keycodes::lookup(code).ok_or_else(|| InjectError::UnknownKey(code.to_owned()))?;
    send(&[key_input(def.vk, def.extended, true)])
}

/// Press and release in a single `SendInput` batch so nothing can interleave.
pub fn key_press(code: &str) -> InjectResult {
    let def = keycodes::lookup(code).ok_or_else(|| InjectError::UnknownKey(code.to_owned()))?;
    send(&[
        key_input(def.vk, def.extended, false),
        key_input(def.vk, def.extended, true),
    ])
}

pub fn modifiers_down(modifiers: &[Modifier]) -> InjectResult {
    let batch: Vec<INPUT> = modifiers
        .iter()
        .map(|m| key_input(keycodes::modifier_vk(*m), keycodes::modifier_extended(*m), false))
        .collect();
    send(&batch)
}

/// Released in reverse order, mirroring how a hand lifts off a chord.
pub fn modifiers_up(modifiers: &[Modifier]) -> InjectResult {
    let batch: Vec<INPUT> = modifiers
        .iter()
        .rev()
        .map(|m| key_input(keycodes::modifier_vk(*m), keycodes::modifier_extended(*m), true))
        .collect();
    send(&batch)
}

/// One UTF-16 code unit per event pair; a surrogate pair sends two units.
pub fn type_char(ch: char) -> InjectResult {
    let mut buf = [0u16; 2];
    let units = ch.encode_utf16(&mut buf);
    let mut batch = Vec::with_capacity(units.len() * 2);
    for unit in units.iter() {
        batch.push(unicode_input(*unit, false));
        batch.push(unicode_input(*unit, true));
    }
    send(&batch)
}

// ------------------------------------------------------------------- mouse

fn button_flags(button: MouseButton, up: bool) -> (MOUSE_EVENT_FLAGS, i32) {
    match (button, up) {
        (MouseButton::Left, false) => (MOUSEEVENTF_LEFTDOWN, 0),
        (MouseButton::Left, true) => (MOUSEEVENTF_LEFTUP, 0),
        (MouseButton::Right, false) => (MOUSEEVENTF_RIGHTDOWN, 0),
        (MouseButton::Right, true) => (MOUSEEVENTF_RIGHTUP, 0),
        (MouseButton::Middle, false) => (MOUSEEVENTF_MIDDLEDOWN, 0),
        (MouseButton::Middle, true) => (MOUSEEVENTF_MIDDLEUP, 0),
        (MouseButton::Mouse4, false) => (MOUSEEVENTF_XDOWN, XBUTTON1),
        (MouseButton::Mouse4, true) => (MOUSEEVENTF_XUP, XBUTTON1),
        (MouseButton::Mouse5, false) => (MOUSEEVENTF_XDOWN, XBUTTON2),
        (MouseButton::Mouse5, true) => (MOUSEEVENTF_XUP, XBUTTON2),
    }
}

pub fn mouse_down(button: MouseButton) -> InjectResult {
    let (flags, data) = button_flags(button, false);
    send(&[mouse_input(flags, 0, 0, data)])
}

pub fn mouse_up(button: MouseButton) -> InjectResult {
    let (flags, data) = button_flags(button, true);
    send(&[mouse_input(flags, 0, 0, data)])
}

pub fn mouse_click(button: MouseButton) -> InjectResult {
    let (down_flags, data) = button_flags(button, false);
    let (up_flags, _) = button_flags(button, true);
    send(&[
        mouse_input(down_flags, 0, 0, data),
        mouse_input(up_flags, 0, 0, data),
    ])
}

pub fn scroll(direction: ScrollDirection, amount: u32) -> InjectResult {
    let sign = match direction {
        ScrollDirection::Up => 1,
        ScrollDirection::Down => -1,
    };
    let delta = sign * WHEEL_DELTA * amount.max(1) as i32;
    send(&[mouse_input(MOUSEEVENTF_WHEEL, 0, 0, delta)])
}

pub fn move_relative(dx: i32, dy: i32) -> InjectResult {
    send(&[mouse_input(MOUSEEVENTF_MOVE, dx, dy, 0)])
}

/// Absolute moves are normalised against the whole virtual desktop, so the
/// coordinates stay correct on multi-monitor setups (§2.1).
pub fn move_absolute(x: i32, y: i32) -> InjectResult {
    let (nx, ny) = normalize_to_virtual_desktop(x, y);
    send(&[mouse_input(
        MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
        nx,
        ny,
        0,
    )])
}

pub fn virtual_desktop() -> (i32, i32, i32, i32) {
    unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1),
            GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1),
        )
    }
}

fn normalize_to_virtual_desktop(x: i32, y: i32) -> (i32, i32) {
    let (vx, vy, vw, vh) = virtual_desktop();
    normalize(x, y, vx, vy, vw, vh)
}

fn normalize(x: i32, y: i32, vx: i32, vy: i32, vw: i32, vh: i32) -> (i32, i32) {
    let span_x = (vw - 1).max(1) as i64;
    let span_y = (vh - 1).max(1) as i64;
    let nx = ((x - vx) as i64 * 65_535 + span_x / 2) / span_x;
    let ny = ((y - vy) as i64 * 65_535 + span_y / 2) / span_y;
    (nx.clamp(0, 65_535) as i32, ny.clamp(0, 65_535) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_corners_of_a_single_screen() {
        assert_eq!(normalize(0, 0, 0, 0, 1920, 1080), (0, 0));
        assert_eq!(normalize(1919, 1079, 0, 0, 1920, 1080), (65_535, 65_535));
    }

    #[test]
    fn normalizes_against_a_negative_origin() {
        // Second monitor placed to the left of the primary one.
        let (x, _) = normalize(-1920, 0, -1920, 0, 3840, 1080);
        assert_eq!(x, 0);
        let (x, _) = normalize(0, 0, -1920, 0, 3840, 1080);
        assert!((32_750..=32_790).contains(&x), "midpoint drifted: {x}");
    }

    #[test]
    fn clamps_out_of_range_points() {
        assert_eq!(normalize(9999, 9999, 0, 0, 1920, 1080), (65_535, 65_535));
        assert_eq!(normalize(-50, -50, 0, 0, 1920, 1080), (0, 0));
    }

    /// End-to-end proof that `SendInput` actually reaches the system: move the
    /// real cursor and read it back. Ignored by default because it takes over
    /// the pointer for a moment — run it with `cargo test -- --ignored`.
    #[test]
    #[ignore = "moves the real mouse cursor"]
    fn absolute_move_lands_where_it_was_asked_to() {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

        let mut origin = POINT::default();
        unsafe { GetCursorPos(&mut origin).expect("read cursor") };

        let (vx, vy, vw, vh) = virtual_desktop();
        let target_x = vx + vw / 3;
        let target_y = vy + vh / 3;

        move_absolute(target_x, target_y).expect("inject move");
        // SendInput is asynchronous; give the input queue a moment to drain.
        std::thread::sleep(std::time::Duration::from_millis(60));

        let mut landed = POINT::default();
        unsafe { GetCursorPos(&mut landed).expect("read cursor") };
        unsafe { SetCursorPos(origin.x, origin.y).expect("restore cursor") };

        // Normalisation rounds to a 1/65535 grid, so allow a couple of pixels.
        assert!(
            (landed.x - target_x).abs() <= 2 && (landed.y - target_y).abs() <= 2,
            "asked for ({target_x}, {target_y}) but landed at ({}, {})",
            landed.x,
            landed.y
        );
    }

    #[test]
    fn x_buttons_carry_distinct_mouse_data() {
        let (_, four) = button_flags(MouseButton::Mouse4, false);
        let (_, five) = button_flags(MouseButton::Mouse5, false);
        assert_eq!(four, XBUTTON1);
        assert_eq!(five, XBUTTON2);
    }
}
