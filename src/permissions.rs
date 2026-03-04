use serde::Serialize;

#[derive(Serialize)]
pub struct PermissionStatus {
    pub microphone: String,
    pub accessibility: bool,
}

#[cfg(target_os = "macos")]
mod inner {
    use std::ffi::{c_char, c_void};

    // AVFoundation — AVCaptureDevice authorizationStatusForMediaType:
    #[link(name = "AVFoundation", kind = "framework")]
    extern "C" {}

    // ApplicationServices — AXIsProcessTrusted / AXIsProcessTrustedWithOptions
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    }

    extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
    }

    /// Returns microphone authorization status:
    /// 0 = notDetermined, 1 = restricted, 2 = denied, 3 = authorized
    pub fn microphone_auth_status() -> i64 {
        unsafe {
            let cls = objc_getClass(c"AVCaptureDevice".as_ptr());
            if cls.is_null() {
                return 0;
            }
            let sel = sel_registerName(c"authorizationStatusForMediaType:".as_ptr());

            let ns_string_cls = objc_getClass(c"NSString".as_ptr());
            let sel_str = sel_registerName(c"stringWithUTF8String:".as_ptr());
            let make_str: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let media_type = make_str(ns_string_cls, sel_str, c"soun".as_ptr());

            let send: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i64 =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(cls, sel, media_type)
        }
    }

    /// Request microphone access (triggers the system prompt if undetermined).
    pub fn request_microphone_access() {
        unsafe {
            let cls = objc_getClass(c"AVCaptureDevice".as_ptr());
            if cls.is_null() {
                return;
            }
            let sel = sel_registerName(c"requestAccessForMediaType:completionHandler:".as_ptr());

            let ns_string_cls = objc_getClass(c"NSString".as_ptr());
            let sel_str = sel_registerName(c"stringWithUTF8String:".as_ptr());
            let make_str: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let media_type = make_str(ns_string_cls, sel_str, c"soun".as_ptr());

            #[repr(C)]
            struct Block {
                isa: *mut c_void,
                flags: i32,
                reserved: i32,
                invoke: unsafe extern "C" fn(*mut Block, bool),
                descriptor: *const BlockDescriptor,
            }
            #[repr(C)]
            struct BlockDescriptor {
                reserved: u64,
                size: u64,
            }

            unsafe extern "C" fn noop_invoke(_block: *mut Block, _granted: bool) {}

            extern "C" {
                #[link_name = "_NSConcreteStackBlock"]
                static NS_CONCRETE_STACK_BLOCK: *mut c_void;
            }

            static DESCRIPTOR: BlockDescriptor = BlockDescriptor {
                reserved: 0,
                size: std::mem::size_of::<Block>() as u64,
            };

            let mut block = Block {
                isa: &raw const NS_CONCRETE_STACK_BLOCK as *mut c_void,
                flags: 0,
                reserved: 0,
                invoke: noop_invoke,
                descriptor: &DESCRIPTOR,
            };

            let send: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(
                cls,
                sel,
                media_type,
                &mut block as *mut Block as *mut c_void,
            );
        }
    }

    pub fn accessibility_trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    /// Prompt the user to grant accessibility via the system dialog.
    /// Uses AXIsProcessTrustedWithOptions with kAXTrustedCheckOptionPrompt,
    /// which ensures macOS correctly identifies this process in TCC.
    pub fn prompt_accessibility() -> bool {
        unsafe {
            // Build { kAXTrustedCheckOptionPrompt: true } dictionary
            let key_str = objc_getClass(c"NSString".as_ptr());
            let sel_str = sel_registerName(c"stringWithUTF8String:".as_ptr());
            let make_str: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let key = make_str(
                key_str,
                sel_str,
                c"AXTrustedCheckOptionPrompt".as_ptr(),
            );

            // NSNumber numberWithBool:YES
            let ns_number = objc_getClass(c"NSNumber".as_ptr());
            let sel_bool = sel_registerName(c"numberWithBool:".as_ptr());
            let make_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, bool) -> *mut c_void =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let val = make_bool(ns_number, sel_bool, true);

            // NSDictionary dictionaryWithObject:forKey:
            let ns_dict = objc_getClass(c"NSDictionary".as_ptr());
            let sel_dict = sel_registerName(c"dictionaryWithObject:forKey:".as_ptr());
            let make_dict: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *mut c_void,
                *mut c_void,
            ) -> *mut c_void =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let opts = make_dict(ns_dict, sel_dict, val, key);

            AXIsProcessTrustedWithOptions(opts)
        }
    }
}

#[tauri::command]
pub fn check_permissions() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        let mic_status = inner::microphone_auth_status();
        let mic = match mic_status {
            3 => "granted",
            2 => "denied",
            1 => "denied",
            _ => "undetermined",
        };
        PermissionStatus {
            microphone: mic.to_string(),
            accessibility: inner::accessibility_trusted(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        PermissionStatus {
            microphone: "granted".to_string(),
            accessibility: true,
        }
    }
}

#[tauri::command]
pub fn open_permission_settings(permission_type: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let url = match permission_type.as_str() {
            "microphone" => {
                let status = inner::microphone_auth_status();
                if status == 0 {
                    inner::request_microphone_access();
                    return Ok(());
                }
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
            }
            "accessibility" => {
                // Use AXIsProcessTrustedWithOptions(prompt: true) so macOS
                // correctly registers THIS process in TCC, then also open
                // System Settings as a fallback/visual guide.
                inner::prompt_accessibility();
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
            }
            _ => return Err(format!("Unknown permission type: {}", permission_type)),
        };
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open system settings: {}", e))?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = permission_type;
        Ok(())
    }
}
