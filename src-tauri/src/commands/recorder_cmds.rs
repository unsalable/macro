//! Recording control (§16, §17).

use serde::Serialize;
use tauri::State;

use crate::config::profiles;
use crate::macros::model::Macro;
use crate::recorder::convert::{to_actions, ConvertOptions, RecordedEvent};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecorderStatus {
    pub recording: bool,
    pub elapsed_ms: u64,
}

#[tauri::command]
pub fn recorder_status(state: State<'_, AppState>) -> RecorderStatus {
    RecorderStatus {
        recording: state.recorder.is_recording(),
        elapsed_ms: state.recorder.elapsed_ms(),
    }
}

#[tauri::command]
pub fn recorder_start(state: State<'_, AppState>) -> Result<(), String> {
    if state.recorder.is_recording() {
        return Err("A recording is already in progress".into());
    }
    if state.start_recording() {
        Ok(())
    } else {
        Err("The input hook is not available".into())
    }
}

#[tauri::command]
pub fn recorder_stop(state: State<'_, AppState>) -> Vec<RecordedEvent> {
    state.stop_recording()
}

/// Converts a recording into an unsaved macro the editor can open (§17).
#[tauri::command]
pub fn recorder_to_macro(name: String, events: Vec<RecordedEvent>, include_moves: bool) -> Macro {
    let mut value = profiles::new_macro(name);
    value.actions = to_actions(
        &events,
        ConvertOptions { include_moves, ..ConvertOptions::default() },
    );
    value
}
