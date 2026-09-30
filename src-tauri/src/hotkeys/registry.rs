//! Owns the input hook and fans its events out to the hotkey matcher and,
//! while recording, to the recorder (§2.2).
//!
//! One hook, one consumer thread: the callback itself never does work, and
//! everything downstream reads from a single ordered stream.
//!
//! The consumer never does slow work either. Acting on a hotkey writes files
//! (history, run counters) and talks to the UI; doing that inline would stall
//! the very loop that has to notice the *next* key press, so actions are
//! handed to a separate thread and the consumer goes straight back to
//! reading (§61).

use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use parking_lot::Mutex;

use crate::hotkeys::matcher::{GlobalBindings, HotkeyAction, MacroBinding, Matcher};
use crate::input::hook::{self, HookEvent, HookHandle};

type ActionSink = Arc<dyn Fn(HotkeyAction) + Send + Sync>;

/// How often the held-key set is checked against the real keyboard. Short
/// enough that a lost key-up is repaired before the user's next press, cheap
/// enough to be irrelevant: it reads a handful of keys at most.
const RECONCILE_INTERVAL: Duration = Duration::from_millis(120);

pub struct Registry {
    matcher: Arc<Mutex<Matcher>>,
    recorder: Arc<Mutex<Option<Sender<HookEvent>>>>,
    hook: Option<HookHandle>,
    consumer: Option<JoinHandle<()>>,
    actions: Option<Sender<HotkeyAction>>,
    dispatcher: Option<JoinHandle<()>>,
}

/// `GetAsyncKeyState`'s high bit is the physical state of the key right now.
#[cfg(windows)]
fn key_is_down(vk: u16) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    (unsafe { GetAsyncKeyState(vk as i32) } as u16 & 0x8000) != 0
}

#[cfg(not(windows))]
fn key_is_down(_vk: u16) -> bool {
    true
}

#[cfg(windows)]
fn raise_thread_priority() {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_ABOVE_NORMAL,
    };
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_ABOVE_NORMAL);
    }
}

#[cfg(not(windows))]
fn raise_thread_priority() {}

impl Registry {
    /// Installs the hook and starts consuming events immediately.
    pub fn start(on_action: ActionSink) -> Self {
        let (hook_handle, events) = hook::start();
        let matcher = Arc::new(Mutex::new(Matcher::new()));
        let recorder: Arc<Mutex<Option<Sender<HookEvent>>>> = Arc::new(Mutex::new(None));

        let (action_tx, action_rx) = crossbeam_channel::unbounded::<HotkeyAction>();
        let dispatcher = thread::Builder::new()
            .name("flowmacro-hotkey".into())
            .spawn(move || {
                for action in action_rx.iter() {
                    on_action(action);
                }
            })
            .expect("spawn hotkey dispatcher");

        let consumer = {
            let matcher = Arc::clone(&matcher);
            let recorder = Arc::clone(&recorder);
            let action_tx = action_tx.clone();
            thread::Builder::new()
                .name("flowmacro-input".into())
                .spawn(move || {
                    raise_thread_priority();
                    let mut last_reconcile = Instant::now();

                    loop {
                        // The timeout is what keeps reconciliation running
                        // while the machine is idle; the elapsed check below
                        // covers the opposite case, a flood of mouse moves.
                        let event = match events.recv_timeout(RECONCILE_INTERVAL) {
                            Ok(event) => Some(event),
                            Err(crossbeam_channel::RecvTimeoutError::Timeout) => None,
                            // The hook thread dropped its sender: shutdown.
                            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                        };

                        if last_reconcile.elapsed() >= RECONCILE_INTERVAL {
                            matcher.lock().reconcile_held(&key_is_down);
                            last_reconcile = Instant::now();
                        }

                        let Some(event) = event else { continue };

                        // Falling behind can only mean a storm of mouse moves.
                        // Shed those; a key event is never worth dropping.
                        if matches!(event, HookEvent::MouseMove { .. })
                            && events.len() > hook::BACKLOG_SHED_AT
                        {
                            continue;
                        }

                        if let Some(sink) = recorder.lock().as_ref() {
                            let _ = sink.try_send(event);
                        }

                        // Mouse side buttons are bindable too, so they go
                        // through the same matcher as the keyboard (§18).
                        let actions = match event {
                            HookEvent::Key { vk, extended, down } => {
                                let actions =
                                    matcher.lock().on_key(vk, extended, down, Instant::now());
                                if crate::util::input_debug() {
                                    eprintln!(
                                        "[hook] vk={vk:#04x} ext={extended} down={down} -> {actions:?}"
                                    );
                                }
                                actions
                            }
                            HookEvent::MouseButton { button, down } => {
                                let actions = matcher.lock().on_mouse_button(button, down);
                                if crate::util::input_debug() {
                                    eprintln!("[hook] button={button:?} down={down} -> {actions:?}");
                                }
                                actions
                            }
                            _ => Vec::new(),
                        };
                        for action in actions {
                            let _ = action_tx.send(action);
                        }
                    }
                })
                .expect("spawn input consumer")
        };

        Self {
            matcher,
            recorder,
            hook: Some(hook_handle),
            consumer: Some(consumer),
            actions: Some(action_tx),
            dispatcher: Some(dispatcher),
        }
    }

    pub fn set_global(&self, bindings: GlobalBindings) {
        self.matcher.lock().set_global(bindings);
    }

    pub fn set_macros(&self, bindings: Vec<MacroBinding>) {
        self.matcher.lock().set_macros(bindings);
    }

    pub fn set_panic_enabled(&self, enabled: bool) {
        self.matcher.lock().set_panic_enabled(enabled);
    }

    /// Passing `None` detaches the recorder; the hook keeps running.
    pub fn set_recorder(&self, sink: Option<Sender<HookEvent>>) {
        *self.recorder.lock() = sink;
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        // Unhook first: that ends the hook thread, which drops the sender and
        // lets the consumer loop fall out on its own.
        if let Some(mut hook) = self.hook.take() {
            hook.stop();
        }
        if let Some(consumer) = self.consumer.take() {
            let _ = consumer.join();
        }
        // The consumer is gone, so no more actions can be queued; dropping the
        // sender ends the dispatcher once it has drained what is left.
        self.actions.take();
        if let Some(dispatcher) = self.dispatcher.take() {
            let _ = dispatcher.join();
        }
    }
}
