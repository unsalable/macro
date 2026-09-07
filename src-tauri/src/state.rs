//! Shared application state and the glue between the engine, the hook and the
//! UI. Commands stay thin: everything with a decision in it lives here.

use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{profiles, settings::AppSettings};
use crate::hotkeys::matcher::{GlobalBindings, HotkeyAction, MacroBinding};
use crate::hotkeys::registry::Registry;
use crate::macros::control::EngineState;
use crate::macros::engine::{Engine, EngineListener, EngineStatus, RunOutcome};
use crate::macros::model::{Macro, ProfileStore};
use crate::recorder::capture::Recorder;
use crate::recorder::convert::RecordedEvent;
use crate::storage::history::{self, HistoryEntry, HistoryEventKind};

pub const EVENT_STATUS: &str = "engine:status";
pub const EVENT_FINISHED: &str = "engine:finished";
pub const EVENT_ERROR: &str = "engine:error";
pub const EVENT_RECORDED: &str = "recorder:event";
pub const EVENT_HOTKEY: &str = "hotkey:triggered";
pub const EVENT_ARMED: &str = "hotkey:armed";
pub const EVENT_DATA_CHANGED: &str = "data:changed";

struct UiListener {
    app: AppHandle,
}

impl EngineListener for UiListener {
    fn on_status(&self, status: &EngineStatus) {
        let _ = self.app.emit(EVENT_STATUS, status);
    }

    fn on_finished(&self, outcome: &RunOutcome) {
        let _ = self.app.emit(EVENT_FINISHED, outcome);
        if let Some(message) = &outcome.error {
            let _ = self.app.emit(EVENT_ERROR, message);
        }
        // The engine is built before `manage`, so the state may legitimately
        // not be there yet on a very early run.
        if let Some(state) = self.app.try_state::<AppState>() {
            state.record_run_finished(outcome);
        }
    }
}

pub struct AppState {
    app: AppHandle,
    pub engine: Arc<Engine>,
    pub recorder: Recorder,
    pub settings: Mutex<AppSettings>,
    pub profiles: Mutex<ProfileStore>,
    registry: Mutex<Option<Registry>>,
    /// The macro the user has put under hotkey control from the window.
    /// Arming runs nothing by itself: it decides what a bare Start hotkey
    /// means, and lets the UI show which key is live (19).
    armed: Mutex<Option<String>>,
}

impl AppState {
    pub fn load(app: AppHandle) -> Self {
        let settings = crate::config::settings::load().unwrap_or_default();
        let store = profiles::load().unwrap_or_default();
        let engine = Arc::new(Engine::new(Arc::new(UiListener { app: app.clone() })));

        let recorder = {
            let app = app.clone();
            Recorder::new(Arc::new(move |event: &RecordedEvent| {
                let _ = app.emit(EVENT_RECORDED, event);
            }))
        };

        Self {
            app,
            engine,
            recorder,
            settings: Mutex::new(settings),
            profiles: Mutex::new(store),
            registry: Mutex::new(None),
            armed: Mutex::new(None),
        }
    }

    pub fn attach_registry(&self, registry: Registry) {
        *self.registry.lock() = Some(registry);
        self.sync_bindings();
    }

