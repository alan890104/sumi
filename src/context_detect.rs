use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppContext {
    pub app_name: String,
    pub bundle_id: String,
    pub url: String,
}

/// Detect the frontmost macOS application and, if it's a known browser, the current URL.
#[cfg(target_os = "macos")]
pub fn detect_frontmost_app() -> AppContext {
    let (app_name, bundle_id) = get_frontmost_app_info();
    let url = get_browser_url(&bundle_id);
    AppContext {
        app_name,
        bundle_id,
        url,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn detect_frontmost_app() -> AppContext {
    AppContext::default()
}

/// Uses Objective-C runtime to get the frontmost application's name and bundle ID.
#[cfg(target_os = "macos")]
fn get_frontmost_app_info() -> (String, String) {
    use std::ffi::c_void;

    extern "C" {
        fn sel_registerName(name: *const u8) -> *mut c_void;
        fn objc_getClass(name: *const u8) -> *mut c_void;
        fn objc_msgSend();
    }

    unsafe {
        // [NSWorkspace sharedWorkspace]
        let cls = objc_getClass(c"NSWorkspace".as_ptr().cast());
        if cls.is_null() {
            return (String::new(), String::new());
        }

        let sel_shared = sel_registerName(c"sharedWorkspace".as_ptr().cast());
        let send_void: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let workspace = send_void(cls, sel_shared);
        if workspace.is_null() {
            return (String::new(), String::new());
        }

        // [workspace frontmostApplication]
        let sel_front = sel_registerName(c"frontmostApplication".as_ptr().cast());
        let app = send_void(workspace, sel_front);
        if app.is_null() {
            return (String::new(), String::new());
        }

        // [app localizedName]
        let sel_name = sel_registerName(c"localizedName".as_ptr().cast());
        let ns_name = send_void(app, sel_name);
        let app_name = nsstring_to_string(ns_name);

        // [app bundleIdentifier]
        let sel_bundle = sel_registerName(c"bundleIdentifier".as_ptr().cast());
        let ns_bundle = send_void(app, sel_bundle);
        let bundle_id = nsstring_to_string(ns_bundle);

        (app_name, bundle_id)
    }
}

/// Convert an NSString pointer to a Rust String.
#[cfg(target_os = "macos")]
unsafe fn nsstring_to_string(nsstr: *mut std::ffi::c_void) -> String {
    use std::ffi::{c_void, CStr};

    if nsstr.is_null() {
        return String::new();
    }

    extern "C" {
        fn sel_registerName(name: *const u8) -> *mut c_void;
        fn objc_msgSend();
    }

    let sel_utf8 = sel_registerName(c"UTF8String".as_ptr().cast());
    let send_cstr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const i8 =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    let cstr_ptr = send_cstr(nsstr, sel_utf8);
    if cstr_ptr.is_null() {
        return String::new();
    }
    CStr::from_ptr(cstr_ptr)
        .to_str()
        .unwrap_or("")
        .to_string()
}

/// For known browsers, run an AppleScript to get the current URL.
/// Returns empty string for non-browser apps or on failure.
#[cfg(target_os = "macos")]
fn get_browser_url(bundle_id: &str) -> String {
    let script = match bundle_id {
        "com.apple.Safari" => {
            r#"tell application "Safari" to get URL of front document"#
        }
        "com.google.Chrome" => {
            r#"tell application "Google Chrome" to get URL of active tab of front window"#
        }
        "company.thebrowser.Browser" => {
            r#"tell application "Arc" to get URL of active tab of front window"#
        }
        "com.brave.Browser" => {
            r#"tell application "Brave Browser" to get URL of active tab of front window"#
        }
        "com.microsoft.edgemac" => {
            r#"tell application "Microsoft Edge" to get URL of active tab of front window"#
        }
        _ => return String::new(),
    };

    std::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

#[cfg(not(target_os = "macos"))]
fn get_browser_url(_bundle_id: &str) -> String {
    String::new()
}
