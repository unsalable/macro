//! Turns a list of actions into real input events.
//!
//! Every wait goes through `timing::sleep_until` with the control flag as its
//! abort predicate, so the run unwinds promptly no matter how deep it is.

use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::input::inject::{self, InjectError};
use crate::input::timing;
use crate::macros::control::Control;
use crate::macros::model::{Action, ActionKind, ClickMode, KeyPhase, MoveMode, Randomization};
use crate::macros::scheduler::{humanize, Cadence};

/// How often an interpolated mouse move emits a step.
const MOVE_STEP: Duration = Duration::from_millis(8);
/// Past this much lag the cadence restarts instead of firing a catch-up burst.
const MAX_LAG: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Completed,
    Aborted,
}

impl Flow {
    fn from_wait(ok: bool) -> Self {
        if ok {
            Self::Completed
        } else {
            Self::Aborted
        }
    }
}

pub struct Metrics {
    pub iterations: AtomicU64,
    pub action_index: AtomicI32,
    pub injections: AtomicU64,
    window: Mutex<CpsWindow>,
}

struct CpsWindow {
    start: Instant,
    count: u64,
    value: f64,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            iterations: AtomicU64::new(0),
            action_index: AtomicI32::new(-1),
            injections: AtomicU64::new(0),
            window: Mutex::new(CpsWindow { start: Instant::now(), count: 0, value: 0.0 }),
        }
    }
}

impl Metrics {
    /// Measured, not requested: the number the UI shows is what actually left
    /// the machine in the last quarter second.
    pub fn record_injection(&self) {
        self.injections.fetch_add(1, Ordering::Relaxed);
        let mut window = self.window.lock();
        window.count += 1;
        let elapsed = window.start.elapsed();
        if elapsed >= Duration::from_millis(250) {
            window.value = window.count as f64 / elapsed.as_secs_f64();
            window.start = Instant::now();
            window.count = 0;
        }
    }

    pub fn cps(&self) -> f64 {
        let window = self.window.lock();
        let elapsed = window.start.elapsed();
        // Decay to zero when nothing has been injected for a while.
        if elapsed > Duration::from_millis(750) {
            0.0
        } else {
            window.value
        }
    }

    pub fn reset(&self) {
        self.iterations.store(0, Ordering::Relaxed);
        self.action_index.store(-1, Ordering::Relaxed);
        self.injections.store(0, Ordering::Relaxed);
        let mut window = self.window.lock();
        window.start = Instant::now();
        window.count = 0;
        window.value = 0.0;
    }
}

pub struct ExecContext<'a> {
    pub control: &'a Control,
    pub metrics: &'a Metrics,
    pub randomization: Randomization,
}

impl ExecContext<'_> {
    fn jitter(&self, base: Duration) -> Duration {
        if self.randomization.enabled {
            humanize(base, self.randomization.jitter_ms)
        } else {
            base
        }
    }

    fn wait(&self, duration: Duration) -> Flow {
        let abort = || self.control.should_abort();
        Flow::from_wait(timing::sleep_for(duration, &abort))
    }

    fn wait_until(&self, deadline: Instant) -> Flow {
        let abort = || self.control.should_abort();
        Flow::from_wait(timing::sleep_until(deadline, &abort))
    }

    fn checkpoint(&self) -> Flow {
        Flow::from_wait(self.control.wait_while_paused())
    }
}

pub fn run_actions(ctx: &ExecContext<'_>, actions: &[Action]) -> Result<Flow, InjectError> {
    run_actions_at(ctx, actions, 0)
}

fn run_actions_at(
    ctx: &ExecContext<'_>,
    actions: &[Action],
    depth: usize,
) -> Result<Flow, InjectError> {
    for (index, action) in actions.iter().enumerate() {
        if ctx.checkpoint() == Flow::Aborted {
            return Ok(Flow::Aborted);
        }
        // The UI highlights the top-level step; reporting an index from inside
        // a repeat block would make the highlight jump around.
        if depth == 0 {
            ctx.metrics.action_index.store(index as i32, Ordering::Relaxed);
        }

        if ctx.wait(ctx.jitter(Duration::from_millis(action.delay_before_ms as u64)))
            == Flow::Aborted
        {
            return Ok(Flow::Aborted);
        }

        if run_action(ctx, action, depth)? == Flow::Aborted {
            return Ok(Flow::Aborted);
        }

        if ctx.wait(ctx.jitter(Duration::from_millis(action.delay_after_ms as u64)))
            == Flow::Aborted
        {
            return Ok(Flow::Aborted);
        }
    }
    Ok(Flow::Completed)
}

