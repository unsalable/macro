//! Hotkey routing over the raw hook stream (§18, §19, §61).
//!
//! This is deliberately pure: it takes key events and a clock reading and
//! returns actions, so every rule below is unit-testable without Windows.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::input::keycodes;
use crate::macros::model::{Activation, Hotkey, Modifier, MouseButton};

/// Three taps of Escape inside this window force the engine to stop, whatever
/// the user's hotkeys are set to. This safety net cannot be turned off from
/// the hotkey screen (§2.5).
pub const PANIC_WINDOW: Duration = Duration::from_millis(500);
pub const PANIC_TAPS: usize = 3;

const VK_ESCAPE: u16 = 0x1B;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyAction {
    Start,
    Stop,
    Pause,
    EmergencyStop,
    MacroToggle(String),
    MacroHoldStart(String),
    MacroHoldEnd(String),
}

#[derive(Debug, Clone)]
pub struct MacroBinding {
    pub macro_id: String,
    pub hotkey: Hotkey,
    pub activation: Activation,
}

#[derive(Debug, Clone, Default)]
pub struct GlobalBindings {
    pub start: Option<Hotkey>,
    pub stop: Option<Hotkey>,
    pub pause: Option<Hotkey>,
    pub emergency_stop: Option<Hotkey>,
}

#[derive(Debug, Default)]
pub struct Matcher {
    global: GlobalBindings,
    macros: Vec<MacroBinding>,
    held: HashSet<u16>,
    escape_taps: Vec<Instant>,
    panic_enabled: bool,
}

impl Matcher {
    pub fn new() -> Self {
        Self { panic_enabled: true, ..Self::default() }
    }

    pub fn set_global(&mut self, global: GlobalBindings) {
        self.global = global;
    }

    pub fn set_macros(&mut self, macros: Vec<MacroBinding>) {
        self.macros = macros;
    }

    pub fn set_panic_enabled(&mut self, enabled: bool) {
        self.panic_enabled = enabled;
    }

    /// Drops keys we believe are held but that are no longer physically down.
    ///
    /// The held set is what suppresses auto-repeat, and it is only ever
    /// cleared by a key-up. Miss one — the hook was reinstalled mid-press, the
    /// key came up on the lock screen or during a UAC prompt, another hook
    /// swallowed it — and that key is treated as "still held" forever, so
    /// every later press is discarded as a repeat and the hotkey is dead until
    /// the app restarts. That is exactly the "the stop key does nothing"
    /// report, so the set is reconciled against the real keyboard on a timer
    /// rather than trusted (§61).
    ///
    /// Called between events, never in place of a key-up: during genuine
    /// auto-repeat the key really is down, so nothing is cleared.
    pub fn reconcile_held(&mut self, is_down: &dyn Fn(u16) -> bool) -> usize {
        let before = self.held.len();
        self.held.retain(|vk| is_down(*vk));
        before - self.held.len()
    }

    #[cfg(test)]
    fn is_held(&self, vk: u16) -> bool {
        self.held.contains(&vk)
    }

    /// Held modifier keys, derived from the key stream rather than polled, so
    /// it stays consistent with what the hook actually saw.
    fn active_modifiers(&self) -> Vec<Modifier> {
        let mut active = Vec::new();
        if self.held.contains(&0xA2) || self.held.contains(&0xA3) || self.held.contains(&0x11) {
            active.push(Modifier::Ctrl);
        }
        if self.held.contains(&0xA0) || self.held.contains(&0xA1) || self.held.contains(&0x10) {
            active.push(Modifier::Shift);
        }
        if self.held.contains(&0xA4) || self.held.contains(&0xA5) || self.held.contains(&0x12) {
            active.push(Modifier::Alt);
        }
        if self.held.contains(&0x5B) || self.held.contains(&0x5C) {
            active.push(Modifier::Win);
        }
        active
    }

    /// Exact match: `F6` must not fire while Ctrl is held, otherwise a chord
    /// bound elsewhere would trigger two things at once.
    fn matches(&self, hotkey: &Hotkey, code: &str) -> bool {
        if !hotkey.code.eq_ignore_ascii_case(code) {
            return false;
        }
        let active = self.active_modifiers();
        active.len() == hotkey.modifiers.len()
            && hotkey.modifiers.iter().all(|needed| active.contains(needed))
    }

