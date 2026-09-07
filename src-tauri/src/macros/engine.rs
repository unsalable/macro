//! Macro worker thread and the handle the UI talks to (§2.4).

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::Serialize;

use crate::input::timing::{self, TimerResolution};
use crate::macros::control::{Control, EngineState};
use crate::macros::executor::{run_actions, ExecContext, Flow, Metrics};
use crate::macros::model::{LoopMode, Macro};
use crate::macros::scheduler::{humanize, iteration_budget};

/// Status is pushed at 10 Hz, not on every click: at 100 CPS a per-event
/// stream would re-render the UI a hundred times a second for no gain (§3).
const STATUS_INTERVAL: Duration = Duration::from_millis(100);

/// Floor between two loop iterations. A macro whose actions carry no delay at
/// all (one click, repeat forever) would otherwise spin the CPU and flood the
/// system input queue faster than Windows can drain it — which reads to the
/// user as the whole desktop lagging. 250 us still allows 4000 iterations a
/// second, far above anything a real macro needs.
const MIN_ITERATION_GAP: Duration = Duration::from_micros(250);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub state: EngineState,
    pub macro_id: Option<String>,
    pub iterations: u64,
    pub action_index: i32,
    pub elapsed_ms: u64,
    pub actual_cps: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunOutcome {
    pub macro_id: String,
    pub macro_name: String,
    pub iterations: u64,
    pub duration_ms: u64,
    pub completed: bool,
    pub error: Option<String>,
}

pub trait EngineListener: Send + Sync + 'static {
    fn on_status(&self, status: &EngineStatus);
    fn on_finished(&self, outcome: &RunOutcome);
}

/// Listener used in tests and before the UI is wired up.
pub struct SilentListener;

impl EngineListener for SilentListener {
    fn on_status(&self, _status: &EngineStatus) {}
    fn on_finished(&self, _outcome: &RunOutcome) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    #[error("A macro is already running")]
    Busy,
    #[error("This macro has no actions")]
    Empty,
}

struct CurrentRun {
    macro_id: String,
    started: Instant,
}

pub struct Engine {
    control: Arc<Control>,
    metrics: Arc<Metrics>,
    current: Arc<Mutex<Option<CurrentRun>>>,
    listener: Arc<dyn EngineListener>,
}

impl Engine {
    pub fn new(listener: Arc<dyn EngineListener>) -> Self {
        Self {
            control: Arc::new(Control::new()),
            metrics: Arc::new(Metrics::default()),
            current: Arc::new(Mutex::new(None)),
            listener,
        }
    }

    pub fn status(&self) -> EngineStatus {
        let current = self.current.lock();
        EngineStatus {
            state: self.control.state(),
            macro_id: current.as_ref().map(|run| run.macro_id.clone()),
            iterations: self.metrics.iterations.load(Ordering::Relaxed),
            action_index: self.metrics.action_index.load(Ordering::Relaxed),
            elapsed_ms: current
                .as_ref()
                .map(|run| run.started.elapsed().as_millis() as u64)
                .unwrap_or(0),
            actual_cps: self.metrics.cps(),
        }
    }

    pub fn state(&self) -> EngineState {
        self.control.state()
    }

    pub fn stop(&self) -> bool {
        self.control.stop()
    }

    pub fn pause(&self) -> bool {
        self.control.pause()
    }

    pub fn resume(&self) -> bool {
        self.control.resume()
    }

    /// Spawns the worker. Returns as soon as the run is claimed; progress
    /// arrives through the listener.
    pub fn start(&self, value: Macro) -> Result<(), EngineError> {
        if value.actions.is_empty() {
            return Err(EngineError::Empty);
        }
        if !self.control.begin() {
            return Err(EngineError::Busy);
        }

        self.metrics.reset();
        *self.current.lock() = Some(CurrentRun {
            macro_id: value.id.clone(),
            started: Instant::now(),
        });

        let control = Arc::clone(&self.control);
        let metrics = Arc::clone(&self.metrics);
        let current = Arc::clone(&self.current);
        let listener = Arc::clone(&self.listener);

        thread::Builder::new()
            .name("flowmacro-engine".into())
            .spawn(move || {
                let pump = spawn_status_pump(
                    Arc::clone(&control),
                    Arc::clone(&metrics),
                    Arc::clone(&current),
                    Arc::clone(&listener),
                );

                let outcome = execute(&value, &control, &metrics);

                control.finish();
                let _ = pump.join();

                *current.lock() = None;
                listener.on_finished(&outcome);
                listener.on_status(&EngineStatus {
                    state: EngineState::Idle,
                    macro_id: None,
                    iterations: outcome.iterations,
                    action_index: -1,
                    elapsed_ms: outcome.duration_ms,
                    actual_cps: 0.0,
                });
            })
            .expect("spawn engine thread");

        Ok(())
    }
}