/// Runs one action outside a macro run, for the editor's Test button (§60).
/// It gets its own control block so it cannot disturb — or be disturbed by —
/// the engine's state machine.
pub fn run_single(action: &Action) -> Result<Flow, InjectError> {
    let control = Control::new();
    control.begin();
    let metrics = Metrics::default();
    let ctx = ExecContext {
        control: &control,
        metrics: &metrics,
        randomization: Randomization::default(),
    };
    let result = run_actions(&ctx, std::slice::from_ref(action));
    control.finish();
    result
}

fn run_action(
    ctx: &ExecContext<'_>,
    action: &Action,
    depth: usize,
) -> Result<Flow, InjectError> {
    match &action.kind {
        ActionKind::MouseClick { button, mode, count, interval_ms, duration_ms } => {
            let mut cadence =
                Cadence::starting_now(ctx.jitter(Duration::from_millis(*interval_ms as u64)));
            for round in 0..*count {
                let deadline = cadence.tick();
                match mode {
                    ClickMode::Single => inject::mouse_click(*button)?,
                    ClickMode::Double => {
                        inject::mouse_click(*button)?;
                        inject::mouse_click(*button)?;
                    }
                    ClickMode::Hold => {
                        inject::mouse_down(*button)?;
                        let held = ctx.wait(Duration::from_millis(*duration_ms as u64));
                        inject::mouse_up(*button)?;
                        if held == Flow::Aborted {
                            return Ok(Flow::Aborted);
                        }
                    }
                }
                ctx.metrics.record_injection();

                // The last click does not wait for its slot; the action's own
                // delay_after takes over from here.
                if round + 1 < *count {
                    let next = cadence.peek();
                    if ctx.wait_until(next) == Flow::Aborted {
                        return Ok(Flow::Aborted);
                    }
                    cadence.resync(Instant::now(), MAX_LAG);
                } else {
                    let _ = deadline;
                }
            }
            Ok(Flow::Completed)
        }

        ActionKind::MouseMove { mode, x, y, duration_ms, curve } => {
            move_mouse(ctx, *mode, *x, *y, *duration_ms, *curve)
        }

        ActionKind::MouseScroll { direction, amount, interval_ms } => {
            let mut cadence =
                Cadence::starting_now(ctx.jitter(Duration::from_millis(*interval_ms as u64)));
            for notch in 0..*amount {
                cadence.tick();
                inject::scroll(*direction, 1)?;
                ctx.metrics.record_injection();
                if notch + 1 < *amount && ctx.wait_until(cadence.peek()) == Flow::Aborted {
                    return Ok(Flow::Aborted);
                }
            }
            Ok(Flow::Completed)
        }

        ActionKind::Key { action: phase, code, modifiers, duration_ms } => {
            match phase {
                KeyPhase::Down => {
                    inject::modifiers_down(modifiers)?;
                    inject::key_down(code)?;
                }
                KeyPhase::Up => {
                    inject::key_up(code)?;
                    inject::modifiers_up(modifiers)?;
                }
                KeyPhase::Press => {
                    inject::modifiers_down(modifiers)?;
                    if *duration_ms == 0 {
                        inject::key_press(code)?;
                    } else {
                        inject::key_down(code)?;
                        let held = ctx.wait(Duration::from_millis(*duration_ms as u64));
                        inject::key_up(code)?;
                        if held == Flow::Aborted {
                            inject::modifiers_up(modifiers)?;
                            return Ok(Flow::Aborted);
                        }
                    }
                    inject::modifiers_up(modifiers)?;
                }
            }
            ctx.metrics.record_injection();
            Ok(Flow::Completed)
        }

        ActionKind::Text { value, per_char_delay_ms } => {
            let gap = Duration::from_millis(*per_char_delay_ms as u64);
            for ch in value.chars() {
                inject::type_char(ch)?;
                ctx.metrics.record_injection();
                if ctx.wait(ctx.jitter(gap)) == Flow::Aborted {
                    return Ok(Flow::Aborted);
                }
            }
            Ok(Flow::Completed)
        }

        ActionKind::Delay { duration_ms } => {
            Ok(ctx.wait(ctx.jitter(Duration::from_millis(*duration_ms as u64))))
        }

        ActionKind::Repeat { times, actions } => {
            for _ in 0..*times {
                if run_actions_at(ctx, actions, depth + 1)? == Flow::Aborted {
                    return Ok(Flow::Aborted);
                }
            }
            Ok(Flow::Completed)
        }
    }
}

