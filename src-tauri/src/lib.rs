pub mod commands;
pub mod config;
pub mod hotkeys;
pub mod input;
pub mod macros;
pub mod recorder;
pub mod state;
pub mod storage;
pub mod tray;
pub mod util;
mod window_effects;

use std::sync::Arc;

use tauri::{Manager, WindowEvent};

use crate::hotkeys::registry::Registry;
use crate::state::AppState;

pub fn run() {
    tauri::Builder::default()
        // First, before anything else can take a lock on the data folder: a
        // second copy of FlowMacro would install its own hooks and fight the
        // first over settings.json, so the newcomer hands its arguments to
        // the running instance and exits (§2.1).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                // The first instance may be sitting in the tray, which is
                // exactly when a user tries to launch it again.
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            commands::engine_cmds::engine_start,
            commands::engine_cmds::engine_stop,
            commands::engine_cmds::engine_pause,
            commands::engine_cmds::engine_resume,
            commands::engine_cmds::engine_status,
            commands::engine_cmds::engine_test_action,
            commands::engine_cmds::engine_arm,
            commands::engine_cmds::engine_disarm,
            commands::engine_cmds::engine_armed,
            commands::macro_cmds::macro_list,
            commands::macro_cmds::macro_get,
            commands::macro_cmds::macro_new,
            commands::macro_cmds::macro_save,
            commands::macro_cmds::macro_delete,
            commands::macro_cmds::macro_duplicate,
            commands::macro_cmds::macro_export,
            commands::macro_cmds::macro_import,
            commands::config_cmds::settings_get,
            commands::config_cmds::settings_set,
            commands::config_cmds::profile_list,
            commands::config_cmds::profile_new,
            commands::config_cmds::profile_save,
            commands::config_cmds::profile_delete,
            commands::config_cmds::profile_switch,
            commands::config_cmds::history_list,
            commands::config_cmds::history_clear,
            commands::recorder_cmds::recorder_status,
            commands::recorder_cmds::recorder_start,
            commands::recorder_cmds::recorder_stop,
            commands::recorder_cmds::recorder_to_macro,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            app.manage(AppState::load(handle.clone()));

            // The hook is installed only after the state exists, so an event
            // arriving on the very first millisecond still finds somewhere to go.
            let registry = Registry::start(Arc::new(move |action| {
                if let Some(state) = handle.try_state::<AppState>() {
                    state.handle_hotkey(action);
                }
            }));
            app.state::<AppState>().attach_registry(registry);

            tray::setup(app.handle())?;

            let window = app
                .get_webview_window("main")
                .expect("main window is declared in tauri.conf.json");
            window_effects::apply(&window);

            let start_minimized = app.state::<AppState>().settings.lock().start_minimized;
            if !start_minimized {
                window.show()?;
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let close_to_tray = app
                    .try_state::<AppState>()
                    .map(|state| state.settings.lock().close_to_tray)
                    .unwrap_or(false);

                // Closing the window always stops the run, tray or not: an
                // auto-clicker still firing behind a hidden window is the one
                // state the user has no way out of.
                if let Some(state) = app.try_state::<AppState>() {
                    state.engine.stop();
                }

                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running FlowMacro");
}
