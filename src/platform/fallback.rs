/// No-op: no Dock icon equivalent on this platform.
pub fn set_accessory_policy() {}

/// No-op: overlay setup not available.
pub unsafe fn setup_overlay(_handle: *mut std::ffi::c_void) {}

/// No-op: overlay show not available.
pub unsafe fn show_no_activate(_handle: *mut std::ffi::c_void) {}

/// No-op: overlay hide not available.
pub unsafe fn hide_window(_handle: *mut std::ffi::c_void) {}

/// Paste simulation not available on this platform.
pub unsafe fn simulate_paste() -> bool {
    false
}

/// Copy simulation not available on this platform.
pub unsafe fn simulate_copy() -> bool {
    false
}

/// Undo simulation not available on this platform.
pub unsafe fn simulate_undo() -> bool {
    false
}
