//! Engine state machine (§2.4).
//!
//! Stop is an atomic flag, not a message: the executor reads it on every spin
//! of its wait loop, so a stop request lands in well under a millisecond even
//! in the middle of a 100 CPS burst (§61). Pause parks the worker on a
//! condition variable so a paused engine costs no CPU at all.

use std::sync::atomic::{AtomicU8, Ordering};

use parking_lot::{Condvar, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineState {
    Idle,
    Running,
    Paused,
    Stopping,
}

impl EngineState {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Running,
            2 => Self::Paused,
            3 => Self::Stopping,
            _ => Self::Idle,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::Running => 1,
            Self::Paused => 2,
            Self::Stopping => 3,
        }
    }
}

#[derive(Debug, Default)]
pub struct Control {
    state: AtomicU8,
    parked: Mutex<()>,
    resumed: Condvar,
}

impl Control {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> EngineState {
        EngineState::from_u8(self.state.load(Ordering::Acquire))
    }

    fn swap_if(&self, from: EngineState, to: EngineState) -> bool {
        self.state
            .compare_exchange(from.as_u8(), to.as_u8(), Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Claims the engine for a new run. Fails if one is already in flight.
    pub fn begin(&self) -> bool {
        self.swap_if(EngineState::Idle, EngineState::Running)
    }

    pub fn pause(&self) -> bool {
        self.swap_if(EngineState::Running, EngineState::Paused)
    }

    pub fn resume(&self) -> bool {
        let changed = self.swap_if(EngineState::Paused, EngineState::Running);
        if changed {
            self.resumed.notify_all();
        }
        changed
    }

    /// Asks the worker to unwind. Also wakes a paused worker so it can notice.
    pub fn stop(&self) -> bool {
        let changed = self.swap_if(EngineState::Running, EngineState::Stopping)
            || self.swap_if(EngineState::Paused, EngineState::Stopping);
        if changed {
            self.resumed.notify_all();
        }
        changed
    }

    /// Called by the worker once it has fully unwound.
    pub fn finish(&self) {
        self.state.store(EngineState::Idle.as_u8(), Ordering::Release);
        self.resumed.notify_all();
    }

    pub fn should_abort(&self) -> bool {
        matches!(self.state(), EngineState::Stopping | EngineState::Idle)
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state(), EngineState::Running)
    }

    /// Blocks while paused. Returns `false` when the run should end instead of
    /// continuing — either a stop arrived, or the engine went idle.
    pub fn wait_while_paused(&self) -> bool {
        if self.state() != EngineState::Paused {
            return !self.should_abort();
        }

        let mut guard = self.parked.lock();
        while self.state() == EngineState::Paused {
            self.resumed.wait(&mut guard);
        }
        !self.should_abort()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn begin_claims_the_engine_only_once() {
        let control = Control::new();
        assert!(control.begin());
        assert!(!control.begin());
        assert_eq!(control.state(), EngineState::Running);
    }

    #[test]
    fn pause_and_resume_round_trip() {
        let control = Control::new();
        control.begin();
        assert!(control.pause());
        assert_eq!(control.state(), EngineState::Paused);
        assert!(!control.pause());
        assert!(control.resume());
        assert_eq!(control.state(), EngineState::Running);
    }

    #[test]
    fn stop_works_from_running_and_from_paused() {
        let control = Control::new();
        control.begin();
        assert!(control.stop());
        assert_eq!(control.state(), EngineState::Stopping);
        control.finish();

        control.begin();
        control.pause();
        assert!(control.stop());
        assert_eq!(control.state(), EngineState::Stopping);
    }

    #[test]
    fn stop_is_ignored_when_idle() {
        let control = Control::new();
        assert!(!control.stop());
        assert_eq!(control.state(), EngineState::Idle);
    }

    #[test]
    fn a_paused_worker_wakes_on_resume() {
        let control = Arc::new(Control::new());
        control.begin();
        control.pause();

        let worker = {
            let control = Arc::clone(&control);
            std::thread::spawn(move || control.wait_while_paused())
        };

        std::thread::sleep(Duration::from_millis(30));
        assert!(!worker.is_finished(), "worker should still be parked");
        control.resume();

        assert!(worker.join().expect("join"), "resume should continue the run");
    }

    #[test]
    fn a_paused_worker_wakes_on_stop_and_reports_abort() {
        let control = Arc::new(Control::new());
        control.begin();
        control.pause();

        let worker = {
            let control = Arc::clone(&control);
            std::thread::spawn(move || control.wait_while_paused())
        };

        std::thread::sleep(Duration::from_millis(30));
        control.stop();

        assert!(!worker.join().expect("join"), "stop should end the run");
    }
}
