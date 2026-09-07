//! Profiles and the macros inside them (§22).
//!
//! Everything lives in one `profiles.json`: the data is small, and a single
//! file keeps import/export and atomic writes trivial.

use std::path::Path;

use crate::macros::model::{Macro, MacroExport, Profile, ProfileStore, SCHEMA_VERSION};
use crate::macros::validate::{normalize_macro, ValidationError};
use crate::storage::atomic_json::{
    data_file, read_or_default, read_strict, write_atomic, StorageResult,
};
use crate::util::{new_id, now_iso};

pub const FILE_NAME: &str = "profiles.json";

pub fn load() -> StorageResult<ProfileStore> {
    let path = data_file(FILE_NAME)?;
    let mut store: ProfileStore = read_or_default(&path)?;
    repair(&mut store);
    Ok(store)
}

pub fn save(store: &ProfileStore) -> StorageResult<()> {
    let path = data_file(FILE_NAME)?;
    write_atomic(&path, store)
}

/// Guarantees the invariants the rest of the app relies on: at least one
/// profile, and an `active_profile_id` that actually resolves.
pub fn repair(store: &mut ProfileStore) {
    store.schema_version = SCHEMA_VERSION;

    if store.profiles.is_empty() {
        store.profiles = ProfileStore::default().profiles;
    }

    let active_exists = store
        .profiles
        .iter()
        .any(|profile| profile.id == store.active_profile_id);
    if !active_exists {
        store.active_profile_id = store
            .profiles
            .first()
            .map(|profile| profile.id.clone())
            .unwrap_or_default();
    }
}

pub fn active_profile(store: &ProfileStore) -> Option<&Profile> {
    store
        .profiles
        .iter()
        .find(|profile| profile.id == store.active_profile_id)
}

pub fn active_profile_mut(store: &mut ProfileStore) -> Option<&mut Profile> {
    let id = store.active_profile_id.clone();
    store.profiles.iter_mut().find(|profile| profile.id == id)
}

pub fn find_macro<'a>(store: &'a ProfileStore, macro_id: &str) -> Option<&'a Macro> {
    store
        .profiles
        .iter()
        .flat_map(|profile| profile.macros.iter())
        .find(|item| item.id == macro_id)
}

/// Inserts or replaces a macro inside the active profile.
pub fn upsert_macro(store: &mut ProfileStore, mut value: Macro) -> Result<Macro, ValidationError> {
    normalize_macro(&mut value)?;
    value.updated_at = now_iso();
    if value.created_at.is_empty() {
        value.created_at = value.updated_at.clone();
    }

    let Some(profile) = active_profile_mut(store) else {
        return Err(ValidationError {
            field: "profile".into(),
            message: "No active profile".into(),
        });
    };

    match profile.macros.iter_mut().find(|item| item.id == value.id) {
        Some(existing) => *existing = value.clone(),
        None => profile.macros.push(value.clone()),
    }

    Ok(value)
}

pub fn delete_macro(store: &mut ProfileStore, macro_id: &str) -> bool {
    let mut removed = false;
    for profile in store.profiles.iter_mut() {
        let before = profile.macros.len();
        profile.macros.retain(|item| item.id != macro_id);
        removed |= profile.macros.len() != before;
    }
    removed
}

pub fn export_macro(store: &ProfileStore, macro_id: &str) -> Option<MacroExport> {
    find_macro(store, macro_id).map(|value| MacroExport {
        schema_version: SCHEMA_VERSION,
        kind: "macro".into(),
        exported_at: now_iso(),
        macro_data: value.clone(),
    })
}

/// Reads an export file. A new id is assigned so importing twice does not
/// silently overwrite the original (§23).
pub fn import_macro(path: &Path) -> Result<Macro, ImportError> {
    let export: MacroExport = read_strict(path).map_err(|_| ImportError::NotAnExport)?;

    if export.kind != "macro" {
        return Err(ImportError::NotAnExport);
    }
    if export.schema_version > SCHEMA_VERSION {
        return Err(ImportError::NewerSchema(export.schema_version));
    }

    let mut value = export.macro_data;
    value.id = new_id("macro");
    value.hotkey = None; // Never steal a shortcut the user already assigned.
    value.stats = Default::default();
    value.created_at = now_iso();
    value.updated_at = value.created_at.clone();
    normalize_macro(&mut value).map_err(ImportError::Invalid)?;

    Ok(value)
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("This file is not a FlowMacro export")]
    NotAnExport,
    #[error("This file was made by a newer version of FlowMacro (schema {0})")]
    NewerSchema(u32),
    #[error("{0}")]
    Invalid(ValidationError),
}

