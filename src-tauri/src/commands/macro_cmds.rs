//! Macro CRUD, import and export (§21, §23).

use std::path::PathBuf;

use tauri::State;

use crate::config::profiles;
use crate::macros::model::{Macro, MacroExport};
use crate::state::AppState;
use crate::storage::atomic_json::write_atomic;
use crate::util::new_id;

#[tauri::command]
pub fn macro_list(state: State<'_, AppState>) -> Vec<Macro> {
    let store = state.profiles.lock();
    profiles::active_profile(&store)
        .map(|profile| profile.macros.clone())
        .unwrap_or_default()
}

#[tauri::command]
pub fn macro_get(state: State<'_, AppState>, macro_id: String) -> Option<Macro> {
    state.find_macro(&macro_id)
}

#[tauri::command]
pub fn macro_new(name: String) -> Macro {
    profiles::new_macro(name)
}

#[tauri::command]
pub fn macro_save(state: State<'_, AppState>, value: Macro) -> Result<Macro, String> {
    let saved = {
        let mut store = state.profiles.lock();
        let saved = profiles::upsert_macro(&mut store, value).map_err(|error| error.to_string())?;
        profiles::save(&store).map_err(|error| error.to_string())?;
        saved
    };

    state.sync_bindings();
    state.notify_data_changed();
    Ok(saved)
}

#[tauri::command]
pub fn macro_delete(state: State<'_, AppState>, macro_id: String) -> Result<bool, String> {
    let removed = {
        let mut store = state.profiles.lock();
        let removed = profiles::delete_macro(&mut store, &macro_id);
        if removed {
            profiles::save(&store).map_err(|error| error.to_string())?;
        }
        removed
    };

    if removed {
        state.sync_bindings();
        state.notify_data_changed();
    }
    Ok(removed)
}

#[tauri::command]
pub fn macro_duplicate(state: State<'_, AppState>, macro_id: String) -> Result<Macro, String> {
    let Some(mut copy) = state.find_macro(&macro_id) else {
        return Err("That macro no longer exists".into());
    };

    copy.id = new_id("macro");
    copy.name = format!("{} copy", copy.name);
    // A duplicate must not inherit the shortcut; two macros cannot share one.
    copy.hotkey = None;
    copy.stats = Default::default();
    copy.created_at = String::new();

    macro_save(state, copy)
}

#[tauri::command]
pub fn macro_export(
    state: State<'_, AppState>,
    macro_id: String,
    path: PathBuf,
) -> Result<(), String> {
    let export: MacroExport = {
        let store = state.profiles.lock();
        profiles::export_macro(&store, &macro_id).ok_or("That macro no longer exists")?
    };
    write_atomic(&path, &export).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn macro_import(state: State<'_, AppState>, path: PathBuf) -> Result<Macro, String> {
    let imported = profiles::import_macro(&path).map_err(|error| error.to_string())?;
    macro_save(state, imported)
}
