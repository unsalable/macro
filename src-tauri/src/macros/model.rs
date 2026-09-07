//! Macro data model. Mirrors `src/types/macro.ts` one-to-one.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

/// Hard ceilings so a malformed import cannot lock the machine up (§40).
pub const MAX_ACTIONS: usize = 2_000;
pub const MAX_NESTING: usize = 4;
pub const MAX_TEXT_LEN: usize = 4_000;
pub const MIN_INTERVAL_MS: u32 = 1;
pub const MAX_INTERVAL_MS: u32 = 3_600_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Mouse4,
    Mouse5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClickMode {
    Single,
    Double,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
    Win,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MoveMode {
    Absolute,
    Relative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MoveCurve {
    Linear,
    Smooth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScrollDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyPhase {
    Press,
    Down,
    Up,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum ActionKind {
    MouseClick {
        button: MouseButton,
        mode: ClickMode,
        count: u32,
        interval_ms: u32,
        duration_ms: u32,
    },
    MouseMove {
        mode: MoveMode,
        x: i32,
        y: i32,
        duration_ms: u32,
        curve: MoveCurve,
    },
    MouseScroll {
        direction: ScrollDirection,
        amount: u32,
        interval_ms: u32,
    },
    Key {
        action: KeyPhase,
        code: String,
        modifiers: Vec<Modifier>,
        duration_ms: u32,
    },
    Text {
        value: String,
        per_char_delay_ms: u32,
    },
    Delay {
        duration_ms: u32,
    },
    Repeat {
        times: u32,
        actions: Vec<Action>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub id: String,
    #[serde(default)]
    pub delay_before_ms: u32,
    #[serde(default)]
    pub delay_after_ms: u32,
    #[serde(flatten)]
    pub kind: ActionKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hotkey {
    pub code: String,
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Activation {
    Toggle,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoopMode {
    None,
    Infinite,
    Count,
    Duration,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopConfig {
    pub mode: LoopMode,
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub duration_ms: u64,
    /// Gap between two iterations of the loop. 0 means "as fast as the actions
    /// allow"; the engine still keeps a small floor so a zero-delay infinite
    /// loop cannot saturate the input queue.
    #[serde(default)]
    pub interval_ms: u32,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self { mode: LoopMode::None, count: 1, duration_ms: 0, interval_ms: 0 }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Randomization {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub jitter_ms: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroStats {
    #[serde(default)]
    pub run_count: u64,
    #[serde(default)]
    pub last_run_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Macro {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub hotkey: Option<Hotkey>,
    #[serde(default = "default_activation")]
    pub activation: Activation,
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(rename = "loop", default)]
    pub loop_config: LoopConfig,
    #[serde(default)]
    pub randomization: Randomization,
    #[serde(default)]
    pub stats: MacroStats,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

fn default_true() -> bool {
    true
}

fn default_activation() -> Activation {
    Activation::Toggle
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProfileColor {
    Sand,
    Clay,
    Sage,
    Slate,
    Plum,
    Ochre,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub color: ProfileColor,
    #[serde(default)]
    pub macros: Vec<Macro>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileStore {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub active_profile_id: String,
    #[serde(default)]
    pub profiles: Vec<Profile>,
}

impl Default for ProfileStore {
    fn default() -> Self {
        let profile = Profile {
            id: "default".into(),
            name: "Default".into(),
            color: ProfileColor::Sand,
            macros: Vec::new(),
        };
        Self {
            schema_version: SCHEMA_VERSION,
            active_profile_id: profile.id.clone(),
            profiles: vec![profile],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroExport {
    pub schema_version: u32,
    pub kind: String,
    pub exported_at: String,
    #[serde(rename = "macro")]
    pub macro_data: Macro,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loop_saved_before_the_interval_existed_still_loads() {
        let value: LoopConfig =
            serde_json::from_str(r#"{"mode":"infinite","count":0,"durationMs":0}"#).expect("parse");
        assert_eq!(value.interval_ms, 0);
    }

    #[test]
    fn the_loop_interval_round_trips_through_json() {
        let value = LoopConfig {
            mode: LoopMode::Infinite,
            count: 0,
            duration_ms: 0,
            interval_ms: 40,
        };
        let text = serde_json::to_string(&value).expect("encode");
        assert!(text.contains("\"intervalMs\":40"), "unexpected json: {text}");

        let parsed: LoopConfig = serde_json::from_str(&text).expect("decode");
        assert_eq!(parsed.interval_ms, 40);
    }
}
