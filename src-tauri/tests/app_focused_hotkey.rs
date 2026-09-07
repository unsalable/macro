//! Drives the *running* FlowMacro window the way the user does: bring it to
//! the front, press the macro's hotkey to start the clicker, let it run into
//! that same focused window for a while, then press the key again to stop —
//! and read the result out of the app's own run history.
//!
//! This is the scenario that was reported broken: with the app's own window in
//! front, the stop key never took. The clicker is bound to mouse 5, which the
//! webview reads as "history forward", so every click used to navigate the
//! page; a hundred of those a second starves the machine, and a starved hook
//! thread is one Windows removes without telling anyone.
//!
//! The test process installs a low level hook of its own before injecting.
//! That is not decoration: without it, this environment's injected input never
//! reaches *other* processes' hooks, and the test reports a failure that a
//! human at the keyboard would never see.
//!
//! Ignored by default because it needs a running app. Run it with:
//!   cargo test --test app_focused_hotkey -- --ignored --nocapture
#![cfg(windows)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use flowmacro_lib::hotkeys::matcher::MacroBinding;
use flowmacro_lib::hotkeys::registry::Registry;
use flowmacro_lib::macros::model::{Activation, Hotkey};
use windows::core::PCWSTR;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, SetActiveWindow, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VIRTUAL_KEY,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetForegroundWindow, SetForegroundWindow,
};

const VK_DELETE: u16 = 0x2E;
const SCAN_DELETE: u16 = 0x53;

fn history_path() -> PathBuf {
    PathBuf::from(std::env::var("APPDATA").expect("APPDATA"))
        .join("FlowMacro")
        .join("history.jsonl")
}

fn history_lines() -> Vec<String> {
    std::fs::read_to_string(history_path())
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
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
                wVk: VIRTUAL_KEY(VK_DELETE),
                wScan: SCAN_DELETE,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    assert_eq!(sent, 1, "Windows refused the injected keypress");
}

fn press_delete() {
    send_delete(false);
    std::thread::sleep(Duration::from_millis(40));
    send_delete(true);
}

/// Puts this process in the hook chain, so injected presses are dispatched to
/// every other hook the way a real key press is.
fn join_the_hook_chain() -> Registry {
    let registry = Registry::start(Arc::new(|_action| {}));
    registry.set_macros(vec![MacroBinding {
        macro_id: "probe".into(),
        hotkey: Hotkey { code: "Delete".into(), modifiers: Vec::new() },
        activation: Activation::Toggle,
    }]);
    std::thread::sleep(Duration::from_millis(300));
    registry
}

fn focus_app() -> HWND {
    let title: Vec<u16> = "FlowMacro\0".encode_utf16().collect();
    let window = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) }
        .expect("FlowMacro window not found - start the app first");

    // Windows only lets a process take the foreground under conditions a test
    // runner does not always meet. Tapping a key first lifts that lock, and a
    // couple of retries cover the rest.
    for _ in 0..5 {
        tap_alt();
        unsafe {
            let _ = SetForegroundWindow(window);
            let _ = SetActiveWindow(window);
        }
        std::thread::sleep(Duration::from_millis(400));
        if unsafe { GetForegroundWindow() } == window {
            return window;
        }
    }
    panic!("could not put the app window in front");
}

fn tap_alt() {
    for up in [false, true] {
        let mut flags = KEYBD_EVENT_FLAGS(0);
        if up {
            flags |= KEYEVENTF_KEYUP;
        }
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0x12),
                    wScan: 0x38,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    }
    std::thread::sleep(Duration::from_millis(60));
}

fn events_since(before: usize) -> Vec<String> {
    history_lines()
        .into_iter()
        .skip(before)
        .map(|line| {
            for kind in ["started", "stopped", "completed", "failed"] {
                if line.contains(&format!("\"event\":\"{kind}\"")) {
                    return kind.to_owned();
                }
            }
            format!("?? {line}")
        })
        .collect()
}

#[test]
#[ignore = "needs FlowMacro running"]
fn the_hotkey_starts_and_stops_while_the_app_window_is_focused() {
    let _chain = join_the_hook_chain();
    focus_app();

    let before = history_lines().len();

    println!("press 1 - start");
    press_delete();
    std::thread::sleep(Duration::from_millis(1_200));

    println!("press 2 - stop");
    press_delete();
    std::thread::sleep(Duration::from_millis(800));

    let fresh = events_since(before);
    println!("history: {fresh:?}");
    assert_eq!(fresh, vec!["started", "stopped"], "one press each way, nothing else");
}

/// The long version: the clicker runs for ten seconds into the app's own
/// window before the stop key is pressed. That is the state the hook used to
/// die in.
#[test]
#[ignore = "needs FlowMacro running"]
fn the_stop_key_still_works_after_a_long_run_into_the_apps_own_window() {
    let _chain = join_the_hook_chain();
    focus_app();

    let before = history_lines().len();

    println!("starting the clicker");
    press_delete();
    std::thread::sleep(Duration::from_secs(10));

    println!("ten seconds of clicking later: stopping");
    press_delete();
    std::thread::sleep(Duration::from_millis(800));

    let fresh = events_since(before);
    println!("history: {fresh:?}");
    assert_eq!(fresh, vec!["started", "stopped"], "the stop key did not take");
}

/// Ten start/stop pairs back to back. This is the "sometimes it just does not
/// react" complaint: a press that lands while the previous run is still
/// unwinding used to be swallowed.
#[test]
#[ignore = "needs FlowMacro running"]
fn ten_start_stop_pairs_in_a_row_all_land() {
    let _chain = join_the_hook_chain();
    focus_app();

    let before = history_lines().len();

    for round in 0..10 {
        press_delete();
        std::thread::sleep(Duration::from_millis(250));
        press_delete();
        std::thread::sleep(Duration::from_millis(250));
        println!("round {round} done");
    }
    std::thread::sleep(Duration::from_millis(800));

    let fresh = events_since(before);
    println!("history: {fresh:?}");
    let expected: Vec<String> = (0..10)
        .flat_map(|_| ["started".to_owned(), "stopped".to_owned()])
        .collect();
    assert_eq!(fresh, expected, "every press must toggle exactly once");
}