    /// Pushes the current hotkey configuration into the matcher. Called after
    /// anything that can change a binding.
    pub fn sync_bindings(&self) {
        let guard = self.registry.lock();
        let Some(registry) = guard.as_ref() else { return };

        let settings = self.settings.lock();
        registry.set_global(GlobalBindings {
            start: settings.hotkeys.start.clone(),
            stop: settings.hotkeys.stop.clone(),
            pause: settings.hotkeys.pause.clone(),
            emergency_stop: settings.hotkeys.emergency_stop.clone(),
        });
        registry.set_panic_enabled(settings.panic_escape_enabled);
        drop(settings);

        let store = self.profiles.lock();
        let bindings = profiles::active_profile(&store)
            .map(|profile| {
                profile
                    .macros
                    .iter()
                    .filter(|item| item.enabled)
                    .filter_map(|item| {
                        item.hotkey.as_ref().map(|hotkey| MacroBinding {
                            macro_id: item.id.clone(),
                            hotkey: hotkey.clone(),
                            activation: item.activation,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        registry.set_macros(bindings);
    }

    pub fn start_recording(&self) -> bool {
        let guard = self.registry.lock();
        match guard.as_ref() {
            Some(registry) => self.recorder.start(registry),
            None => false,
        }
    }

    pub fn stop_recording(&self) -> Vec<RecordedEvent> {
        let guard = self.registry.lock();
        match guard.as_ref() {
            Some(registry) => self.recorder.stop(registry),
            None => Vec::new(),
        }
    }

    pub fn find_macro(&self, macro_id: &str) -> Option<Macro> {
        let store = self.profiles.lock();
        profiles::find_macro(&store, macro_id).cloned()
    }

    // ------------------------------------------------------------- arming

    pub fn armed_macro(&self) -> Option<String> {
        self.armed.lock().clone()
    }

    /// Puts a macro under hotkey control. Returns false when it could not run
    /// anyway, so the window never shows a key as live while pressing it would
    /// do nothing.
    pub fn arm(&self, macro_id: &str) -> bool {
        let ok = self
            .find_macro(macro_id)
            .is_some_and(|value| value.enabled && !value.actions.is_empty());
        if ok {
            *self.armed.lock() = Some(macro_id.to_owned());
            self.emit_armed();
        }
        ok
    }

    pub fn disarm(&self) {
        let had = self.armed.lock().take().is_some();
        if had {
            self.emit_armed();
        }
    }

    fn emit_armed(&self) {
        let _ = self.app.emit(EVENT_ARMED, self.armed_macro());
    }

    /// The macro a bare Start hotkey should run: the armed one, else the most
    /// recently used enabled macro in the active profile.
    pub fn default_macro(&self) -> Option<Macro> {
        if let Some(armed) = self.armed_macro() {
            if let Some(value) = self.find_macro(&armed) {
                if value.enabled && !value.actions.is_empty() {
                    return Some(value);
                }
            }
        }

        let store = self.profiles.lock();
        let profile = profiles::active_profile(&store)?;
        profile
            .macros
            .iter()
            .filter(|item| item.enabled && !item.actions.is_empty())
            .max_by(|a, b| a.stats.last_run_at.cmp(&b.stats.last_run_at))
            .cloned()
    }

    pub fn start_macro(&self, macro_id: &str) -> Result<(), String> {
        let Some(value) = self.find_macro(macro_id) else {
            return Err("That macro no longer exists".into());
        };
        let name = value.name.clone();

        // A run that is still unwinding would reject this one as busy, and the
        // press would have done nothing at all as far as the user can tell.
        // Unwinding takes well under a millisecond, so waiting for it is
        // invisible - and it is what makes a quick stop/start pair on the same
        // key reliable (61).
        self.wait_for_idle(Duration::from_millis(600));
        self.engine.start(value).map_err(|error| error.to_string())?;

        self.record_run_started(macro_id, &name);
        Ok(())
    }

    fn wait_for_idle(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while self.engine.state() == EngineState::Stopping {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        true
    }

    fn record_run_started(&self, macro_id: &str, macro_name: &str) {
        let limit = self.settings.lock().history_limit;
        let entry = HistoryEntry::new(macro_id, macro_name, HistoryEventKind::Started);
        let _ = history::append(&entry, limit);

        let mut store = self.profiles.lock();
        for profile in store.profiles.iter_mut() {
            if let Some(item) = profile.macros.iter_mut().find(|item| item.id == macro_id) {
                item.stats.run_count += 1;
                item.stats.last_run_at = Some(crate::util::now_iso());
            }
        }
        let _ = profiles::save(&store);
        drop(store);

        let _ = self.app.emit(EVENT_DATA_CHANGED, ());
    }

    pub fn record_run_finished(&self, outcome: &RunOutcome) {
        let limit = self.settings.lock().history_limit;
        let event = if outcome.error.is_some() {
            HistoryEventKind::Failed
        } else if outcome.completed {
            HistoryEventKind::Completed
        } else {
            HistoryEventKind::Stopped
        };

        let mut entry = HistoryEntry::new(&outcome.macro_id, &outcome.macro_name, event)
            .with_run(outcome.iterations, outcome.duration_ms);
        if let Some(message) = &outcome.error {
            entry = entry.with_detail(message.clone());
        }
        let _ = history::append(&entry, limit);
    }

    pub fn handle_hotkey(&self, action: HotkeyAction) {
        if crate::util::input_debug() {
            eprintln!("[hotkey] {action:?} state={:?}", self.engine.state());
        }
        let _ = self.app.emit(EVENT_HOTKEY, format!("{action:?}"));

        match action {
            HotkeyAction::Start => {
                if self.engine.state() == EngineState::Idle {
                    if let Some(value) = self.default_macro() {
                        let _ = self.start_macro(&value.id);
                    }
                }
            }
            HotkeyAction::Stop | HotkeyAction::EmergencyStop => {
                self.engine.stop();
            }
            HotkeyAction::Pause => {
                if self.engine.state() == EngineState::Paused {
                    self.engine.resume();
                } else {
                    self.engine.pause();
                }
            }
            HotkeyAction::MacroToggle(macro_id) => match self.engine.state() {
                // Anything running stops, even when it is not the macro this
                // key belongs to. The old rule ignored the press in that case,
                // which left the user hammering a key that could not possibly
                // work while something else clicked away (61).
                EngineState::Running | EngineState::Paused => {
                    self.engine.stop();
                }
                // Already on its way down: a second press must not queue a run.
                EngineState::Stopping => {}
                EngineState::Idle => {
                    let _ = self.start_macro(&macro_id);
                }
            },
            HotkeyAction::MacroHoldStart(macro_id) => {
                if self.engine.state() == EngineState::Idle {
                    let _ = self.start_macro(&macro_id);
                }
            }
            HotkeyAction::MacroHoldEnd(macro_id) => {
                let status = self.engine.status();
                // Release stops the run this key owns. The id can already be
                // gone if the run ended on its own, in which case stopping is
                // a no-op anyway.
                if status.macro_id.as_deref() == Some(macro_id.as_str())
                    || status.macro_id.is_none()
                {
                    self.engine.stop();
                }
            }
        }
    }

    pub fn notify_data_changed(&self) {
        let _ = self.app.emit(EVENT_DATA_CHANGED, ());
    }
}
