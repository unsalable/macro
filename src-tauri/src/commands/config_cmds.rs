//! Settings, profiles and history (§22, §24, §25).

use tauri::State;

use crate::config::{profiles, settings};
use crate::config::settings::AppSettings;
use crate::macros::model::{Profile, ProfileColor, ProfileStore};
use crate::state::AppState;
use crate::storage::history::{self, HistoryEntry};
use crate::util::new_id;

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> AppSettings {
    state.settings.lock().clone()
}

#[tauri::command]
pub fn settings_set(state: State<'_, AppState>, value: AppSettings) -> Result<AppSettings, String> {
    let saved = {
        let mut current = state.settings.lock();
        *current = value;
        current.normalize();
        settings::save(&current).map_err(|error| error.to_string())?;
        current.clone()
    };

    state.sync_bindings();
    Ok(saved)
}

#[tauri::command]
pub fn profile_list(state: State<'_, AppState>) -> ProfileStore {
    state.profiles.lock().clone()
}

#[tauri::command]
pub fn profile_save(state: State<'_, AppState>, value: Profile) -> Result<Profile, String> {
    let saved = {
        let mut store = state.profiles.lock();
        match store.profiles.iter_mut().find(|item| item.id == value.id) {
            Some(existing) => {
                // Macros are edited through the macro commands; a profile save
                // must not clobber them with a stale copy from the UI.
                existing.name = value.name.clone();
                existing.color = value.color;
                existing.clone()
            }
            None => {
                store.profiles.push(value.clone());
                value
            }
        }
    };

    let store = state.profiles.lock();
    profiles::save(&store).map_err(|error| error.to_string())?;
    drop(store);

    state.notify_data_changed();
    Ok(saved)
}

#[tauri::command]
pub fn profile_new(name: String, color: ProfileColor) -> Profile {
    Profile {
        id: new_id("prof"),
        name,
        color,
        macros: Vec::new(),
    }
}

#[tauri::command]
pub fn profile_delete(state: State<'_, AppState>, profile_id: String) -> Result<bool, String> {
    let removed = {
        let mut store = state.profiles.lock();
        // The last profile is never deleted; the app always needs one.
        if store.profiles.len() <= 1 {
            return Err("The last profile cannot be deleted".into());
        }
        let before = store.profiles.len();
        store.profiles.retain(|item| item.id != profile_id);
        let removed = store.profiles.len() != before;
        profiles::repair(&mut store);
        profiles::save(&store).map_err(|error| error.to_string())?;
        removed
    };

    if removed {
        state.sync_bindings();
        state.notify_data_changed();
    }
    Ok(removed)
}

#[tauri::command]
pub fn profile_switch(state: State<'_, AppState>, profile_id: String) -> Result<(), String> {
    {
        let mut store = state.profiles.lock();
        if !store.profiles.iter().any(|item| item.id == profile_id) {
            return Err("That profile no longer exists".into());
        }
        store.active_profile_id = profile_id;
        profiles::save(&store).map_err(|error| error.to_string())?;
    }

    // Switching profiles swaps the whole set of macro shortcuts (§22).
    state.sync_bindings();
    state.notify_data_changed();
    Ok(())
}

#[tauri::command]
pub fn history_list(state: State<'_, AppState>, limit: Option<usize>) -> Vec<HistoryEntry> {
    let fallback = state.settings.lock().history_limit;
    history::list(limit.unwrap_or(fallback)).unwrap_or_default()
}

#[tauri::command]
pub fn history_clear() -> Result<(), String> {
    history::clear().map_err(|error| error.to_string())
}