fn execute(value: &Macro, control: &Control, metrics: &Metrics) -> RunOutcome {
    // Raised only for the duration of the run: holding 1 ms resolution all the
    // time costs battery across the whole system (§2.3).
    let _resolution = TimerResolution::acquire();

    let started = Instant::now();
    let budget = iteration_budget(value.loop_config.mode, value.loop_config.count);
    let deadline = match value.loop_config.mode {
        LoopMode::Duration => {
            Some(started + Duration::from_millis(value.loop_config.duration_ms))
        }
        _ => None,
    };

    let ctx = ExecContext { control, metrics, randomization: value.randomization };
    let mut iterations = 0u64;
    let mut error = None;
    let mut completed = true;

    // Iterations run on an absolute timeline rather than "sleep the gap after
    // each pass": adding the gap to a stored instant keeps the average rate
    // exact instead of accumulating every overshoot (§2.3).
    let loop_gap = Duration::from_millis(value.loop_config.interval_ms as u64);
    let mut next_start = Instant::now();
    let abort = || control.should_abort();

    loop {
        if let Some(limit) = budget {
            if iterations >= limit {
                break;
            }
        }
        if deadline.is_some_and(|at| Instant::now() >= at) {
            break;
        }
        if control.should_abort() {
            completed = false;
            break;
        }

        if !timing::sleep_until(next_start, &abort) {
            completed = false;
            break;
        }
        // The wait above can cross the duration deadline; do not start a pass
        // that the loop has already outlived.
        if deadline.is_some_and(|at| Instant::now() >= at) {
            break;
        }

        match run_actions(&ctx, &value.actions) {
            Ok(Flow::Completed) => {
                iterations += 1;
                metrics.iterations.store(iterations, Ordering::Relaxed);
            }
            Ok(Flow::Aborted) => {
                completed = false;
                break;
            }
            Err(failure) => {
                error = Some(failure.to_string());
                completed = false;
                break;
            }
        }

        let gap = if value.randomization.enabled {
            humanize(loop_gap, value.randomization.jitter_ms)
        } else {
            loop_gap
        };
        next_start += gap;

        // Behind schedule (a long pass, a paused run, no gap at all): restart
        // the timeline from now so we never fire a catch-up burst.
        let floor = Instant::now() + MIN_ITERATION_GAP;
        if next_start < floor {
            next_start = floor;
        }
    }

    RunOutcome {
        macro_id: value.id.clone(),
        macro_name: value.name.clone(),
        iterations,
        duration_ms: started.elapsed().as_millis() as u64,
        completed,
        error,
    }
}

