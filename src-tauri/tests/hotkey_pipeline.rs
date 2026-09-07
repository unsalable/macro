//! End-to-end check of the path a hotkey actually travels: real Windows
//! injection -> low level hook -> matcher -> action.
//!
//! The unit tests cover the matcher's rules on their own. What they cannot
//! show is the part that broke in the field: whether a keypress reaches the
//! matcher at all, and whether it keeps reaching it while the engine is
//! hammering `SendInput`. That only shows up against the real hook.
//!
//! Serialised into one test on purpose — two Registries at once would each
//! install their own global hook and see each other's traffic.
#![cfg(windows)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use flowmacro_lib::hotkeys::matcher::{GlobalBindings, HotkeyAction, MacroBinding};
use flowmacro_lib::hotkeys::registry::Registry;
use flowmacro_lib::input::inject;
use flowmacro_lib::macros::model::{Activation, Hotkey, MouseButton};

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
};

const VK_DELETE: u16 = 0x2E;
const SCAN_DELETE: u16 = 0x53;

/// A keypress with no signature on it: as far as the hook can tell, this is a
/// human pressing the key (and it is also how a remapper's output looks).
fn press_delete(hold: Duration) {
    send_delete(false);
    std::thread::sleep(hold);
    send_delete(true);
}

fn send_delete(up: bool) {
    let mut flags = KEYEVENTF_EXTENDEDKEY;
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(VK_DELETE),
                wScan: SCAN_DELETE,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
}

fn collect(actions: &Receiver<HotkeyAction>, within: Duration) -> Vec<HotkeyAction> {
    let deadline = Instant::now() + within;
    let mut seen = Vec::new();
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match actions.recv_timeout(left) {
            Ok(action) => seen.push(action),
            Err(_) => break,
        }
    }
    seen
}

fn registry() -> (Registry, Receiver<HotkeyAction>) {
    let (tx, rx) = crossbeam_channel::unbounded();
    let registry = Registry::start(Arc::new(move |action| {
        let _ = tx.send(action);
    }));
    registry.set_global(GlobalBindings::default());
    registry.set_macros(vec![MacroBinding {
        macro_id: "quick-click".into(),
        hotkey: Hotkey { code: "Delete".into(), modifiers: Vec::new() },
        activation: Activation::Toggle,
    }]);
    // Let the hook thread finish installing before anything is injected.
    std::thread::sleep(Duration::from_millis(300));
    (registry, rx)
}

#[test]
fn the_whole_hotkey_path_holds_up() {
    let (registry, actions) = registry();
    let toggle = HotkeyAction::MacroToggle("quick-click".into());

    // 1. A press reaches the matcher at all.
    press_delete(Duration::from_millis(30));
    assert_eq!(
        collect(&actions, Duration::from_millis(400)),
        vec![toggle.clone()],
        "a plain keypress did not reach the matcher"
    );

    // 2. Press it again: this is the stop, and it is the one the user reported
    //    as dead. One press, one toggle - never zero, never two.
    press_delete(Duration::from_millis(30));
    assert_eq!(
        collect(&actions, Duration::from_millis(400)),
        vec![toggle.clone()],
        "the second press did not toggle"
    );

    // 3. Ten presses in a row: every one lands.
    for round in 0..10 {
        press_delete(Duration::from_millis(15));
        assert_eq!(
            collect(&actions, Duration::from_millis(400)),
            vec![toggle.clone()],
            "press {round} was swallowed"
        );
    }

    // 4. Holding the key repeats it in Windows; only the first press counts.
    send_delete(false);
    for _ in 0..8 {
        std::thread::sleep(Duration::from_millis(30));
        send_delete(false);
    }
    send_delete(true);
    assert_eq!(
        collect(&actions, Duration::from_millis(400)),
        vec![toggle.clone()],
        "auto-repeat toggled more than once"
    );

    drop(registry);
}

#[test]
fn our_own_injection_never_triggers_our_own_hotkeys() {
    let (registry, actions) = registry();

    // What the engine does while a macro runs. It carries the signature, so
    // none of it may come back as a hotkey - otherwise a macro that clicks or
    // types could drive itself.
    for _ in 0..50 {
        let _ = inject::mouse_click(MouseButton::Mouse5);
        let _ = inject::key_press("Delete");
        std::thread::sleep(Duration::from_millis(2));
    }

    assert!(
        collect(&actions, Duration::from_millis(400)).is_empty(),
        "the engine's own injection came back as a hotkey"
    );

    drop(registry);
}

/// The failure the user hit: with a clicker running flat out, the stop key did
/// nothing. Here the load is real - 100 clicks a second going through the same
/// hook - and the key still has to land.
#[test]
fn a_press_still_lands_while_the_engine_floods_the_hook() {
    let (registry, actions) = registry();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let flood = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = inject::mouse_click(MouseButton::Mouse5);
                std::thread::sleep(Duration::from_millis(10));
            }
        })
    };

    std::thread::sleep(Duration::from_millis(200));
    for round in 0..5 {
        press_delete(Duration::from_millis(20));
        assert_eq!(
            collect(&actions, Duration::from_millis(600)),
            vec![HotkeyAction::MacroToggle("quick-click".into())],
            "press {round} was lost under load"
        );
        std::thread::sleep(Duration::from_millis(100));
    }

    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = flood.join();
    drop(registry);
}
