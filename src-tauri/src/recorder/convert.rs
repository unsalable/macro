//! Turns a raw recording into a macro (§16, §17).
//!
//! Raw hook output is far too noisy to replay verbatim — a two second
//! recording holds hundreds of mouse moves. The rules here are the difference
//! between a usable macro and an unreadable transcript, so they are pure
//! functions with tests rather than logic buried in the capture thread.

use serde::{Deserialize, Serialize};

use crate::macros::model::{
    Action, ActionKind, ClickMode, MouseButton, MoveCurve, MoveMode, ScrollDirection,
};
use crate::util::new_id;

/// Moves shorter than this are dropped as hand tremor.
const MOVE_THRESHOLD_PX: i32 = 4;
/// A button held longer than this becomes a hold action, not a click.
const HOLD_THRESHOLD_MS: u64 = 250;
/// Scroll notches this close together are merged into one action.
const SCROLL_MERGE_MS: u64 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Down,
    Up,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum RecordedKind {
    Key { action: Phase, code: String },
    MouseButton { action: Phase, button: MouseButton },
    MouseMove { x: i32, y: i32 },
    MouseScroll { direction: ScrollDirection, amount: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedEvent {
    /// Milliseconds since the recording started.
    pub at: u64,
    #[serde(flatten)]
    pub kind: RecordedKind,
}

#[derive(Debug, Clone, Copy)]
pub struct ConvertOptions {
    pub include_moves: bool,
    /// Caps the gap inserted between actions so an idle pause does not become
    /// a ten minute delay in the macro.
    pub max_gap_ms: u64,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self { include_moves: true, max_gap_ms: 5_000 }
    }
}

pub fn to_actions(events: &[RecordedEvent], options: ConvertOptions) -> Vec<Action> {
    let mut actions: Vec<Action> = Vec::new();
    let mut last_at = events.first().map(|event| event.at).unwrap_or(0);
    let mut last_point: Option<(i32, i32)> = None;
    let mut pending_key: Vec<(String, u64)> = Vec::new();
    let mut pending_button: Vec<(MouseButton, u64)> = Vec::new();

    for event in events {
        match &event.kind {
            RecordedKind::MouseMove { x, y } => {
                if !options.include_moves {
                    continue;
                }
                if let Some((px, py)) = last_point {
                    if (x - px).abs() < MOVE_THRESHOLD_PX && (y - py).abs() < MOVE_THRESHOLD_PX {
                        continue;
                    }
                }
                last_point = Some((*x, *y));
                actions.push(action(
                    gap(event.at, last_at, options),
                    ActionKind::MouseMove {
                        mode: MoveMode::Absolute,
                        x: *x,
                        y: *y,
                        duration_ms: 0,
                        curve: MoveCurve::Linear,
                    },
                ));
                last_at = event.at;
            }

            RecordedKind::Key { action: Phase::Down, code } => {
                pending_key.push((code.clone(), event.at));
            }

            RecordedKind::Key { action: Phase::Up, code } => {
                let Some(index) = pending_key.iter().rposition(|(held, _)| held == code) else {
                    continue;
                };
                let (_, down_at) = pending_key.remove(index);
                let held_ms = event.at.saturating_sub(down_at);
                actions.push(action(
                    gap(down_at, last_at, options),
                    ActionKind::Key {
                        action: crate::macros::model::KeyPhase::Press,
                        code: code.clone(),
                        modifiers: Vec::new(),
                        duration_ms: if held_ms > HOLD_THRESHOLD_MS { held_ms as u32 } else { 0 },
                    },
                ));
                last_at = event.at;
            }

            RecordedKind::MouseButton { action: Phase::Down, button } => {
                pending_button.push((*button, event.at));
            }

            RecordedKind::MouseButton { action: Phase::Up, button } => {
                let Some(index) = pending_button.iter().rposition(|(held, _)| held == button) else {
                    continue;
                };
                let (_, down_at) = pending_button.remove(index);
                let held_ms = event.at.saturating_sub(down_at);
                let hold = held_ms > HOLD_THRESHOLD_MS;
                actions.push(action(
                    gap(down_at, last_at, options),
                    ActionKind::MouseClick {
                        button: *button,
                        mode: if hold { ClickMode::Hold } else { ClickMode::Single },
                        count: 1,
                        interval_ms: 1,
                        duration_ms: if hold { held_ms as u32 } else { 0 },
                    },
                ));
                last_at = event.at;
            }

            RecordedKind::MouseScroll { direction, amount } => {
                if let Some(previous) = actions.last_mut() {
                    if let ActionKind::MouseScroll { direction: prev_dir, amount: prev_amount, .. } =
                        &mut previous.kind
                    {
                        if prev_dir == direction && event.at.saturating_sub(last_at) <= SCROLL_MERGE_MS
                        {
                            *prev_amount += *amount;
                            last_at = event.at;
                            continue;
                        }
                    }
                }
                actions.push(action(
                    gap(event.at, last_at, options),
                    ActionKind::MouseScroll {
                        direction: *direction,
                        amount: *amount,
                        interval_ms: 30,
                    },
                ));
                last_at = event.at;
            }
        }
    }

    actions
}

fn gap(at: u64, last_at: u64, options: ConvertOptions) -> u32 {
    at.saturating_sub(last_at).min(options.max_gap_ms) as u32
}

fn action(delay_before_ms: u32, kind: ActionKind) -> Action {
    Action {
        id: new_id("act"),
        delay_before_ms,
        delay_after_ms: 0,
        kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(at: u64, action: Phase, code: &str) -> RecordedEvent {
        RecordedEvent { at, kind: RecordedKind::Key { action, code: code.into() } }
    }

    fn button(at: u64, action: Phase, button: MouseButton) -> RecordedEvent {
        RecordedEvent { at, kind: RecordedKind::MouseButton { action, button } }
    }

    fn mv(at: u64, x: i32, y: i32) -> RecordedEvent {
        RecordedEvent { at, kind: RecordedKind::MouseMove { x, y } }
    }

    fn scroll(at: u64, direction: ScrollDirection) -> RecordedEvent {
        RecordedEvent { at, kind: RecordedKind::MouseScroll { direction, amount: 1 } }
    }

    #[test]
    fn a_tap_becomes_a_single_key_press() {
        let events = vec![key(0, Phase::Down, "KeyA"), key(40, Phase::Up, "KeyA")];
        let actions = to_actions(&events, ConvertOptions::default());

        assert_eq!(actions.len(), 1);
        match &actions[0].kind {
            ActionKind::Key { code, duration_ms, .. } => {
                assert_eq!(code, "KeyA");
                assert_eq!(*duration_ms, 0, "a short tap should not carry a hold time");
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn a_held_key_keeps_its_duration() {
        let events = vec![key(0, Phase::Down, "KeyA"), key(800, Phase::Up, "KeyA")];
        let actions = to_actions(&events, ConvertOptions::default());
        match &actions[0].kind {
            ActionKind::Key { duration_ms, .. } => assert_eq!(*duration_ms, 800),
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn a_long_press_becomes_a_hold_click() {
        let events = vec![
            button(0, Phase::Down, MouseButton::Left),
            button(600, Phase::Up, MouseButton::Left),
        ];
        let actions = to_actions(&events, ConvertOptions::default());
        match &actions[0].kind {
            ActionKind::MouseClick { mode, duration_ms, .. } => {
                assert_eq!(*mode, ClickMode::Hold);
                assert_eq!(*duration_ms, 600);
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn tremor_moves_are_dropped() {
        let events = vec![mv(0, 100, 100), mv(10, 101, 101), mv(20, 102, 100)];
        let actions = to_actions(&events, ConvertOptions::default());
        assert_eq!(actions.len(), 1, "only the first sample should survive");
    }

    #[test]
    fn real_moves_are_kept() {
        let events = vec![mv(0, 100, 100), mv(10, 400, 300)];
        let actions = to_actions(&events, ConvertOptions::default());
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn moves_can_be_excluded_entirely() {
        let events = vec![mv(0, 10, 10), mv(30, 900, 900), key(40, Phase::Down, "KeyA"), key(50, Phase::Up, "KeyA")];
        let options = ConvertOptions { include_moves: false, ..ConvertOptions::default() };
        let actions = to_actions(&events, options);
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn consecutive_scrolls_merge_into_one_action() {
        let events = vec![
            scroll(0, ScrollDirection::Down),
            scroll(40, ScrollDirection::Down),
            scroll(80, ScrollDirection::Down),
        ];
        let actions = to_actions(&events, ConvertOptions::default());
        assert_eq!(actions.len(), 1);
        match &actions[0].kind {
            ActionKind::MouseScroll { amount, .. } => assert_eq!(*amount, 3),
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn a_direction_change_starts_a_new_scroll_action() {
        let events = vec![scroll(0, ScrollDirection::Down), scroll(40, ScrollDirection::Up)];
        let actions = to_actions(&events, ConvertOptions::default());
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn idle_gaps_are_capped() {
        let events = vec![
            key(0, Phase::Down, "KeyA"),
            key(10, Phase::Up, "KeyA"),
            key(60_000, Phase::Down, "KeyB"),
            key(60_010, Phase::Up, "KeyB"),
        ];
        let options = ConvertOptions { max_gap_ms: 2_000, ..ConvertOptions::default() };
        let actions = to_actions(&events, options);
        assert_eq!(actions[1].delay_before_ms, 2_000);
    }

    #[test]
    fn an_unmatched_release_is_ignored() {
        let events = vec![key(0, Phase::Up, "KeyA")];
        assert!(to_actions(&events, ConvertOptions::default()).is_empty());
    }

    #[test]
    fn overlapping_keys_pair_with_their_own_release() {
        let events = vec![
            key(0, Phase::Down, "KeyA"),
            key(10, Phase::Down, "KeyB"),
            key(20, Phase::Up, "KeyB"),
            key(30, Phase::Up, "KeyA"),
        ];
        let actions = to_actions(&events, ConvertOptions::default());
        assert_eq!(actions.len(), 2);
        match (&actions[0].kind, &actions[1].kind) {
            (ActionKind::Key { code: first, .. }, ActionKind::Key { code: second, .. }) => {
                assert_eq!(first, "KeyB");
                assert_eq!(second, "KeyA");
            }
            other => panic!("unexpected actions: {other:?}"),
        }
    }
}
