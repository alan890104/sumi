use std::ffi::c_void;

extern "C" {
    fn sel_registerName(name: *const u8) -> *mut c_void;
    fn objc_msgSend();
    fn objc_getClass(name: *const u8) -> *mut c_void;
    fn objc_allocateClassPair(
        superclass: *mut c_void,
        name: *const u8,
        extra_bytes: usize,
    ) -> *mut c_void;
    fn objc_registerClassPair(cls: *mut c_void);
    fn object_setClass(obj: *mut c_void, cls: *mut c_void) -> *mut c_void;
}

/// Hide the Dock icon by setting the activation policy to Accessory.
/// NSApplicationActivationPolicyAccessory = 1
pub unsafe fn set_accessory_policy() {
    let cls_name = b"NSApplication\0";
    let sel_shared = sel_registerName(b"sharedApplication\0".as_ptr());
    let send_shared: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());

    let class_sel = sel_registerName(b"class\0".as_ptr());
    let _ = class_sel;

    let ns_app_class = objc_getClass(cls_name.as_ptr());
    if ns_app_class.is_null() {
        return;
    }
    let ns_app = send_shared(ns_app_class, sel_shared);
    if ns_app.is_null() {
        return;
    }

    let sel_policy = sel_registerName(b"setActivationPolicy:\0".as_ptr());
    let send_policy: unsafe extern "C" fn(*mut c_void, *mut c_void, i64) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send_policy(ns_app, sel_policy, 1); // 1 = Accessory
}

