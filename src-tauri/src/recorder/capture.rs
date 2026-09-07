//! Live recording session (§16).
//!
//! The registry already owns the hook, so recording is just a second consumer
//! of the same stream: start attaches a channel, stop detaches it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use parking_lot::Mutex;

use crate::hotkeys::registry::Registry;
use crate::input::hook::HookEvent;
use crate::input::keycodes;
use crate::recorder::convert::{Phase, RecordedEvent, RecordedKind};

const CHANNEL_CAPACITY: usize = 8_192;

type EventSink = Arc<dyn Fn(&RecordedEvent) + Send + Sync>;

struct Session {
    started: Instant,
    events: Arc<Mutex<Vec<RecordedEvent>>>,
    collector: Option<JoinHandle<()>>,
}

pub struct Recorder {
    session: Mutex<Option<Session>>,
    recording: AtomicBool,
    sink: EventSink,
}

impl Recorder {
    pub fn new(sink: EventSink) -> Self {
        Self {
            session: Mutex::new(None),
            recording: AtomicBool::new(false),
            sink,
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::Acquire)
    }

    /// Returns false if a recording is already in progress.
    pub fn start(&self, registry: &Registry) -> bool {
        let mut slot = self.session.lock();
        if slot.is_some() {
            return false;
        }

        let (sender, receiver) = crossbeam_channel::bounded::<HookEvent>(CHANNEL_CAPACITY);
        let events: Arc<Mutex<Vec<RecordedEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let started = Instant::now();

        let collector = {
            let events = Arc::clone(&events);
            let sink = Arc::clone(&self.sink);
            thread::Builder::new()
                .name("flowmacro-recorder".into())
                .spawn(move || {
                    // Ends when `stop` drops the sender.
                    for raw in receiver.iter() {
                        let at = started.elapsed().as_millis() as u64;
                        let Some(kind) = translate(raw) else { continue };
                        let event = RecordedEvent { at, kind };

                        // Moves are stored but not streamed: they arrive by the
                        // hundred and would drown the live timeline (§3).
                        if !matches!(event.kind, RecordedKind::MouseMove { .. }) {
                            sink(&event);
                        }
                        events.lock().push(event);
                    }
                })
                .expect("spawn recorder thread")
        };

        registry.set_recorder(Some(sender));
        *slot = Some(Session { started, events, collector: Some(collector) });
        self.recording.store(true, Ordering::Release);
        true
    }

    pub fn stop(&self, registry: &Registry) -> Vec<RecordedEvent> {
        let mut slot = self.session.lock();
        let Some(mut session) = slot.take() else {
            return Vec::new();
        };

        // Detaching drops the only sender, which ends the collector loop.
        registry.set_recorder(None);
        self.recording.store(false, Ordering::Release);
        if let Some(collector) = session.collector.take() {
            let _ = collector.join();
        }

        let _ = session.started;
        let events = session.events.lock().clone();
        events
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.session
            .lock()
            .as_ref()
            .map(|session| session.started.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }
}

fn translate(raw: HookEvent) -> Option<RecordedKind> {
    match raw {
        HookEvent::Key { vk, extended, down } => keycodes::name_for_vk(vk, extended).map(|code| {
            RecordedKind::Key {
                action: if down { Phase::Down } else { Phase::Up },
                code: code.to_owned(),
            }
        }),
        HookEvent::MouseButton { button, down } => Some(RecordedKind::MouseButton {
            action: if down { Phase::Down } else { Phase::Up },
            button,
        }),
        HookEvent::MouseMove { x, y } => Some(RecordedKind::MouseMove { x, y }),
        HookEvent::MouseWheel { clicks } if clicks != 0 => Some(RecordedKind::MouseScroll {
            direction: if clicks > 0 {
                crate::macros::model::ScrollDirection::Up
            } else {
                crate::macros::model::ScrollDirection::Down
            },
            amount: clicks.unsigned_abs(),
        }),
        HookEvent::MouseWheel { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::model::{MouseButton, ScrollDirection};

    #[test]
    fn key_events_translate_to_code_names() {
        let kind = translate(HookEvent::Key { vk: 0x41, extended: false, down: true });
        match kind {
            Some(RecordedKind::Key { action, code }) => {
                assert_eq!(action, Phase::Down);
                assert_eq!(code, "KeyA");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn an_unknown_virtual_key_is_dropped() {
        assert!(translate(HookEvent::Key { vk: 0xFE, extended: false, down: true }).is_none());
    }

    #[test]
    fn wheel_direction_follows_the_sign() {
        match translate(HookEvent::MouseWheel { clicks: -3 }) {
            Some(RecordedKind::MouseScroll { direction, amount }) => {
                assert_eq!(direction, ScrollDirection::Down);
                assert_eq!(amount, 3);
            }
            other => panic!("unexpected: {other:?}"),
        }
        assert!(translate(HookEvent::MouseWheel { clicks: 0 }).is_none());
    }

    #[test]
    fn side_buttons_survive_translation() {
        match translate(HookEvent::MouseButton { button: MouseButton::Mouse5, down: false }) {
            Some(RecordedKind::MouseButton { action, button }) => {
                assert_eq!(action, Phase::Up);
                assert_eq!(button, MouseButton::Mouse5);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}