    /// Records the press or release and reports whether it was auto-repeat.
    ///
    /// Windows repeats WM_KEYDOWN roughly every 30 ms while a key is held,
    /// and the hook reports every one of them. A repeat is not a new press:
    /// letting it through toggles a macro dozens of times a second, so the
    /// run the user is holding the key to stop restarts before their finger
    /// comes up — which reads as "it will not turn off" (§19, §61).
    fn track_held(&mut self, vk: u16, down: bool) -> bool {
        if down {
            // `insert` is false when the key was already down.
            !self.held.insert(vk)
        } else {
            self.held.remove(&vk);
            false
        }
    }

    pub fn on_key(&mut self, vk: u16, extended: bool, down: bool, now: Instant) -> Vec<HotkeyAction> {
        if self.track_held(vk, down) {
            return Vec::new();
        }

        if down && vk == VK_ESCAPE && self.panic_enabled && self.register_escape(now) {
            return vec![HotkeyAction::EmergencyStop];
        }

        let Some(code) = keycodes::name_for_vk(vk, extended) else {
            return Vec::new();
        };

        // A modifier press on its own never triggers anything.
        if keycodes::is_modifier_key(code) {
            return Vec::new();
        }

        self.dispatch(code, down)
    }

    /// The mouse side of the same stream: a bound side button drives hotkeys
    /// exactly like a key, modifiers included (§18).
    pub fn on_mouse_button(&mut self, button: MouseButton, down: bool) -> Vec<HotkeyAction> {
        let (Some(vk), Some(code)) =
            (keycodes::mouse_button_vk(button), keycodes::mouse_button_name(button))
        else {
            // Left and right are never bindable, so they are not tracked
            // either — a stray entry in the held set would only confuse
            // reconciliation.
            return Vec::new();
        };

        if self.track_held(vk, down) {
            return Vec::new();
        }

        self.dispatch(code, down)
    }

    /// Routes one resolved key/button name to the bindings that want it.
    fn dispatch(&self, code: &str, down: bool) -> Vec<HotkeyAction> {
        let mut actions = Vec::new();

        if down {
            if self.global.emergency_stop.as_ref().is_some_and(|h| self.matches(h, code)) {
                actions.push(HotkeyAction::EmergencyStop);
            }
            if self.global.start.as_ref().is_some_and(|h| self.matches(h, code)) {
                actions.push(HotkeyAction::Start);
            }
            if self.global.stop.as_ref().is_some_and(|h| self.matches(h, code)) {
                actions.push(HotkeyAction::Stop);
            }
            if self.global.pause.as_ref().is_some_and(|h| self.matches(h, code)) {
                actions.push(HotkeyAction::Pause);
            }
        }

        for binding in &self.macros {
            if !self.matches(&binding.hotkey, code) {
                // A hold binding must still release when its key comes up,
                // even if a modifier was let go first.
                if !down
                    && binding.activation == Activation::Hold
                    && binding.hotkey.code.eq_ignore_ascii_case(code)
                {
                    actions.push(HotkeyAction::MacroHoldEnd(binding.macro_id.clone()));
                }
                continue;
            }

            match (binding.activation, down) {
                (Activation::Toggle, true) => {
                    actions.push(HotkeyAction::MacroToggle(binding.macro_id.clone()))
                }
                (Activation::Hold, true) => {
                    actions.push(HotkeyAction::MacroHoldStart(binding.macro_id.clone()))
                }
                (Activation::Hold, false) => {
                    actions.push(HotkeyAction::MacroHoldEnd(binding.macro_id.clone()))
                }
                (Activation::Toggle, false) => {}
            }
        }

        actions
    }