fn spawn_status_pump(
    control: Arc<Control>,
    metrics: Arc<Metrics>,
    current: Arc<Mutex<Option<CurrentRun>>>,
    listener: Arc<dyn EngineListener>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("flowmacro-status".into())
        .spawn(move || {
            while control.state() != EngineState::Idle {
                let status = {
                    let run = current.lock();
                    EngineStatus {
                        state: control.state(),
                        macro_id: run.as_ref().map(|item| item.macro_id.clone()),
                        iterations: metrics.iterations.load(Ordering::Relaxed),
                        action_index: metrics.action_index.load(Ordering::Relaxed),
                        elapsed_ms: run
                            .as_ref()
                            .map(|item| item.started.elapsed().as_millis() as u64)
                            .unwrap_or(0),
                        actual_cps: metrics.cps(),
                    }
                };
                listener.on_status(&status);
                thread::sleep(STATUS_INTERVAL);
            }
        })
        .expect("spawn status thread")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::model::{Action, ActionKind, LoopConfig};


    fn delay_macro(id: &str, delay_ms: u32, loop_config: LoopConfig) -> Macro {
        Macro {
            id: id.into(),
            name: "Test".into(),
            enabled: true,
            hotkey: None,
            activation: crate::macros::model::Activation::Toggle,
            actions: vec![Action {
                id: "a1".into(),
                delay_before_ms: 0,
                delay_after_ms: 0,
                kind: ActionKind::Delay { duration_ms: delay_ms },
            }],
            loop_config,
            randomization: Default::default(),
            stats: Default::default(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn wait_for_idle(engine: &Engine, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if engine.state() == EngineState::Idle {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }
        false
    }

    #[test]
    fn refuses_a_macro_with_no_actions() {
        let engine = Engine::new(Arc::new(SilentListener));
        let mut value = delay_macro("m1", 1, LoopConfig::default());
        value.actions.clear();
        assert_eq!(engine.start(value), Err(EngineError::Empty));
        assert_eq!(engine.state(), EngineState::Idle);
    }

    #[test]
    fn refuses_a_second_concurrent_run() {
        let engine = Engine::new(Arc::new(SilentListener));
        let value = delay_macro("m1", 300, LoopConfig::default());
        engine.start(value.clone()).expect("first run");
        assert_eq!(engine.start(value), Err(EngineError::Busy));
        engine.stop();
        assert!(wait_for_idle(&engine, Duration::from_secs(2)));
    }

    #[test]
    fn a_single_pass_macro_returns_to_idle() {
        let engine = Engine::new(Arc::new(SilentListener));
        engine
            .start(delay_macro("m1", 20, LoopConfig::default()))
            .expect("start");
        assert!(wait_for_idle(&engine, Duration::from_secs(2)));
        assert_eq!(engine.status().iterations, 1);
    }

    #[test]
    fn count_loops_run_exactly_that_many_times() {
        let engine = Engine::new(Arc::new(SilentListener));
        engine
            .start(delay_macro(
                "m1",
                5,
                LoopConfig { mode: LoopMode::Count, count: 4, duration_ms: 0, interval_ms: 0 },
            ))
            .expect("start");
        assert!(wait_for_idle(&engine, Duration::from_secs(3)));
        assert_eq!(engine.status().iterations, 4);
    }

    #[test]
    fn stop_cancels_an_infinite_loop_quickly() {
        let engine = Engine::new(Arc::new(SilentListener));
        engine
            .start(delay_macro(
                "m1",
                50,
                LoopConfig { mode: LoopMode::Infinite, count: 0, duration_ms: 0, interval_ms: 0 },
            ))
            .expect("start");

        thread::sleep(Duration::from_millis(120));
        assert_eq!(engine.state(), EngineState::Running);

        let asked_at = Instant::now();
        engine.stop();
        assert!(wait_for_idle(&engine, Duration::from_secs(2)));
        assert!(
            asked_at.elapsed() < Duration::from_millis(300),
            "stop took {:?}",
            asked_at.elapsed()
        );
    }

    #[test]
    fn the_loop_interval_paces_the_iterations() {
        let engine = Engine::new(Arc::new(SilentListener));
        let started = Instant::now();
        engine
            .start(delay_macro(
                "m1",
                1,
                LoopConfig { mode: LoopMode::Count, count: 5, duration_ms: 0, interval_ms: 40 },
            ))
            .expect("start");

        assert!(wait_for_idle(&engine, Duration::from_secs(3)));
        assert_eq!(engine.status().iterations, 5);

        // Four gaps of 40 ms between the five passes; the first runs at once.
        let elapsed = started.elapsed();
        assert!(elapsed >= Duration::from_millis(150), "ran too fast: {elapsed:?}");
        assert!(elapsed < Duration::from_millis(450), "ran too slow: {elapsed:?}");
    }

    #[test]
    fn a_zero_delay_infinite_loop_stays_below_the_floor() {
        let engine = Engine::new(Arc::new(SilentListener));
        engine
            .start(delay_macro(
                "m1",
                0,
                LoopConfig { mode: LoopMode::Infinite, count: 0, duration_ms: 0, interval_ms: 0 },
            ))
            .expect("start");

        thread::sleep(Duration::from_millis(200));
        let iterations = engine.status().iterations;
        engine.stop();
        assert!(wait_for_idle(&engine, Duration::from_secs(2)));

        // 200 ms at the 250 us floor is at most ~800 passes; without the floor
        // this loop reaches millions and drowns the input queue.
        assert!(iterations <= 1_600, "loop ran unthrottled: {iterations}");
        assert!(iterations > 50, "loop barely ran: {iterations}");
    }

    /// End-to-end over the real `SendInput` path: a relative move of (0, 0)
    /// travels the whole injection stack but does not touch the cursor, so the
    /// achieved rate can be measured without disturbing the desktop.
    #[cfg(windows)]
    #[test]
    fn real_injection_holds_the_requested_rate() {
        use crate::macros::model::MoveCurve;

        let value = Macro {
            id: "m1".into(),
            name: "Rate".into(),
            enabled: true,
            hotkey: None,
            activation: crate::macros::model::Activation::Toggle,
            actions: vec![Action {
                id: "a1".into(),
                delay_before_ms: 0,
                delay_after_ms: 0,
                kind: ActionKind::MouseMove {
                    mode: crate::macros::model::MoveMode::Relative,
                    x: 0,
                    y: 0,
                    duration_ms: 0,
                    curve: MoveCurve::Linear,
                },
            }],
            loop_config: LoopConfig {
                mode: LoopMode::Count,
                count: 25,
                duration_ms: 0,
                interval_ms: 40,
            },
            randomization: Default::default(),
            stats: Default::default(),
            created_at: String::new(),
            updated_at: String::new(),
        };

        let engine = Engine::new(Arc::new(SilentListener));
        let started = Instant::now();
        engine.start(value).expect("start");

        // Half way through, the measured rate should already be near 25/s.
        thread::sleep(Duration::from_millis(500));
        let mid = engine.status().actual_cps;

        assert!(wait_for_idle(&engine, Duration::from_secs(5)));
        let elapsed = started.elapsed();

        assert_eq!(engine.status().iterations, 25);
        assert!(
            (15.0..=40.0).contains(&mid),
            "measured rate was {mid}/s, expected about 25"
        );
        // 24 gaps of 40 ms is 960 ms; allow for scheduler noise on a busy box.
        assert!(elapsed >= Duration::from_millis(880), "ran too fast: {elapsed:?}");
        assert!(elapsed < Duration::from_millis(1_800), "ran too slow: {elapsed:?}");
    }

    #[test]
    fn duration_loops_stop_on_their_own() {
        let engine = Engine::new(Arc::new(SilentListener));
        let started = Instant::now();
        engine
            .start(delay_macro(
                "m1",
                10,
                LoopConfig { mode: LoopMode::Duration, count: 0, duration_ms: 200, interval_ms: 0 },
            ))
            .expect("start");

        assert!(wait_for_idle(&engine, Duration::from_secs(3)));
        let elapsed = started.elapsed();
        assert!(elapsed >= Duration::from_millis(180), "ended too early: {elapsed:?}");
        assert!(elapsed < Duration::from_millis(900), "ran too long: {elapsed:?}");
    }

    #[test]
    fn pause_holds_the_run_until_resume() {
        let engine = Engine::new(Arc::new(SilentListener));
        engine
            .start(delay_macro(
                "m1",
                5,
                LoopConfig { mode: LoopMode::Infinite, count: 0, duration_ms: 0, interval_ms: 0 },
            ))
            .expect("start");

        thread::sleep(Duration::from_millis(60));
        assert!(engine.pause());
        thread::sleep(Duration::from_millis(40));
        let frozen = engine.status().iterations;
        thread::sleep(Duration::from_millis(120));
        assert_eq!(engine.status().iterations, frozen, "iterations moved while paused");

        assert!(engine.resume());
        thread::sleep(Duration::from_millis(80));
        assert!(engine.status().iterations > frozen);

        engine.stop();
        assert!(wait_for_idle(&engine, Duration::from_secs(2)));
    }
}
