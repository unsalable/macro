//! Global low-level keyboard and mouse hooks (§2.2).
//!
//! The callback runs on Windows' own timetable: if it takes too long the OS
//! silently unhooks us. So it does the bare minimum — decode the struct, drop
//! anything we injected ourselves, push to the queue — and returns.
//! All real work happens on the consumer thread.
//!
//! Two things protect the hook from the one failure that has no error code:
//! Windows removing it because the callback did not answer inside
//! `LowLevelHooksTimeout` (300 ms by default). That happens when the hook
//! thread cannot get scheduled — exactly the situation a 100 CPS run on a
//! loaded machine creates. So the thread runs at time-critical priority, and
//! a watchdog compares the last event we saw against the system's own last
//! input time and reinstalls the hooks when they have gone deaf (§61).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crossbeam_channel::Sender;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, KillTimer, PostThreadMessageW, SetTimer, SetWindowsHookExW,
    UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL,
    WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WM_TIMER, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

use crate::input::inject;
use crate::macros::model::MouseButton;

const LLKHF_EXTENDED: u32 = 0x01;

/// The queue is unbounded on purpose. A bounded one drops whatever arrives
/// while it is full, and the event most likely to be dropped during a busy
/// run — a key-up — is the one that must never be lost: the matcher would go
/// on believing the key is held and treat every later press as auto-repeat,
/// which is precisely the "the stop key does nothing" failure. Backlog is
/// handled the other way round: the consumer sheds mouse moves when it falls
/// behind (`BACKLOG_SHED_AT`).
pub const BACKLOG_SHED_AT: usize = 512;

/// How often the watchdog looks at whether the hooks are still being called.
const WATCHDOG_INTERVAL_MS: u32 = 1_000;

/// The hooks count as deaf when the system has seen input this much more
/// recently than our callback has. Comfortably above one watchdog tick so a
/// scheduling hiccup alone never triggers a reinstall.
const DEAF_AFTER_MS: u32 = 1_500;

const WATCHDOG_TIMER_ID: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    Key { vk: u16, extended: bool, down: bool },
    MouseButton { button: MouseButton, down: bool },
    MouseMove { x: i32, y: i32 },
    MouseWheel { clicks: i32 },
}

impl HookEvent {
    /// Key events drive the hotkeys, so they are the ones that must never be
    /// dropped: a lost key-up leaves the matcher believing the key is still
    /// held, and every later press then looks like auto-repeat.
    #[cfg(test)]
    fn is_key(&self) -> bool {
        matches!(self, HookEvent::Key { .. })
    }
}

/// Tick (from `GetTickCount`) of the last event either hook callback saw,
/// counted before any filtering so our own injection keeps it fresh too.
static LAST_HOOK_TICK: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static SINK: std::cell::RefCell<Option<Sender<HookEvent>>> =
        const { std::cell::RefCell::new(None) };
}

fn tick_now() -> u32 {
    unsafe { windows::Win32::System::SystemInformation::GetTickCount() }
}