    fn register_escape(&mut self, now: Instant) -> bool {
        self.escape_taps
            .retain(|at| now.saturating_duration_since(*at) <= PANIC_WINDOW);
        self.escape_taps.push(now);
        if self.escape_taps.len() >= PANIC_TAPS {
            self.escape_taps.clear();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hotkey(code: &str, modifiers: &[Modifier]) -> Hotkey {
        Hotkey { code: code.into(), modifiers: modifiers.to_vec() }
    }

    fn matcher() -> Matcher {
        let mut matcher = Matcher::new();
        matcher.set_global(GlobalBindings {
            start: Some(hotkey("F6", &[])),
            stop: Some(hotkey("F7", &[])),
            pause: Some(hotkey("F8", &[])),
            emergency_stop: Some(hotkey("F12", &[])),
        });
        matcher
    }

    #[test]
    fn function_keys_map_to_the_engine_controls() {
        let mut matcher = matcher();
        let now = Instant::now();
        assert_eq!(matcher.on_key(0x75, false, true, now), vec![HotkeyAction::Start]);
        assert_eq!(matcher.on_key(0x76, false, true, now), vec![HotkeyAction::Stop]);
        assert_eq!(matcher.on_key(0x77, false, true, now), vec![HotkeyAction::Pause]);
        assert_eq!(
            matcher.on_key(0x7B, false, true, now),
            vec![HotkeyAction::EmergencyStop]
        );
    }

    #[test]
    fn key_up_does_not_retrigger_a_global_hotkey() {
        let mut matcher = matcher();
        let now = Instant::now();
        matcher.on_key(0x75, false, true, now);
        assert!(matcher.on_key(0x75, false, false, now).is_empty());
    }

    #[test]
    fn a_bare_hotkey_does_not_fire_while_a_modifier_is_held() {
        let mut matcher = matcher();
        let now = Instant::now();
        matcher.on_key(0xA2, false, true, now); // Ctrl down
        assert!(matcher.on_key(0x75, false, true, now).is_empty());
    }

    #[test]
    fn a_chord_fires_only_with_its_modifier() {
        let mut matcher = Matcher::new();
        matcher.set_macros(vec![MacroBinding {
            macro_id: "m1".into(),
            hotkey: hotkey("KeyG", &[Modifier::Ctrl]),
            activation: Activation::Toggle,
        }]);
        let now = Instant::now();

        assert!(matcher.on_key(0x47, false, true, now).is_empty());
        matcher.on_key(0x47, false, false, now);

        matcher.on_key(0xA2, false, true, now);
        assert_eq!(
            matcher.on_key(0x47, false, true, now),
            vec![HotkeyAction::MacroToggle("m1".into())]
        );
    }

    #[test]
    fn hold_bindings_emit_a_start_and_an_end() {
        let mut matcher = Matcher::new();
        matcher.set_macros(vec![MacroBinding {
            macro_id: "m1".into(),
            hotkey: hotkey("KeyH", &[]),
            activation: Activation::Hold,
        }]);
        let now = Instant::now();

        assert_eq!(
            matcher.on_key(0x48, false, true, now),
            vec![HotkeyAction::MacroHoldStart("m1".into())]
        );
        assert_eq!(
            matcher.on_key(0x48, false, false, now),
            vec![HotkeyAction::MacroHoldEnd("m1".into())]
        );
    }

    #[test]
    fn a_hold_release_still_lands_if_the_modifier_was_let_go_first() {
        let mut matcher = Matcher::new();
        matcher.set_macros(vec![MacroBinding {
            macro_id: "m1".into(),
            hotkey: hotkey("KeyH", &[Modifier::Alt]),
            activation: Activation::Hold,
        }]);
        let now = Instant::now();

        matcher.on_key(0xA4, false, true, now);
        assert_eq!(
            matcher.on_key(0x48, false, true, now),
            vec![HotkeyAction::MacroHoldStart("m1".into())]
        );

        matcher.on_key(0xA4, false, false, now); // Alt released early
        assert_eq!(
            matcher.on_key(0x48, false, false, now),
            vec![HotkeyAction::MacroHoldEnd("m1".into())]
        );
    }

    #[test]
    fn three_escapes_inside_the_window_force_a_stop() {
        let mut matcher = matcher();
        let start = Instant::now();
        // Three real taps: each one has a release, which is what separates a
        // tap from the auto-repeat of a held key.
        assert!(matcher.on_key(VK_ESCAPE, false, true, start).is_empty());
        matcher.on_key(VK_ESCAPE, false, false, start + Duration::from_millis(50));
        assert!(matcher
            .on_key(VK_ESCAPE, false, true, start + Duration::from_millis(100))
            .is_empty());
        matcher.on_key(VK_ESCAPE, false, false, start + Duration::from_millis(150));
        assert_eq!(
            matcher.on_key(VK_ESCAPE, false, true, start + Duration::from_millis(200)),
            vec![HotkeyAction::EmergencyStop]
        );
    }

    #[test]
    fn slow_escapes_do_not_trip_the_panic_stop() {
        let mut matcher = matcher();
        let start = Instant::now();
        matcher.on_key(VK_ESCAPE, false, true, start);
        matcher.on_key(VK_ESCAPE, false, false, start);
        matcher.on_key(VK_ESCAPE, false, true, start + Duration::from_millis(400));
        matcher.on_key(VK_ESCAPE, false, false, start + Duration::from_millis(400));
        let third = matcher.on_key(VK_ESCAPE, false, true, start + Duration::from_millis(1_200));
        assert!(third.is_empty(), "escapes were too far apart to count");
    }

    #[test]
    fn the_panic_stop_can_be_disabled() {
        let mut matcher = matcher();
        matcher.set_panic_enabled(false);
        let start = Instant::now();
        for offset in [0, 80, 160] {
            let at = start + Duration::from_millis(offset);
            assert!(matcher.on_key(VK_ESCAPE, false, true, at).is_empty());
            matcher.on_key(VK_ESCAPE, false, false, at);
        }
    }

    #[test]
    fn auto_repeat_does_not_retrigger_a_toggle() {
        let mut matcher = Matcher::new();
        matcher.set_macros(vec![MacroBinding {
            macro_id: "m1".into(),
            hotkey: hotkey("Delete", &[]),
            activation: Activation::Toggle,
        }]);
        let now = Instant::now();

        assert_eq!(
            matcher.on_key(0x2E, true, true, now),
            vec![HotkeyAction::MacroToggle("m1".into())]
        );
        // Held down: Windows keeps sending key-down. None of them count.
        for step in 1..=8 {
            let at = now + Duration::from_millis(step * 30);
            assert!(
                matcher.on_key(0x2E, true, true, at).is_empty(),
                "repeat {step} retriggered the toggle"
            );
        }
        // Release, then a genuine second press toggles again.
        assert!(matcher.on_key(0x2E, true, false, now).is_empty());
        assert_eq!(
            matcher.on_key(0x2E, true, true, now),
            vec![HotkeyAction::MacroToggle("m1".into())]
        );
    }

    #[test]
    fn a_held_hold_binding_starts_exactly_once() {
        let mut matcher = Matcher::new();
        matcher.set_macros(vec![MacroBinding {
            macro_id: "m1".into(),
            hotkey: hotkey("KeyH", &[]),
            activation: Activation::Hold,
        }]);
        let now = Instant::now();

        assert_eq!(
            matcher.on_key(0x48, false, true, now),
            vec![HotkeyAction::MacroHoldStart("m1".into())]
        );
        assert!(matcher.on_key(0x48, false, true, now).is_empty());
        assert_eq!(
            matcher.on_key(0x48, false, false, now),
            vec![HotkeyAction::MacroHoldEnd("m1".into())]
        );
    }

    #[test]
    fn holding_escape_never_trips_the_panic_stop() {
        let mut matcher = matcher();
        let start = Instant::now();
        matcher.on_key(VK_ESCAPE, false, true, start);
        for step in 1..=10 {
            let at = start + Duration::from_millis(step * 30);
            assert!(
                matcher.on_key(VK_ESCAPE, false, true, at).is_empty(),
                "auto-repeat counted as a tap"
            );
        }
    }

    #[test]
    fn a_lost_key_up_does_not_deafen_the_hotkey_forever() {
        let mut matcher = Matcher::new();
        matcher.set_macros(vec![MacroBinding {
            macro_id: "m1".into(),
            hotkey: hotkey("Delete", &[]),
            activation: Activation::Toggle,
        }]);
        let now = Instant::now();

        assert_eq!(
            matcher.on_key(0x2E, true, true, now),
            vec![HotkeyAction::MacroToggle("m1".into())]
        );

        // The key-up never arrives. Without reconciliation the next press is
        // written off as auto-repeat and the macro can never be stopped.
        assert!(matcher.on_key(0x2E, true, true, now).is_empty());

        let released = |_vk: u16| false;
        assert_eq!(matcher.reconcile_held(&released), 1);

        assert_eq!(
            matcher.on_key(0x2E, true, true, now),
            vec![HotkeyAction::MacroToggle("m1".into())],
            "the second press must still toggle after a lost key-up"
        );
    }

    #[test]
    fn reconciliation_keeps_a_key_that_is_really_still_held() {
        let mut matcher = Matcher::new();
        matcher.set_macros(vec![MacroBinding {
            macro_id: "m1".into(),
            hotkey: hotkey("Delete", &[]),
            activation: Activation::Toggle,
        }]);
        let now = Instant::now();

        matcher.on_key(0x2E, true, true, now);
        let still_down = |_vk: u16| true;
        assert_eq!(matcher.reconcile_held(&still_down), 0);
        assert!(matcher.is_held(0x2E));
        // Auto-repeat is still suppressed.
        assert!(matcher.on_key(0x2E, true, true, now).is_empty());
    }

    #[test]
    fn reconciliation_also_clears_a_stuck_modifier() {
        let mut matcher = matcher();
        let now = Instant::now();

        matcher.on_key(0xA2, false, true, now); // Ctrl down, key-up never seen
        assert!(matcher.on_key(0x75, false, true, now).is_empty(), "bare F6 blocked by Ctrl");
        matcher.on_key(0x75, false, false, now);

        // Nothing is physically down any more: the stuck Ctrl must go.
        let nothing_down = |_vk: u16| false;
        assert_eq!(matcher.reconcile_held(&nothing_down), 1);
        assert_eq!(matcher.on_key(0x75, false, true, now), vec![HotkeyAction::Start]);
    }

    #[test]
    fn a_side_button_toggles_a_macro() {
        let mut matcher = Matcher::new();
        matcher.set_macros(vec![MacroBinding {
            macro_id: "m1".into(),
            hotkey: hotkey("Mouse4", &[]),
            activation: Activation::Toggle,
        }]);

        assert_eq!(
            matcher.on_mouse_button(MouseButton::Mouse4, true),
            vec![HotkeyAction::MacroToggle("m1".into())]
        );
        assert!(matcher.on_mouse_button(MouseButton::Mouse4, false).is_empty());
        // A different button must not fire it.
        assert!(matcher.on_mouse_button(MouseButton::Mouse5, true).is_empty());
    }

    #[test]
    fn the_middle_button_can_drive_a_global_hotkey() {
        let mut matcher = matcher();
        matcher.set_global(GlobalBindings {
            start: Some(hotkey("MouseMiddle", &[])),
            ..GlobalBindings::default()
        });
        assert_eq!(
            matcher.on_mouse_button(MouseButton::Middle, true),
            vec![HotkeyAction::Start]
        );
    }

    #[test]
    fn a_side_button_chord_needs_its_modifier() {
        let mut matcher = Matcher::new();
        matcher.set_macros(vec![MacroBinding {
            macro_id: "m1".into(),
            hotkey: hotkey("Mouse5", &[Modifier::Shift]),
            activation: Activation::Hold,
        }]);
        let now = Instant::now();

        assert!(matcher.on_mouse_button(MouseButton::Mouse5, true).is_empty());
        matcher.on_mouse_button(MouseButton::Mouse5, false);

        matcher.on_key(0xA0, false, true, now); // Shift down
        assert_eq!(
            matcher.on_mouse_button(MouseButton::Mouse5, true),
            vec![HotkeyAction::MacroHoldStart("m1".into())]
        );
        assert_eq!(
            matcher.on_mouse_button(MouseButton::Mouse5, false),
            vec![HotkeyAction::MacroHoldEnd("m1".into())]
        );
    }

    #[test]
    fn a_held_side_button_starts_exactly_once() {
        let mut matcher = Matcher::new();
        matcher.set_macros(vec![MacroBinding {
            macro_id: "m1".into(),
            hotkey: hotkey("Mouse4", &[]),
            activation: Activation::Toggle,
        }]);

        assert_eq!(
            matcher.on_mouse_button(MouseButton::Mouse4, true),
            vec![HotkeyAction::MacroToggle("m1".into())]
        );
        assert!(matcher.on_mouse_button(MouseButton::Mouse4, true).is_empty());
    }

    #[test]
    fn the_left_button_is_never_bindable() {
        let mut matcher = Matcher::new();
        matcher.set_macros(vec![MacroBinding {
            macro_id: "m1".into(),
            hotkey: hotkey("MouseLeft", &[]),
            activation: Activation::Toggle,
        }]);
        assert!(matcher.on_mouse_button(MouseButton::Left, true).is_empty());
        assert!(matcher.on_mouse_button(MouseButton::Right, true).is_empty());
    }

    #[test]
    fn modifier_keys_alone_trigger_nothing() {
        let mut matcher = matcher();
        let now = Instant::now();
        assert!(matcher.on_key(0xA0, false, true, now).is_empty());
        assert!(matcher.on_key(0x5B, true, true, now).is_empty());
    }
}
