//! System tray icon and menu (§43).

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::macros::control::EngineState;
use crate::state::AppState;

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open FlowMacro", true, None::<&str>)?;
    let start = MenuItem::with_id(app, "start", "Start", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause / Resume", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "Stop", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &open,
            &PredefinedMenuItem::separator(app)?,
            &start,
            &pause,
            &stop,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("flowmacro")
        .tooltip("FlowMacro")
        .menu(&menu)
        // Left click opens the window; the menu belongs on right click, which
        // is what Windows users expect.
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id.as_ref() {
        "open" => show_main_window(app),
        "start" => {
            if let Some(state) = app.try_state::<AppState>() {
                if let Some(value) = state.default_macro() {
                    let _ = state.start_macro(&value.id);
                }
            }
        }
        "pause" => {
            if let Some(state) = app.try_state::<AppState>() {
                if state.engine.state() == EngineState::Paused {
                    state.engine.resume();
                } else {
                    state.engine.pause();
                }
            }
        }
        "stop" => {
            if let Some(state) = app.try_state::<AppState>() {
                state.engine.stop();
            }
        }
        "quit" => {
            if let Some(state) = app.try_state::<AppState>() {
                state.engine.stop();
            }
            app.exit(0);
        }
        _ => {}
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
