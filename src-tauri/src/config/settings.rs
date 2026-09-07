//! Application settings. Mirrors `src/types/settings.ts`.

use serde::{Deserialize, Serialize};

use crate::macros::model::{Hotkey, SCHEMA_VERSION};
use crate::storage::atomic_json::{data_file, read_or_default, write_atomic, StorageResult};

pub const FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    En,
    Tr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeMode {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationQuality {
    Low,
    Balanced,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyMap {
    #[serde(default)]
    pub start: Option<Hotkey>,
    #[serde(default)]
    pub stop: Option<Hotkey>,
    #[serde(default)]
    pub pause: Option<Hotkey>,
    #[serde(default)]
    pub emergency_stop: Option<Hotkey>,
}

impl Default for HotkeyMap {
    fn default() -> Self {
        Self {
            start: Some(Hotkey { code: "F6".into(), modifiers: Vec::new() }),
            stop: Some(Hotkey { code: "F7".into(), modifiers: Vec::new() }),
            pause: Some(Hotkey { code: "F8".into(), modifiers: Vec::new() }),
            emergency_stop: Some(Hotkey { code: "F12".into(), modifiers: Vec::new() }),
        }
    }
}

/// `#[serde(default)]` on every field: an older file simply misses the new
/// keys and still loads (§2.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default = "default_language")]
    pub language: Language,
    #[serde(default = "default_theme")]
    pub theme: ThemeMode,
    #[serde(default = "default_quality")]
    pub animation_quality: AnimationQuality,
    #[serde(default)]
    pub start_with_windows: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    #[serde(default = "default_true")]
    pub show_tray_notifications: bool,
    #[serde(default = "default_true")]
    pub confirm_before_delete: bool,
    #[serde(default = "default_true")]
    pub panic_escape_enabled: bool,
    #[serde(default)]
    pub hotkeys: HotkeyMap,
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,
    #[serde(default)]
    pub onboarding_done: bool,
}

fn default_language() -> Language {
    Language::En
}

fn default_theme() -> ThemeMode {
    ThemeMode::System
}

fn default_quality() -> AnimationQuality {
    AnimationQuality::Balanced
}

fn default_true() -> bool {
    true
}

fn default_history_limit() -> usize {
    crate::storage::history::DEFAULT_LIMIT
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            language: default_language(),
            theme: default_theme(),
            animation_quality: default_quality(),
            start_with_windows: false,
            start_minimized: false,
            close_to_tray: true,
            show_tray_notifications: true,
            confirm_before_delete: true,
            panic_escape_enabled: true,
            hotkeys: HotkeyMap::default(),
            history_limit: default_history_limit(),
            onboarding_done: false,
        }
    }
}

impl AppSettings {
    /// Keeps values inside the range the UI can actually represent.
    pub fn normalize(&mut self) {
        self.schema_version = SCHEMA_VERSION;
        self.history_limit = self.history_limit.clamp(50, 5_000);
    }
}

pub fn load() -> StorageResult<AppSettings> {
    let path = data_file(FILE_NAME)?;
    let mut settings: AppSettings = read_or_default(&path)?;
    settings.normalize();
    Ok(settings)
}

pub fn save(settings: &AppSettings) -> StorageResult<()> {
    let path = data_file(FILE_NAME)?;
    write_atomic(&path, settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_documented_hotkeys() {
        let settings = AppSettings::default();
        assert_eq!(settings.hotkeys.start.as_ref().map(|h| h.code.as_str()), Some("F6"));
        assert_eq!(settings.hotkeys.stop.as_ref().map(|h| h.code.as_str()), Some("F7"));
        assert_eq!(settings.hotkeys.pause.as_ref().map(|h| h.code.as_str()), Some("F8"));
        assert_eq!(
            settings.hotkeys.emergency_stop.as_ref().map(|h| h.code.as_str()),
            Some("F12")
        );
        assert!(settings.panic_escape_enabled);
    }

    #[test]
    fn a_partial_file_fills_in_defaults() {
        let raw = r#"{ "language": "tr", "theme": "dark" }"#;
        let mut settings: AppSettings = serde_json::from_str(raw).expect("parse");
        settings.normalize();

        assert_eq!(settings.language, Language::Tr);
        assert_eq!(settings.theme, ThemeMode::Dark);
        assert_eq!(settings.animation_quality, AnimationQuality::Balanced);
        assert!(settings.close_to_tray);
        assert_eq!(settings.history_limit, crate::storage::history::DEFAULT_LIMIT);
    }

    #[test]
    fn round_trips_through_json_in_camel_case() {
        let settings = AppSettings::default();
        let json = serde_json::to_string(&settings).expect("serialize");
        assert!(json.contains("\"startWithWindows\""));
        assert!(json.contains("\"emergencyStop\""));

        let parsed: AppSettings = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed, settings);
    }

    #[test]
    fn history_limit_is_clamped() {
        let mut settings = AppSettings { history_limit: 5, ..AppSettings::default() };
        settings.normalize();
        assert_eq!(settings.history_limit, 50);

        let mut settings = AppSettings { history_limit: 900_000, ..AppSettings::default() };
        settings.normalize();
        assert_eq!(settings.history_limit, 5_000);
    }
}