pub fn new_macro(name: impl Into<String>) -> Macro {
    let now = now_iso();
    Macro {
        id: new_id("macro"),
        name: name.into(),
        enabled: true,
        hotkey: None,
        activation: crate::macros::model::Activation::Toggle,
        actions: Vec::new(),
        loop_config: Default::default(),
        randomization: Default::default(),
        stats: Default::default(),
        created_at: now.clone(),
        updated_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::model::ProfileColor;

    fn store_with_one_profile() -> ProfileStore {
        ProfileStore {
            schema_version: SCHEMA_VERSION,
            active_profile_id: "p1".into(),
            profiles: vec![Profile {
                id: "p1".into(),
                name: "Gaming".into(),
                color: ProfileColor::Sage,
                macros: Vec::new(),
            }],
        }
    }

    #[test]
    fn repair_restores_a_missing_active_profile() {
        let mut store = store_with_one_profile();
        store.active_profile_id = "gone".into();
        repair(&mut store);
        assert_eq!(store.active_profile_id, "p1");
    }

    #[test]
    fn repair_creates_a_profile_when_none_exist() {
        let mut store = ProfileStore {
            schema_version: 0,
            active_profile_id: String::new(),
            profiles: Vec::new(),
        };
        repair(&mut store);
        assert_eq!(store.profiles.len(), 1);
        assert_eq!(store.active_profile_id, store.profiles[0].id);
    }

    #[test]
    fn upsert_adds_then_replaces() {
        let mut store = store_with_one_profile();
        let mut value = new_macro("Click fast");
        value.id = "m1".into();

        upsert_macro(&mut store, value.clone()).expect("insert");
        assert_eq!(store.profiles[0].macros.len(), 1);

        value.name = "Click faster".into();
        upsert_macro(&mut store, value).expect("update");
        assert_eq!(store.profiles[0].macros.len(), 1);
        assert_eq!(store.profiles[0].macros[0].name, "Click faster");
    }

    #[test]
    fn upsert_stamps_timestamps() {
        let mut store = store_with_one_profile();
        let mut value = new_macro("Timed");
        value.created_at = String::new();
        value.updated_at = String::new();

        let saved = upsert_macro(&mut store, value).expect("insert");
        assert!(!saved.created_at.is_empty());
        assert_eq!(saved.created_at, saved.updated_at);
    }

    #[test]
    fn delete_reports_whether_anything_was_removed() {
        let mut store = store_with_one_profile();
        let mut value = new_macro("Doomed");
        value.id = "m1".into();
        upsert_macro(&mut store, value).expect("insert");

        assert!(delete_macro(&mut store, "m1"));
        assert!(!delete_macro(&mut store, "m1"));
    }

    #[test]
    fn export_wraps_the_macro_with_schema_metadata() {
        let mut store = store_with_one_profile();
        let mut value = new_macro("Shareable");
        value.id = "m1".into();
        upsert_macro(&mut store, value).expect("insert");

        let export = export_macro(&store, "m1").expect("export");
        assert_eq!(export.kind, "macro");
        assert_eq!(export.schema_version, SCHEMA_VERSION);
        assert_eq!(export.macro_data.name, "Shareable");
    }

    #[test]
    fn import_rejects_a_foreign_file() {
        let dir = std::env::temp_dir().join("flowmacro-tests");
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("not-an-export.json");
        std::fs::write(&path, br#"{"hello":"world"}"#).expect("seed");

        assert!(matches!(import_macro(&path), Err(ImportError::NotAnExport)));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn import_assigns_a_fresh_id_and_clears_the_hotkey() {
        let dir = std::env::temp_dir().join("flowmacro-tests");
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("valid-export.json");

        let mut store = store_with_one_profile();
        let mut value = new_macro("Imported");
        value.id = "m1".into();
        value.hotkey = Some(crate::macros::model::Hotkey {
            code: "F6".into(),
            modifiers: Vec::new(),
        });
        upsert_macro(&mut store, value).expect("insert");
        let export = export_macro(&store, "m1").expect("export");
        std::fs::write(&path, serde_json::to_vec(&export).expect("json")).expect("seed");

        let imported = import_macro(&path).expect("import");
        assert_ne!(imported.id, "m1");
        assert!(imported.hotkey.is_none());
        assert_eq!(imported.name, "Imported");
        let _ = std::fs::remove_file(&path);
    }
}