fn emit(event: HookEvent) {
    SINK.with(|sink| {
        if let Some(sender) = sink.borrow().as_ref() {
            // Never blocks: the queue is unbounded, so this only fails once
            // the consumer is gone, which happens at shutdown.
            let _ = sender.try_send(event);
        }
    });
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        LAST_HOOK_TICK.store(tick_now(), Ordering::Relaxed);
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        // Skip our own SendInput traffic, otherwise a macro that types a key
        // bound to a hotkey would drive itself (§2.2). Input injected by
        // *other* software still counts: remappers are how a lot of people
        // press keys.
        if info.dwExtraInfo != inject::SIGNATURE {
            let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            let up = matches!(wparam.0 as u32, WM_KEYUP | WM_SYSKEYUP);
            if down || up {
                emit(HookEvent::Key {
                    vk: info.vkCode as u16,
                    extended: info.flags.0 & LLKHF_EXTENDED != 0,
                    down,
                });
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        LAST_HOOK_TICK.store(tick_now(), Ordering::Relaxed);
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        if info.dwExtraInfo != inject::SIGNATURE {
            let high_word = ((info.mouseData >> 16) & 0xFFFF) as u16;
            let event = match wparam.0 as u32 {
                WM_LBUTTONDOWN => Some(HookEvent::MouseButton {
                    button: MouseButton::Left,
                    down: true,
                }),
                WM_LBUTTONUP => Some(HookEvent::MouseButton {
                    button: MouseButton::Left,
                    down: false,
                }),
                WM_RBUTTONDOWN => Some(HookEvent::MouseButton {
                    button: MouseButton::Right,
                    down: true,
                }),
                WM_RBUTTONUP => Some(HookEvent::MouseButton {
                    button: MouseButton::Right,
                    down: false,
                }),
                WM_MBUTTONDOWN => Some(HookEvent::MouseButton {
                    button: MouseButton::Middle,
                    down: true,
                }),
                WM_MBUTTONUP => Some(HookEvent::MouseButton {
                    button: MouseButton::Middle,
                    down: false,
                }),
                WM_XBUTTONDOWN => Some(HookEvent::MouseButton {
                    button: x_button(high_word),
                    down: true,
                }),
                WM_XBUTTONUP => Some(HookEvent::MouseButton {
                    button: x_button(high_word),
                    down: false,
                }),
                WM_MOUSEMOVE => Some(HookEvent::MouseMove {
                    x: info.pt.x,
                    y: info.pt.y,
                }),
                WM_MOUSEWHEEL => Some(HookEvent::MouseWheel {
                    clicks: (high_word as i16) as i32 / 120,
                }),
                _ => None,
            };
            if let Some(event) = event {
                emit(event);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn x_button(high_word: u16) -> MouseButton {
    if high_word == 0x0002 {
        MouseButton::Mouse5
    } else {
        MouseButton::Mouse4
    }
}

/// Owns the hook thread. Dropping it unhooks and joins.
pub struct HookHandle {
    thread_id: Arc<AtomicU32>,
    join: Option<JoinHandle<()>>,
}

impl HookHandle {
    pub fn stop(&mut self) {
        let id = self.thread_id.load(Ordering::Acquire);
        if id != 0 {
            unsafe {
                let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for HookHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Installs both hooks on a dedicated thread with its own message loop.
pub fn start() -> (HookHandle, crossbeam_channel::Receiver<HookEvent>) {
    let (sender, receiver) = crossbeam_channel::unbounded();
    let thread_id = Arc::new(AtomicU32::new(0));
    let thread_id_for_worker = Arc::clone(&thread_id);

    let join = thread::Builder::new()
        .name("flowmacro-hook".into())
        .spawn(move || run_hook_thread(sender, thread_id_for_worker))
        .expect("spawn hook thread");

    (
        HookHandle {
            thread_id,
            join: Some(join),
        },
        receiver,
    )
}

struct Hooks {
    keyboard: Option<HHOOK>,
    mouse: Option<HHOOK>,
}

impl Hooks {
    fn install() -> Self {
        let keyboard = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) };
        let mouse = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0) };
        LAST_HOOK_TICK.store(tick_now(), Ordering::Relaxed);
        Self {
            keyboard: keyboard.ok(),
            mouse: mouse.ok(),
        }
    }

    fn any(&self) -> bool {
        self.keyboard.is_some() || self.mouse.is_some()
    }

    fn remove(&mut self) {
        for handle in [self.keyboard.take(), self.mouse.take()].into_iter().flatten() {
            unsafe {
                let _ = UnhookWindowsHookEx(handle);
            }
        }
    }
}

/// True when the system has registered input more recently than our callbacks
/// have — the only observable symptom of Windows having dropped the hooks.
fn hooks_look_deaf() -> bool {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if !unsafe { GetLastInputInfo(&mut info) }.as_bool() {
        return false;
    }

    let seen = LAST_HOOK_TICK.load(Ordering::Relaxed);
    // Both counters wrap after 49 days; the wrapping difference stays correct.
    let system_input_age = unsafe { GetTickCount() }.wrapping_sub(info.dwTime);
    let gap = info.dwTime.wrapping_sub(seen);

    // Ignore a stale reading: only recent system input proves we should have
    // been called.
    system_input_age < DEAF_AFTER_MS && gap > DEAF_AFTER_MS && gap < i32::MAX as u32
}

fn raise_thread_priority() {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
    };
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
    }
}

fn run_hook_thread(sender: Sender<HookEvent>, thread_id: Arc<AtomicU32>) {
    use windows::Win32::System::Threading::GetCurrentThreadId;

    SINK.with(|sink| *sink.borrow_mut() = Some(sender));
    thread_id.store(unsafe { GetCurrentThreadId() }, Ordering::Release);
    // The callback has ~300 ms to answer or Windows unhooks us without a word.
    // Nothing else on this thread does real work, so it can afford to win
    // every scheduling contest.
    raise_thread_priority();

    let mut hooks = Hooks::install();
    if crate::util::input_debug() {
        eprintln!(
            "[hookthread] installed keyboard={} mouse={}",
            hooks.keyboard.is_some(),
            hooks.mouse.is_some()
        );
    }
    let timer = unsafe { SetTimer(None, WATCHDOG_TIMER_ID, WATCHDOG_INTERVAL_MS, None) };

    let mut message = MSG::default();
    // GetMessageW returns 0 on WM_QUIT and -1 on error. `BOOL::as_bool()`
    // is true for -1, so testing it would spin forever on an error.
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        if message.message == WM_TIMER && (hooks_look_deaf() || !hooks.any()) {
            hooks.remove();
            hooks = Hooks::install();
            if crate::util::input_debug() {
                eprintln!("[hookthread] reinstalled after going deaf");
            }
        }
    }

    if timer != 0 {
        unsafe {
            let _ = KillTimer(None, WATCHDOG_TIMER_ID);
        }
    }
    hooks.remove();
    SINK.with(|sink| *sink.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_button_high_word_maps_to_side_buttons() {
        assert_eq!(x_button(0x0001), MouseButton::Mouse4);
        assert_eq!(x_button(0x0002), MouseButton::Mouse5);
    }

    #[test]
    fn wheel_delta_converts_to_click_counts() {
        let up = (120i16) as i32 / 120;
        let down = (-240i16) as i32 / 120;
        assert_eq!(up, 1);
        assert_eq!(down, -2);
    }

    #[test]
    fn key_events_are_the_ones_kept_when_the_queue_overflows() {
        assert!(HookEvent::Key { vk: 0x2E, extended: true, down: false }.is_key());
        assert!(!HookEvent::MouseMove { x: 0, y: 0 }.is_key());
    }

    /// A backlog of mouse moves must never cost us a key event — losing one
    /// key-up is what leaves a hotkey permanently dead.
    #[test]
    fn a_deep_backlog_still_delivers_every_key_event() {
        let (sender, receiver) = crossbeam_channel::unbounded::<HookEvent>();
        SINK.with(|sink| *sink.borrow_mut() = Some(sender));

        for step in 0..10_000 {
            emit(HookEvent::MouseMove { x: step, y: step });
        }
        emit(HookEvent::Key { vk: 0x2E, extended: true, down: true });
        emit(HookEvent::Key { vk: 0x2E, extended: true, down: false });

        let keys: Vec<_> = receiver.try_iter().filter(HookEvent::is_key).collect();
        assert_eq!(
            keys,
            vec![
                HookEvent::Key { vk: 0x2E, extended: true, down: true },
                HookEvent::Key { vk: 0x2E, extended: true, down: false },
            ]
        );
        SINK.with(|sink| *sink.borrow_mut() = None);
    }
}
