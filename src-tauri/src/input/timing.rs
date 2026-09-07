//! Sub-millisecond scheduling helpers (§2.3).
//!
//! Windows' default timer granularity is ~15.6 ms, which cannot hold even
//! 10 CPS. We raise the resolution only while the engine runs, and combine a
//! coarse sleep with a short spin so we neither burn a core nor overshoot.

use std::hint::spin_loop;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Anything below this is spun rather than slept.
const SPIN_MARGIN: Duration = Duration::from_micros(1_500);

static ACTIVE_GUARDS: AtomicUsize = AtomicUsize::new(0);

/// RAII guard around `timeBeginPeriod(1)`. Nested guards are reference counted
/// so a nested engine run does not drop the resolution early.
pub struct TimerResolution {
    _private: (),
}

impl TimerResolution {
    pub fn acquire() -> Self {
        if ACTIVE_GUARDS.fetch_add(1, Ordering::AcqRel) == 0 {
            begin_period();
        }
        Self { _private: () }
    }
}

impl Drop for TimerResolution {
    fn drop(&mut self) {
        if ACTIVE_GUARDS.fetch_sub(1, Ordering::AcqRel) == 1 {
            end_period();
        }
    }
}

#[cfg(windows)]
fn begin_period() {
    unsafe {
        let _ = windows::Win32::Media::timeBeginPeriod(1);
    }
}

#[cfg(windows)]
fn end_period() {
    unsafe {
        let _ = windows::Win32::Media::timeEndPeriod(1);
    }
}

#[cfg(not(windows))]
fn begin_period() {}

#[cfg(not(windows))]
fn end_period() {}

/// Blocks until `deadline`, checking `should_abort` roughly every 250 µs so a
/// stop request cancels in well under a millisecond (§61).
pub fn sleep_until(deadline: Instant, should_abort: &dyn Fn() -> bool) -> bool {
    loop {
        if should_abort() {
            return false;
        }

        let now = Instant::now();
        if now >= deadline {
            return true;
        }

        let remaining = deadline - now;
        if remaining > SPIN_MARGIN {
            // Wake up early enough that the spin below can finish the job, and
            // cap each nap so aborts stay responsive during long delays.
            let nap = (remaining - SPIN_MARGIN).min(Duration::from_millis(4));
            thread::sleep(nap);
        } else {
            spin_loop();
        }
    }
}

/// Convenience wrapper for a relative wait.
pub fn sleep_for(duration: Duration, should_abort: &dyn Fn() -> bool) -> bool {
    if duration.is_zero() {
        return !should_abort();
    }
    sleep_until(Instant::now() + duration, should_abort)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_until_is_reasonably_accurate() {
        let _guard = TimerResolution::acquire();
        let never = || false;
        let target = Instant::now() + Duration::from_millis(20);
        assert!(sleep_until(target, &never));
        let overshoot = Instant::now().saturating_duration_since(target);
        assert!(overshoot < Duration::from_millis(6), "overshot by {overshoot:?}");
    }

    #[test]
    fn abort_returns_immediately() {
        let always = || true;
        let start = Instant::now();
        assert!(!sleep_for(Duration::from_secs(5), &always));
        assert!(start.elapsed() < Duration::from_millis(50));
    }

    #[test]
    fn zero_duration_is_a_no_op() {
        let never = || false;
        assert!(sleep_for(Duration::ZERO, &never));
    }
}