fn move_mouse(
    ctx: &ExecContext<'_>,
    mode: MoveMode,
    x: i32,
    y: i32,
    duration_ms: u32,
    curve: crate::macros::model::MoveCurve,
) -> Result<Flow, InjectError> {
    if duration_ms == 0 {
        match mode {
            MoveMode::Absolute => inject::move_absolute(x, y)?,
            MoveMode::Relative => inject::move_relative(x, y)?,
        }
        ctx.metrics.record_injection();
        return Ok(Flow::Completed);
    }

    let total = Duration::from_millis(duration_ms as u64);
    let steps = interpolation_steps(total);
    let start = Instant::now();
    let mut emitted = (0i32, 0i32);

    for step in 1..=steps {
        let progress = ease(step as f64 / steps as f64, curve);
        let target = ((x as f64 * progress) as i32, (y as f64 * progress) as i32);

        match mode {
            // Absolute moves walk toward the destination point.
            MoveMode::Absolute => inject::move_absolute(target.0, target.1)?,
            // Relative moves emit only the delta not yet sent.
            MoveMode::Relative => {
                inject::move_relative(target.0 - emitted.0, target.1 - emitted.1)?
            }
        }
        emitted = target;
        ctx.metrics.record_injection();

        let deadline = start + total.mul_f64(step as f64 / steps as f64);
        if ctx.wait_until(deadline) == Flow::Aborted {
            return Ok(Flow::Aborted);
        }
    }

    Ok(Flow::Completed)
}

fn interpolation_steps(total: Duration) -> u32 {
    ((total.as_millis() as u64 / MOVE_STEP.as_millis() as u64).max(1) as u32).min(600)
}

/// `smooth` is a cubic ease-in-out; `linear` passes progress through.
fn ease(t: f64, curve: crate::macros::model::MoveCurve) -> f64 {
    use crate::macros::model::MoveCurve;
    match curve {
        MoveCurve::Linear => t,
        MoveCurve::Smooth => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::model::MoveCurve;

    #[test]
    fn easing_is_pinned_at_both_ends() {
        for curve in [MoveCurve::Linear, MoveCurve::Smooth] {
            assert!((ease(0.0, curve) - 0.0).abs() < 1e-9);
            assert!((ease(1.0, curve) - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn smooth_easing_is_monotonic() {
        let mut previous = 0.0;
        for step in 0..=100 {
            let value = ease(step as f64 / 100.0, MoveCurve::Smooth);
            assert!(value >= previous - 1e-9, "went backwards at {step}");
            previous = value;
        }
    }

    #[test]
    fn linear_easing_matches_its_input() {
        assert!((ease(0.25, MoveCurve::Linear) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn step_count_scales_with_duration_and_stays_bounded() {
        assert_eq!(interpolation_steps(Duration::from_millis(0)), 1);
        assert_eq!(interpolation_steps(Duration::from_millis(80)), 10);
        assert_eq!(interpolation_steps(Duration::from_secs(60)), 600);
    }

    #[test]
    fn metrics_track_injections_and_reset() {
        let metrics = Metrics::default();
        for _ in 0..5 {
            metrics.record_injection();
        }
        assert_eq!(metrics.injections.load(Ordering::Relaxed), 5);

        metrics.reset();
        assert_eq!(metrics.injections.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.action_index.load(Ordering::Relaxed), -1);
    }

    #[test]
    fn cps_is_measured_over_the_rolling_window() {
        let metrics = Metrics::default();
        for _ in 0..30 {
            metrics.record_injection();
            std::thread::sleep(Duration::from_millis(10));
        }
        let cps = metrics.cps();
        assert!((60.0..=160.0).contains(&cps), "unexpected cps: {cps}");
    }
}
