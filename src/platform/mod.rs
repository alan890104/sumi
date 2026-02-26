#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub mod fallback;

/// Hide the Dock icon (macOS) or equivalent.
pub fn set_app_accessory_mode() {
    #[cfg(target_os = "macos")]
    unsafe {
        macos::set_accessory_policy();
    }
    #[cfg(target_os = "windows")]
    windows::set_accessory_policy();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    fallback::set_accessory_policy();
}

/// One-time overlay setup (non-activating, always-on-top, all Spaces).
pub fn setup_overlay_window(overlay: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    if let Ok(ns_win) = overlay.ns_window() {
        unsafe { macos::setup_overlay(ns_win); }
    }
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = overlay.hwnd() {
        unsafe { windows::setup_overlay(hwnd.0); }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = overlay;
    }
}

/// Show overlay without activating.
pub fn show_overlay(overlay: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    if let Ok(ns_win) = overlay.ns_window() {
        unsafe { macos::show_no_activate(ns_win); }
    }
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = overlay.hwnd() {
        unsafe { windows::show_no_activate(hwnd.0); }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = overlay.show();
    }
}

/// Hide overlay.
pub fn hide_overlay(overlay: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    if let Ok(ns_win) = overlay.ns_window() {
        unsafe { macos::hide_window(ns_win); }
    }
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = overlay.hwnd() {
        unsafe { windows::hide_window(hwnd.0); }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = overlay.hide();
    }
}

/// Simulate paste (Cmd+V on macOS, Ctrl+V on Windows).
pub fn simulate_paste() -> bool {
    #[cfg(target_os = "macos")]
    { unsafe { macos::simulate_cmd_v() } }
    #[cfg(target_os = "windows")]
    { unsafe { windows::simulate_paste() } }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { false }
}

/// Simulate copy (Cmd+C on macOS, Ctrl+C on Windows).
pub fn simulate_copy() -> bool {
    #[cfg(target_os = "macos")]
    { unsafe { macos::simulate_cmd_c() } }
    #[cfg(target_os = "windows")]
    { unsafe { windows::simulate_copy() } }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { false }
}

/// Simulate undo (Cmd+Z on macOS, Ctrl+Z on Windows).
pub fn simulate_undo() -> bool {
    #[cfg(target_os = "macos")]
    { unsafe { macos::simulate_cmd_z() } }
    #[cfg(target_os = "windows")]
    { unsafe { windows::simulate_undo() } }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { false }
}
