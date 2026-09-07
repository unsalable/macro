//! Native window chrome tweaks that Tauri does not expose (§34).

#[cfg(windows)]
pub fn apply(window: &tauri::WebviewWindow) {
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        DWM_WINDOW_CORNER_PREFERENCE,
    };

    let Ok(raw) = window.hwnd() else { return };
    // Rebuild the handle locally so a `windows` crate version mismatch with
    // Tauri's own dependency cannot break the build.
    let hwnd = HWND(raw.0 as isize as *mut c_void);
    let preference = DWMWCP_ROUND;

    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::addr_of!(preference).cast::<c_void>(),
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
}

#[cfg(not(windows))]
pub fn apply(_window: &tauri::WebviewWindow) {}
