//! Deadline bookkeeping and humanised timing (§2.3, §13).

use std::time::{Duration, Instant};

/// A fixed-rate schedule built on an **absolute** timeline.
///
/// Sleeping for `interval` each round accumulates every overshoot: at 100 CPS
/// a 0.2 ms error per click is 12 seconds of drift over ten minutes. Adding
/// `interval` to a stored deadline instead keeps the average rate exact.
pub struct Cadence {
    next: Instant,
    interval: Duration,
}

impl Cadence {
    pub fn starting_now(interval: Duration) -> Self {
        Self { next: Instant::now(), interval }
    }

    pub fn starting_at(start: Instant, interval: Duration) -> Self {
        Self { next: start, interval }
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// The deadline for the current step; advances the schedule by one slot.
    pub fn tick(&mut self) -> Instant {
        let at = self.next;
        self.next += self.interval;
        at
    }

    pub fn peek(&self) -> Instant {
        self.next
    }

    /// If the machine stalled (a scheduler hiccup, a paused run) we would
    /// otherwise fire a burst of catch-up events. Past `max_lag` behind, the
    /// schedule restarts from now instead.
    pub fn resync(&mut self, now: Instant, max_lag: Duration) {
        if now > self.next && now - self.next > max_lag {
            self.next = now;
        }
    }

    pub fn shift(&mut self, by: Duration) {
        self.next += by;
    }
}

/// Symmetric jitter around `base`, never below zero (§13).
pub fn humanize(base: Duration, jitter_ms: u32) -> Duration {
    if jitter_ms == 0 {
        return base;
    }
    let span = jitter_ms as i64;
    let offset = fastrand::i64(-span..=span);
    let base_ms = base.as_micros() as i64 / 1_000;
    let result_ms = (base_ms + offset).max(0);
    Duration::from_micros((result_ms * 1_000).max(0) as u64)
}

/// How many loop iterations to run, or `None` for "until stopped".
pub fn iteration_budget(mode: crate::macros::model::LoopMode, count: u32) -> Option<u64> {
    use crate::macros::model::LoopMode;
    match mode {
        LoopMode::None => Some(1),
        LoopMode::Count => Some(count.max(1) as u64),
        LoopMode::Infinite | LoopMode::Duration => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::model::LoopMode;

    #[test]
    fn cadence_does_not_drift() {
        let start = Instant::now();
        let mut cadence = Cadence::starting_at(start, Duration::from_millis(10));
        for _ in 0..100 {
            cadence.tick();
        }
        // 100 ticks of 10 ms is exactly one second past the start.
        let expected = start + Duration::from_secs(1);
        assert_eq!(cadence.peek(), expected);
    }

    #[test]
    fn resync_only_kicks_in_past_the_lag_budget() {
        let start = Instant::now();
        let mut cadence = Cadence::starting_at(start, Duration::from_millis(10));
        cadence.tick();

        let slightly_late = start + Duration::from_millis(15);
        cadence.resync(slightly_late, Duration::from_millis(50));
        assert_eq!(cadence.peek(), start + Duration::from_millis(10));

        let very_late = start + Duration::from_secs(5);
        cadence.resync(very_late, Duration::from_millis(50));
        assert_eq!(cadence.peek(), very_late);
    }

    #[test]
    fn humanize_stays_within_the_jitter_band() {
        let base = Duration::from_millis(50);
        for _ in 0..500 {
            let value = humanize(base, 10).as_millis() as i64;
            assert!((40..=60).contains(&value), "out of band: {value}");
        }
    }

    #[test]
    fn humanize_never_returns_a_negative_wait() {
        let base = Duration::from_millis(2);
        for _ in 0..500 {
            assert!(humanize(base, 100) >= Duration::ZERO);
        }
    }

    #[test]
    fn zero_jitter_is_the_identity() {
        let base = Duration::from_millis(33);
        assert_eq!(humanize(base, 0), base);
    }

    #[test]
    fn loop_budgets_match_the_modes() {
        assert_eq!(iteration_budget(LoopMode::None, 9), Some(1));
        assert_eq!(iteration_budget(LoopMode::Count, 5), Some(5));
        assert_eq!(iteration_budget(LoopMode::Count, 0), Some(1));
        assert_eq!(iteration_budget(LoopMode::Infinite, 5), None);
        assert_eq!(iteration_budget(LoopMode::Duration, 5), None);
    }
}