/// Enable dragging the main window by its background.
///
/// With `titleBarStyle: "Overlay"` + `resizable: true`, macOS no longer
/// provides a native title-bar drag region because the web content covers
/// the full window.  Setting `isMovableByWindowBackground = YES` lets the
/// user drag the window by clicking anywhere that is not an interactive
/// control.
pub unsafe fn set_movable_by_background(ns_window: *mut c_void) {
    let sel = sel_registerName(b"setMovableByWindowBackground:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, 1); // YES
}

/// Collection behavior flags for the overlay window.
const OVERLAY_BEHAVIOR: u64 = 1    // canJoinAllSpaces
                            | 8    // transient
                            | 64   // ignoresCycle
                            | 256; // fullScreenAuxiliary

/// Swizzle the Tauri NSWindow into an NSPanel subclass.
///
/// macOS fullscreen Spaces only allow **NSPanel** (not NSWindow) to
/// appear alongside the fullscreen app.  We create a one-off
/// runtime class that inherits from NSPanel and swap the window's
/// isa pointer so the window server treats it as a panel.
unsafe fn make_panel(ns_window: *mut c_void) {
    let panel_class_name = b"SumiOverlayPanel\0".as_ptr();
    let mut cls = objc_getClass(panel_class_name);
    if cls.is_null() {
        let ns_panel = objc_getClass(b"NSPanel\0".as_ptr());
        if ns_panel.is_null() {
            return;
        }
        cls = objc_allocateClassPair(ns_panel, panel_class_name, 0);
        if cls.is_null() {
            return;
        }
        objc_registerClassPair(cls);
    }
    object_setClass(ns_window, cls);

    // NSPanel-specific: don't become key unless user explicitly clicks
    let sel = sel_registerName(b"setBecomesKeyOnlyIfNeeded:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, 1);

    // NSPanel-specific: treat as a floating panel
    let sel = sel_registerName(b"setFloatingPanel:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, 1);

    // Add non-activating panel to style mask (bit 7 = 128)
    let sel_mask = sel_registerName(b"styleMask\0".as_ptr());
    let get_mask: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u64 =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    let mask = get_mask(ns_window, sel_mask);

    let sel_set = sel_registerName(b"setStyleMask:\0".as_ptr());
    let set_mask: unsafe extern "C" fn(*mut c_void, *mut c_void, u64) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    set_mask(ns_window, sel_set, mask | (1 << 7)); // NSWindowStyleMaskNonactivatingPanel
}

/// One-time setup: convert to NSPanel, floating level, stays visible
/// when app deactivates, joins all Spaces (including fullscreen).
pub unsafe fn setup_overlay(ns_window: *mut c_void) {
    // Convert NSWindow → NSPanel so it can appear in fullscreen Spaces
    make_panel(ns_window);

    // setLevel: kCGPopUpMenuWindowLevel (101) — above fullscreen windows
    let sel = sel_registerName(b"setLevel:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, i64) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, 101);

    // setHidesOnDeactivate: NO
    let sel = sel_registerName(b"setHidesOnDeactivate:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, 0);

    // setCollectionBehavior
    let sel = sel_registerName(b"setCollectionBehavior:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, u64) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, OVERLAY_BEHAVIOR);

    // Register with window server immediately (alpha=0 so invisible),
    // ensuring the window joins all Spaces from the start.
    let sel = sel_registerName(b"setAlphaValue:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, f64) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, 0.0);

    let sel = sel_registerName(b"setIgnoresMouseEvents:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, 1);

    // Order front while invisible to register with all Spaces immediately
    let sel = sel_registerName(b"orderFrontRegardless\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel);

}

/// Show without activating the application.
pub unsafe fn show_no_activate(ns_window: *mut c_void) {
    // Accept mouse events
    let sel = sel_registerName(b"setIgnoresMouseEvents:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, 0);

    // Make visible
    let sel = sel_registerName(b"setAlphaValue:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, f64) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, 1.0);

    // Bring to front without activating
    let sel = sel_registerName(b"orderFrontRegardless\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel);
}

/// Hide the overlay (alpha-based, stays in window server for all Spaces).
pub unsafe fn hide_window(ns_window: *mut c_void) {
    let sel = sel_registerName(b"setAlphaValue:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, f64) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, 0.0);

    let sel = sel_registerName(b"setIgnoresMouseEvents:\0".as_ptr());
    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    send(ns_window, sel, 1);
}

// ── CGEvent: keyboard simulation ────────────────────────────────────

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceCreate(state_id: i32) -> *mut c_void;
    fn CGEventCreateKeyboardEvent(
        source: *mut c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut c_void;
    fn CGEventSetFlags(event: *mut c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut c_void);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *mut c_void);
}

/// Simulate Cmd+<key> via CGEvent.
unsafe fn simulate_cmd_key(virtual_key: u16) -> bool {
    const COMBINED_STATE: i32 = 0;
    const HID_EVENT_TAP: u32 = 0;
    const FLAG_CMD: u64 = 0x100000;

    let source = CGEventSourceCreate(COMBINED_STATE);
    if source.is_null() {
        return false;
    }

    let key_down = CGEventCreateKeyboardEvent(source, virtual_key, true);
    CGEventSetFlags(key_down, FLAG_CMD);
    CGEventPost(HID_EVENT_TAP, key_down);

    let key_up = CGEventCreateKeyboardEvent(source, virtual_key, false);
    CGEventSetFlags(key_up, FLAG_CMD);
    CGEventPost(HID_EVENT_TAP, key_up);

    CFRelease(key_down);
    CFRelease(key_up);
    CFRelease(source);

    true
}

/// Convert an NSString pointer to a Rust String.
pub unsafe fn nsstring_to_string(nsstr: *mut c_void) -> String {
    if nsstr.is_null() {
        return String::new();
    }

    let sel_utf8 = sel_registerName(b"UTF8String\0".as_ptr());
    let send_cstr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const i8 =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    let cstr_ptr = send_cstr(nsstr, sel_utf8);
    if cstr_ptr.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(cstr_ptr)
        .to_str()
        .unwrap_or("")
        .to_string()
}

/// Simulate Cmd+V (paste).
pub unsafe fn simulate_cmd_v() -> bool { simulate_cmd_key(9) }
/// Simulate Cmd+C (copy).
pub unsafe fn simulate_cmd_c() -> bool { simulate_cmd_key(8) }
/// Simulate Cmd+Z (undo).
pub unsafe fn simulate_cmd_z() -> bool { simulate_cmd_key(6) }
