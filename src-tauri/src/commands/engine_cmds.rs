//! Engine control surface (§7).

use tauri::State;

use crate::macros::engine::EngineStatus;
use crate::macros::executor;
use crate::macros::model::Action;
use crate::state::AppState;

#[tauri::command]
pub fn engine_start(state: State<'_, AppState>, macro_id: String) -> Result<(), String> {
    state.start_macro(&macro_id)
}

#[tauri::command]
pub fn engine_stop(state: State<'_, AppState>) -> bool {
    state.engine.stop()
}

#[tauri::command]
pub fn engine_pause(state: State<'_, AppState>) -> bool {
    state.engine.pause()
}

#[tauri::command]
pub fn engine_resume(state: State<'_, AppState>) -> bool {
    state.engine.resume()
}

#[tauri::command]
pub fn engine_status(state: State<'_, AppState>) -> EngineStatus {
    state.engine.status()
}

/// Fires a single action so the user can feel what it does before saving.
/// Async so the injection never runs on the UI thread.
#[tauri::command]
pub async fn engine_test_action(action: Action) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        executor::run_single(&action).map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "The test could not be started".to_string())?
    .map(|_| ())
}

/// Puts a macro under hotkey control without running it: the window's Start
/// button arms, the key does the starting and stopping (§19).
#[tauri::command]
pub fn engine_arm(state: State<'_, AppState>, macro_id: String) -> Result<(), String> {
    if state.arm(&macro_id) {
        Ok(())
    } else {
        Err("That macro cannot run yet".into())
    }
}

#[tauri::command]
pub fn engine_disarm(state: State<'_, AppState>) {
    state.disarm();
}

#[tauri::command]
pub fn engine_armed(state: State<'_, AppState>) -> Option<String> {
    state.armed_macro()
}
