mod context_detect;
mod history;
mod polisher;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
    Arc, Mutex,
};
use std::io::Read as _;
use std::time::Instant;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use whisper_rs::{WhisperContext, WhisperContextParameters};

const MAX_RECORDING_SECS: u64 = 30;

// ── macOS: non-activating window helpers ────────────────────────────────────
//
// On macOS, `NSWindow.makeKeyAndOrderFront:` (used by Tauri's `window.show()`)
// activates the application — stealing focus from whatever the user was typing in.
// For a menu-bar overlay that must never grab focus we call the Objective-C runtime
// directly:  `orderFrontRegardless` to show, `orderOut:` to hide.

#[cfg(target_os = "macos")]
mod macos_ffi {
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
        // [NSApplication sharedApplication]
        // Use CFString for the class name
        let cls_name = b"NSApplication\0";
        let sel_shared = sel_registerName(b"sharedApplication\0".as_ptr());
        let send_shared: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());

        // Get NSApplication class
        let class_sel = sel_registerName(b"class\0".as_ptr());
        let _ = class_sel; // unused, we use objc_getClass instead

        // objc_getClass("NSApplication")
        let ns_app_class = objc_getClass(cls_name.as_ptr());
        if ns_app_class.is_null() {
            return;
        }
        let ns_app = send_shared(ns_app_class, sel_shared);
        if ns_app.is_null() {
            return;
        }

        // [NSApp setActivationPolicy: NSApplicationActivationPolicyAccessory]
        let sel_policy = sel_registerName(b"setActivationPolicy:\0".as_ptr());
        let send_policy: unsafe extern "C" fn(*mut c_void, *mut c_void, i64) =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        send_policy(ns_app, sel_policy, 1); // 1 = Accessory
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
        let panel_class_name = b"OTLOverlayPanel\0".as_ptr();
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
        // ── Convert NSWindow → NSPanel so it can appear in fullscreen Spaces ──
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

    // ── CGEvent: Cmd+V paste simulation ────────────────────────────────────

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

    /// Simulate Cmd+V via CGEvent.
    ///
    /// Only two events are posted: V key-down and V key-up, both carrying the
    /// Command modifier flag.  Explicit Cmd key-down / key-up events are NOT
    /// sent — that avoids extra events flowing through the TSM (input method)
    /// and global-shortcut event tap chains, which previously caused
    /// double-paste on systems with a CJK input method active.
    pub unsafe fn simulate_cmd_v() -> bool {
        const COMBINED_STATE: i32 = 0; // kCGEventSourceStateCombinedSessionState
        const HID_EVENT_TAP: u32 = 0; // kCGHIDEventTap
        const FLAG_CMD: u64 = 0x100000; // kCGEventFlagMaskCommand
        const VK_V: u16 = 9;

        let source = CGEventSourceCreate(COMBINED_STATE);
        if source.is_null() {
            return false;
        }

        // V down with Cmd flag
        let v_d = CGEventCreateKeyboardEvent(source, VK_V, true);
        CGEventSetFlags(v_d, FLAG_CMD);
        CGEventPost(HID_EVENT_TAP, v_d);

        // V up with Cmd flag
        let v_u = CGEventCreateKeyboardEvent(source, VK_V, false);
        CGEventSetFlags(v_u, FLAG_CMD);
        CGEventPost(HID_EVENT_TAP, v_u);

        CFRelease(v_d);
        CFRelease(v_u);
        CFRelease(source);

        true
    }
}

// ── Keychain (macOS) ─────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod keychain {
    use std::process::Command;

    const ACCOUNT: &str = "opentypeless";

    fn service_name(provider: &str) -> String {
        format!("opentypeless-api-key-{}", provider)
    }

    pub fn save(provider: &str, key: &str) -> Result<(), String> {
        let service = service_name(provider);
        // -U updates if exists, creates if not
        let output = Command::new("security")
            .args([
                "add-generic-password",
                "-a", ACCOUNT,
                "-s", &service,
                "-w", key,
                "-U",
            ])
            .output()
            .map_err(|e| format!("Failed to run security command: {}", e))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Keychain save failed: {}", stderr.trim()))
        }
    }

    pub fn load(provider: &str) -> Result<String, String> {
        let service = service_name(provider);
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-a", ACCOUNT,
                "-s", &service,
                "-w",
            ])
            .output()
            .map_err(|e| format!("Failed to run security command: {}", e))?;
        if output.status.success() {
            let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(key)
        } else {
            // Not found is not an error — return empty string
            Ok(String::new())
        }
    }

    pub fn delete(provider: &str) -> Result<(), String> {
        let service = service_name(provider);
        let output = Command::new("security")
            .args([
                "delete-generic-password",
                "-a", ACCOUNT,
                "-s", &service,
            ])
            .output()
            .map_err(|e| format!("Failed to run security command: {}", e))?;
        if output.status.success() {
            Ok(())
        } else {
            // Not found is fine
            Ok(())
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod keychain {
    pub fn save(_provider: &str, _key: &str) -> Result<(), String> {
        Err("Keychain is only supported on macOS".to_string())
    }

    pub fn load(_provider: &str) -> Result<String, String> {
        Ok(String::new())
    }

    pub fn delete(_provider: &str) -> Result<(), String> {
        Err("Keychain is only supported on macOS".to_string())
    }
}

// ── Settings ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hotkey: String,
    pub auto_paste: bool,
    #[serde(default)]
    pub polish: polisher::PolishConfig,
    /// 0 = keep forever, otherwise number of days to retain history entries.
    #[serde(default)]
    pub history_retention_days: u32,
    /// UI language override. None = auto-detect from system.
    #[serde(default)]
    pub language: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Alt+KeyZ".to_string(),
            auto_paste: true,
            polish: polisher::PolishConfig::default(),
            history_retention_days: 0, // forever
            language: None,
        }
    }
}

// ── Consolidated data directory: ~/.opentypeless ─────────────────────────────

fn base_dir() -> PathBuf {
    dirs::home_dir()
        .expect("no home dir")
        .join(".opentypeless")
}
fn config_dir() -> PathBuf {
    base_dir().join("config")
}
fn models_dir() -> PathBuf {
    base_dir().join("models")
}
fn history_dir() -> PathBuf {
    base_dir().join("history")
}
fn audio_dir() -> PathBuf {
    base_dir().join("audio")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

fn load_settings() -> Settings {
    let path = settings_path();
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    } else {
        Settings::default()
    }
}

fn save_settings_to_disk(settings: &Settings) {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(&path, json);
    }
}

// ── Hotkey parsing ──────────────────────────────────────────────────────────

fn parse_key_code(s: &str) -> Option<Code> {
    match s {
        "KeyA" => Some(Code::KeyA),
        "KeyB" => Some(Code::KeyB),
        "KeyC" => Some(Code::KeyC),
        "KeyD" => Some(Code::KeyD),
        "KeyE" => Some(Code::KeyE),
        "KeyF" => Some(Code::KeyF),
        "KeyG" => Some(Code::KeyG),
        "KeyH" => Some(Code::KeyH),
        "KeyI" => Some(Code::KeyI),
        "KeyJ" => Some(Code::KeyJ),
        "KeyK" => Some(Code::KeyK),
        "KeyL" => Some(Code::KeyL),
        "KeyM" => Some(Code::KeyM),
        "KeyN" => Some(Code::KeyN),
        "KeyO" => Some(Code::KeyO),
        "KeyP" => Some(Code::KeyP),
        "KeyQ" => Some(Code::KeyQ),
        "KeyR" => Some(Code::KeyR),
        "KeyS" => Some(Code::KeyS),
        "KeyT" => Some(Code::KeyT),
        "KeyU" => Some(Code::KeyU),
        "KeyV" => Some(Code::KeyV),
        "KeyW" => Some(Code::KeyW),
        "KeyX" => Some(Code::KeyX),
        "KeyY" => Some(Code::KeyY),
        "KeyZ" => Some(Code::KeyZ),
        "Digit0" => Some(Code::Digit0),
        "Digit1" => Some(Code::Digit1),
        "Digit2" => Some(Code::Digit2),
        "Digit3" => Some(Code::Digit3),
        "Digit4" => Some(Code::Digit4),
        "Digit5" => Some(Code::Digit5),
        "Digit6" => Some(Code::Digit6),
        "Digit7" => Some(Code::Digit7),
        "Digit8" => Some(Code::Digit8),
        "Digit9" => Some(Code::Digit9),
        "F1" => Some(Code::F1),
        "F2" => Some(Code::F2),
        "F3" => Some(Code::F3),
        "F4" => Some(Code::F4),
        "F5" => Some(Code::F5),
        "F6" => Some(Code::F6),
        "F7" => Some(Code::F7),
        "F8" => Some(Code::F8),
        "F9" => Some(Code::F9),
        "F10" => Some(Code::F10),
        "F11" => Some(Code::F11),
        "F12" => Some(Code::F12),
        "Space" => Some(Code::Space),
        "Enter" => Some(Code::Enter),
        "Tab" => Some(Code::Tab),
        "Backspace" => Some(Code::Backspace),
        "Delete" => Some(Code::Delete),
        "Escape" => Some(Code::Escape),
        "ArrowUp" => Some(Code::ArrowUp),
        "ArrowDown" => Some(Code::ArrowDown),
        "ArrowLeft" => Some(Code::ArrowLeft),
        "ArrowRight" => Some(Code::ArrowRight),
        "Home" => Some(Code::Home),
        "End" => Some(Code::End),
        "PageUp" => Some(Code::PageUp),
        "PageDown" => Some(Code::PageDown),
        "Minus" => Some(Code::Minus),
        "Equal" => Some(Code::Equal),
        "BracketLeft" => Some(Code::BracketLeft),
        "BracketRight" => Some(Code::BracketRight),
        "Backslash" => Some(Code::Backslash),
        "Semicolon" => Some(Code::Semicolon),
        "Quote" => Some(Code::Quote),
        "Comma" => Some(Code::Comma),
        "Period" => Some(Code::Period),
        "Slash" => Some(Code::Slash),
        "Backquote" => Some(Code::Backquote),
        _ => None,
    }
}

fn parse_hotkey_string(s: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = s.split('+').collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in &parts {
        match *part {
            "Alt" => modifiers |= Modifiers::ALT,
            "Control" => modifiers |= Modifiers::CONTROL,
            "Shift" => modifiers |= Modifiers::SHIFT,
            "Super" => modifiers |= Modifiers::SUPER,
            other => {
                key_code = parse_key_code(other);
            }
        }
    }

    let code = key_code?;
    let mods = if modifiers.is_empty() {
        None
    } else {
        Some(modifiers)
    };
    Some(Shortcut::new(mods, code))
}

fn hotkey_display_label(s: &str) -> String {
    let parts: Vec<&str> = s.split('+').collect();
    let mut labels = Vec::new();
    for part in &parts {
        let label = match *part {
            "Alt" => "⌥",
            "Control" => "⌃",
            "Shift" => "⇧",
            "Super" => "⌘",
            other => {
                // Strip "Key" prefix for letters, "Digit" for numbers
                if let Some(letter) = other.strip_prefix("Key") {
                    labels.push(letter.to_string());
                    continue;
                }
                if let Some(digit) = other.strip_prefix("Digit") {
                    labels.push(digit.to_string());
                    continue;
                }
                labels.push(other.to_string());
                continue;
            }
        };
        labels.push(label.to_string());
    }
    labels.join(" ")
}

// ── App State ───────────────────────────────────────────────────────────────

pub struct AppState {
    is_recording: Arc<AtomicBool>,
    /// True while the transcription/polish/paste pipeline is running.
    /// Prevents the hotkey from accidentally starting a new recording
    /// if the user double-presses or the OS sends a key-repeat event.
    is_processing: AtomicBool,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: Mutex<Option<u32>>,
    settings: Mutex<Settings>,
    mic_available: AtomicBool,
    whisper_ctx: Mutex<Option<WhisperContext>>,
    llm_model: Mutex<Option<polisher::LlmModelCache>>,
    captured_context: Mutex<Option<context_detect::AppContext>>,
    /// Optional override for frontmost-app context (used by test page step 3).
    context_override: Mutex<Option<context_detect::AppContext>>,
    /// When true, the global hotkey only emits `hotkey-activated` without recording.
    test_mode: AtomicBool,
    /// Debounce: timestamp of the last processed hotkey event.
    /// Prevents macOS key-repeat from toggling recording on/off too quickly.
    last_hotkey_time: Mutex<Instant>,
}

/// Spawn a persistent audio thread that builds and immediately starts the cpal
/// input stream.  The stream runs for the entire app lifetime — the callback
/// checks `is_recording` atomically and discards samples when false.
///
/// This gives true zero-latency recording: flipping `is_recording` to `true`
/// causes the very next callback invocation to start writing samples.
fn spawn_audio_thread(
    buffer: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
) -> Result<u32, String> {
    let (init_tx, init_rx) = mpsc::channel::<Result<u32, String>>();

    let buf_for_thread = Arc::clone(&buffer);
    let rec_for_thread = Arc::clone(&is_recording);

    std::thread::spawn(move || {
        let host = cpal::default_host();

        let device = match host.default_input_device() {
            Some(d) => d,
            None => {
                let _ = init_tx.send(Err("找不到麥克風裝置".to_string()));
                return;
            }
        };

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                let _ = init_tx.send(Err(format!("無法取得輸入設定: {}", e)));
                return;
            }
        };

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        // Build the stream callback — guarded by `is_recording` so no samples
        // are written while the user is not recording.
        let stream = {
            let buf = Arc::clone(&buf_for_thread);
            let rec = Arc::clone(&rec_for_thread);
            match config.sample_format() {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if !rec.load(Ordering::Relaxed) {
                            return;
                        }
                        let mut buf = buf.lock().unwrap();
                        if channels == 1 {
                            buf.extend_from_slice(data);
                        } else {
                            for chunk in data.chunks(channels) {
                                buf.push(chunk.iter().sum::<f32>() / channels as f32);
                            }
                        }
                    },
                    |err| eprintln!("[OpenTypeless] audio stream error: {}", err),
                    None,
                ),
                cpal::SampleFormat::I16 => {
                    let buf = Arc::clone(&buf_for_thread);
                    let rec = Arc::clone(&rec_for_thread);
                    device.build_input_stream(
                        &config.into(),
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            if !rec.load(Ordering::Relaxed) {
                                return;
                            }
                            let mut buf = buf.lock().unwrap();
                            if channels == 1 {
                                buf.extend(data.iter().map(|&s| s as f32 / i16::MAX as f32));
                            } else {
                                for chunk in data.chunks(channels) {
                                    buf.push(
                                        chunk
                                            .iter()
                                            .map(|&s| s as f32 / i16::MAX as f32)
                                            .sum::<f32>()
                                            / channels as f32,
                                    );
                                }
                            }
                        },
                        |err| eprintln!("[OpenTypeless] audio stream error: {}", err),
                        None,
                    )
                }
                other => {
                    let _ = init_tx.send(Err(format!("不支援的音訊格式: {:?}", other)));
                    return;
                }
            }
        };

        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                let _ = init_tx.send(Err(format!("無法建立錄音串流: {}", e)));
                return;
            }
        };

        // Start the stream immediately — it runs for the entire app lifetime.
        // The callback discards samples while is_recording is false, so there
        // is negligible CPU overhead when idle.
        if let Err(e) = stream.play() {
            let _ = init_tx.send(Err(format!("無法啟動錄音串流: {}", e)));
            return;
        }

        println!(
            "[OpenTypeless] Audio stream always-on: {} Hz, {} ch",
            sample_rate, channels
        );
        let _ = init_tx.send(Ok(sample_rate));

        // Park the thread forever to keep `stream` alive.
        // The stream callback continues running on CoreAudio's own thread.
        loop {
            std::thread::park();
        }
    });

    let sample_rate = init_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "音訊執行緒初始化逾時".to_string())??;

    Ok(sample_rate)
}

/// Attempt to reconnect the microphone when `mic_available` is false.
/// On success, updates `sample_rate` and `mic_available` in AppState.
fn try_reconnect_audio(state: &AppState) -> Result<(), String> {
    if state.mic_available.load(Ordering::SeqCst) {
        return Ok(());
    }
    let sr = spawn_audio_thread(Arc::clone(&state.buffer), Arc::clone(&state.is_recording))?;
    *state.sample_rate.lock().map_err(|e| e.to_string())? = Some(sr);
    state.mic_available.store(true, Ordering::SeqCst);
    println!("[OpenTypeless] Microphone reconnected: {} Hz", sr);
    Ok(())
}

// ── Tauri Commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(
    state: State<'_, AppState>,
    new_settings: Settings,
) -> Result<(), String> {
    let mut current = state.settings.lock().map_err(|e| e.to_string())?;
    current.auto_paste = new_settings.auto_paste;
    current.polish = new_settings.polish;
    current.history_retention_days = new_settings.history_retention_days;
    save_settings_to_disk(&current);
    Ok(())
}

#[tauri::command]
fn update_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    new_hotkey: String,
) -> Result<(), String> {
    let shortcut =
        parse_hotkey_string(&new_hotkey).ok_or_else(|| "Invalid hotkey string".to_string())?;

    // Unregister all existing shortcuts
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    // Register the new shortcut
    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| format!("Failed to register shortcut: {}", e))?;

    // Update state and persist
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.hotkey = new_hotkey.clone();
    save_settings_to_disk(&settings);

    // Update tray tooltip
    let label = hotkey_display_label(&new_hotkey);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(&format!("OpenTypeless – {} to record", label)));
    }

    println!(
        "[OpenTypeless] Hotkey updated to: {} ({})",
        new_hotkey, label
    );
    Ok(())
}

#[tauri::command]
fn reset_settings(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let defaults = Settings::default();
    let default_hotkey = defaults.hotkey.clone();

    // Replace in-memory settings
    {
        let mut current = state.settings.lock().map_err(|e| e.to_string())?;
        *current = defaults;
    }

    // Persist defaults to disk
    save_settings_to_disk(&Settings::default());

    // Re-register the default hotkey
    let shortcut = parse_hotkey_string(&default_hotkey)
        .ok_or_else(|| "Invalid default hotkey string".to_string())?;

    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| format!("Failed to register shortcut: {}", e))?;

    // Update tray tooltip
    let label = hotkey_display_label(&default_hotkey);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(&format!("OpenTypeless – {} to record", label)));
    }

    println!("[OpenTypeless] Settings reset to defaults (hotkey: {})", label);
    Ok(())
}

#[tauri::command]
fn get_default_prompt(language: Option<String>) -> String {
    let lang: polisher::OutputLanguage = language
        .and_then(|s| serde_json::from_value(serde_json::Value::String(s)).ok())
        .unwrap_or_default();
    polisher::base_prompt_template(&lang)
}

#[tauri::command]
fn get_default_prompt_rules(language: Option<String>) -> Vec<polisher::PromptRule> {
    let lang: polisher::OutputLanguage = language
        .and_then(|s| serde_json::from_value(serde_json::Value::String(s)).ok())
        .unwrap_or_default();
    polisher::default_prompt_rules_for(&lang)
}

#[tauri::command]
fn save_api_key(provider: String, key: String) -> Result<(), String> {
    if key.is_empty() {
        keychain::delete(&provider)?;
    } else {
        keychain::save(&provider, &key)?;
    }
    Ok(())
}

#[tauri::command]
fn get_api_key(provider: String) -> Result<String, String> {
    keychain::load(&provider)
}

#[tauri::command]
fn get_history() -> Vec<history::HistoryEntry> {
    history::load_history(&history_dir())
}

#[tauri::command]
fn delete_history_entry(id: String) -> Result<(), String> {
    history::delete_entry(&history_dir(), &audio_dir(), &id);
    Ok(())
}

#[tauri::command]
fn export_history_audio(id: String) -> Result<String, String> {
    let dest = history::export_audio(&audio_dir(), &id)?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
fn clear_all_history() -> Result<(), String> {
    history::clear_all(&history_dir(), &audio_dir());
    Ok(())
}

#[tauri::command]
fn get_history_storage_path() -> String {
    base_dir().to_string_lossy().to_string()
}

#[derive(Serialize)]
struct TestPolishResult {
    current_result: String,
    edited_result: String,
}

#[tauri::command]
fn test_polish(
    state: State<'_, AppState>,
    test_text: String,
    custom_prompt: String,
) -> Result<TestPolishResult, String> {
    let config = state.settings.lock().map_err(|e| e.to_string())?.polish.clone();
    let model_dir = models_dir();

    // Default built-in prompt
    let default_tmpl = polisher::base_prompt_template(&config.output_language);
    let default_system_prompt =
        polisher::resolve_prompt(&default_tmpl, &config.output_language);

    // User's custom prompt (from textarea)
    let custom_system_prompt =
        polisher::resolve_prompt(&custom_prompt, &config.output_language);

    let default_result = polisher::polish_with_prompt(
        &state.llm_model,
        &model_dir,
        &config,
        &default_system_prompt,
        &test_text,
    )?;

    let custom_result = polisher::polish_with_prompt(
        &state.llm_model,
        &model_dir,
        &config,
        &custom_system_prompt,
        &test_text,
    )?;

    Ok(TestPolishResult {
        current_result: default_result,
        edited_result: custom_result,
    })
}

// Keep Tauri commands for potential future use from frontend
#[tauri::command]
fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    do_start_recording(&state)
}

#[tauri::command]
fn stop_recording(state: State<'_, AppState>) -> Result<String, String> {
    do_stop_recording(&state).map(|(text, _samples)| text)
}

#[tauri::command]
fn set_test_mode(state: State<'_, AppState>, enabled: bool) {
    state.test_mode.store(enabled, Ordering::SeqCst);
}

#[tauri::command]
fn set_context_override(
    state: State<'_, AppState>,
    app_name: String,
    bundle_id: String,
    url: String,
) -> Result<(), String> {
    if let Ok(mut ctx) = state.context_override.lock() {
        if app_name.is_empty() && bundle_id.is_empty() && url.is_empty() {
            *ctx = None;
        } else {
            *ctx = Some(context_detect::AppContext { app_name, bundle_id, url });
        }
    }
    Ok(())
}

#[tauri::command]
fn cancel_recording(app: AppHandle, state: State<'_, AppState>) {
    state.is_recording.store(false, Ordering::SeqCst);
    if let Some(overlay) = app.get_webview_window("overlay") {
        #[cfg(target_os = "macos")]
        if let Ok(ns_win) = overlay.ns_window() {
            unsafe { macos_ffi::hide_window(ns_win); }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = overlay.hide();
    }
}

#[derive(Serialize)]
struct MicStatus {
    connected: bool,
    default_device: Option<String>,
    devices: Vec<String>,
}

#[tauri::command]
fn get_mic_status(state: State<'_, AppState>) -> MicStatus {
    let host = cpal::default_host();
    let default_device = host.default_input_device().and_then(|d| d.name().ok());
    let devices: Vec<String> = host
        .input_devices()
        .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default();
    MicStatus {
        connected: state.mic_available.load(Ordering::SeqCst),
        default_device,
        devices,
    }
}

// ── Model download ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ModelStatus {
    engine: String,
    model_exists: bool,
}

#[tauri::command]
fn check_model_status() -> ModelStatus {
    let model_exists = models_dir()
        .join("ggml-large-v3-turbo-zh-TW.bin")
        .exists();
    ModelStatus {
        engine: "whisper".to_string(),
        model_exists,
    }
}

#[tauri::command]
fn download_model(app: AppHandle) -> Result<(), String> {
    let dir = models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let model_path = dir.join("ggml-large-v3-turbo-zh-TW.bin");
    if model_path.exists() {
        let _ = app.emit("model-download-progress", serde_json::json!({
            "status": "complete",
            "downloaded": 0u64,
            "total": 0u64,
            "percent": 100.0
        }));
        return Ok(());
    }

    let tmp_path = model_path.with_extension("bin.part");
    let _ = std::fs::remove_file(&tmp_path);

    std::thread::spawn(move || {
        let url = "https://huggingface.co/Alkd/whisper-large-v3-turbo-zh-TW/resolve/main/ggml-model.bin";
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to create HTTP client: {}", e)
                }));
                return;
            }
        };

        let resp = match client.get(url).send() {
            Ok(r) => r,
            Err(e) => {
                let _ = app.emit("model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Download request failed: {}", e)
                }));
                return;
            }
        };

        if !resp.status().is_success() {
            let _ = app.emit("model-download-progress", serde_json::json!({
                "status": "error",
                "message": format!("Download returned HTTP {}", resp.status())
            }));
            return;
        }

        let total = resp.content_length().unwrap_or(0);

        let mut file = match std::fs::File::create(&tmp_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = app.emit("model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to create temp file: {}", e)
                }));
                return;
            }
        };

        let mut downloaded: u64 = 0;
        let mut buf = [0u8; 65536]; // 64 KB
        let mut last_emit = Instant::now();
        let mut reader = resp;

        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = app.emit("model-download-progress", serde_json::json!({
                        "status": "error",
                        "message": format!("Download read error: {}", e)
                    }));
                    return;
                }
            };

            if let Err(e) = std::io::Write::write_all(&mut file, &buf[..n]) {
                let _ = app.emit("model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to write to disk: {}", e)
                }));
                return;
            }

            downloaded += n as u64;

            // Throttle events to ~10 Hz
            if last_emit.elapsed() >= std::time::Duration::from_millis(100) {
                let percent = if total > 0 {
                    (downloaded as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                let _ = app.emit("model-download-progress", serde_json::json!({
                    "status": "downloading",
                    "downloaded": downloaded,
                    "total": total,
                    "percent": percent
                }));
                last_emit = Instant::now();
            }
        }

        // Flush and rename
        drop(file);
        if let Err(e) = std::fs::rename(&tmp_path, &model_path) {
            let _ = app.emit("model-download-progress", serde_json::json!({
                "status": "error",
                "message": format!("Failed to rename temp file: {}", e)
            }));
            return;
        }

        // Invalidate cached WhisperContext so next transcription loads the new model
        if let Some(app_state) = app.try_state::<AppState>() {
            if let Ok(mut ctx) = app_state.whisper_ctx.lock() {
                *ctx = None;
                println!("[OpenTypeless] Whisper context cache invalidated after model download");
            }
        }

        let _ = app.emit("model-download-progress", serde_json::json!({
            "status": "complete",
            "downloaded": downloaded,
            "total": total,
            "percent": 100.0
        }));
        println!("[OpenTypeless] Whisper model downloaded: {:?}", model_path);
    });

    Ok(())
}

// ── LLM Model management ────────────────────────────────────────────────────

#[derive(Serialize)]
struct LlmModelStatus {
    model: String,
    model_exists: bool,
    model_size_bytes: u64,
}

#[tauri::command]
fn check_llm_model_status(state: State<'_, AppState>) -> LlmModelStatus {
    let settings = state.settings.lock().unwrap();
    let model = &settings.polish.model;
    let dir = models_dir();
    LlmModelStatus {
        model: model.display_name().to_string(),
        model_exists: polisher::model_file_exists(&dir, model),
        model_size_bytes: polisher::model_file_size(&dir, model),
    }
}

#[tauri::command]
fn download_llm_model(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let dir = models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    let model = settings.polish.model.clone();
    drop(settings);

    let model_path = dir.join(model.filename());
    if model_path.exists() {
        let _ = app.emit("llm-model-download-progress", serde_json::json!({
            "status": "complete",
            "downloaded": 0u64,
            "total": 0u64,
            "percent": 100.0
        }));
        return Ok(());
    }

    let tmp_path = model_path.with_extension("gguf.part");
    let _ = std::fs::remove_file(&tmp_path);

    let url = model.download_url().to_string();

    std::thread::spawn(move || {
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(1800))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("llm-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to create HTTP client: {}", e)
                }));
                return;
            }
        };

        let resp = match client.get(&url).send() {
            Ok(r) => r,
            Err(e) => {
                let _ = app.emit("llm-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Download request failed: {}", e)
                }));
                return;
            }
        };

        if !resp.status().is_success() {
            let _ = app.emit("llm-model-download-progress", serde_json::json!({
                "status": "error",
                "message": format!("Download returned HTTP {}", resp.status())
            }));
            return;
        }

        let total = resp.content_length().unwrap_or(0);

        let mut file = match std::fs::File::create(&tmp_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = app.emit("llm-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to create temp file: {}", e)
                }));
                return;
            }
        };

        let mut downloaded: u64 = 0;
        let mut buf = [0u8; 65536]; // 64 KB
        let mut last_emit = Instant::now();
        let mut reader = resp;

        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = app.emit("llm-model-download-progress", serde_json::json!({
                        "status": "error",
                        "message": format!("Download read error: {}", e)
                    }));
                    return;
                }
            };

            if let Err(e) = std::io::Write::write_all(&mut file, &buf[..n]) {
                let _ = app.emit("llm-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to write to disk: {}", e)
                }));
                return;
            }

            downloaded += n as u64;

            // Throttle events to ~10 Hz
            if last_emit.elapsed() >= std::time::Duration::from_millis(100) {
                let percent = if total > 0 {
                    (downloaded as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                let _ = app.emit("llm-model-download-progress", serde_json::json!({
                    "status": "downloading",
                    "downloaded": downloaded,
                    "total": total,
                    "percent": percent
                }));
                last_emit = Instant::now();
            }
        }

        // Flush and rename
        drop(file);
        if let Err(e) = std::fs::rename(&tmp_path, &model_path) {
            let _ = app.emit("llm-model-download-progress", serde_json::json!({
                "status": "error",
                "message": format!("Failed to rename temp file: {}", e)
            }));
            return;
        }

        // Invalidate cached LLM so next polish loads the new model
        if let Some(app_state) = app.try_state::<AppState>() {
            polisher::invalidate_cache(&app_state.llm_model);
        }

        let _ = app.emit("llm-model-download-progress", serde_json::json!({
            "status": "complete",
            "downloaded": downloaded,
            "total": total,
            "percent": 100.0
        }));
        println!("[OpenTypeless] LLM model downloaded: {:?}", model_path);
    });

    Ok(())
}

// ── Recording ───────────────────────────────────────────────────────────────

/// Start recording — truly instant because the audio stream is always running.
///
/// All we do is clear the buffer and flip the flag.  The very next audio
/// callback invocation (typically <5 ms away) will start writing samples.
fn do_start_recording(state: &AppState) -> Result<(), String> {
    if !state.mic_available.load(Ordering::SeqCst) {
        try_reconnect_audio(state)?;
    }

    if state.is_recording.load(Ordering::SeqCst) {
        return Err("已在錄音中".to_string());
    }

    // Clear the buffer BEFORE enabling the flag, so the callback doesn't
    // write into a stale buffer.
    {
        let mut buf = state.buffer.lock().map_err(|e| e.to_string())?;
        buf.clear();
    }

    // Enable writing in the audio callback — the always-on stream will
    // start storing samples on its very next callback invocation.
    state.is_recording.store(true, Ordering::SeqCst);

    Ok(())
}

/// Resolve the path to the whisper GGML model.
/// Returns an error if the model hasn't been downloaded yet.
fn whisper_model_path() -> Result<PathBuf, String> {
    let model_path = models_dir().join("ggml-large-v3-turbo-zh-TW.bin");
    if model_path.exists() {
        Ok(model_path)
    } else {
        Err("Whisper model not downloaded. Please download it from Settings.".to_string())
    }
}

/// Transcribe 16 kHz mono f32 samples using the cached WhisperContext.
/// The context is lazily loaded on first use and reused across transcriptions.
fn transcribe_with_cached_whisper(
    state: &AppState,
    samples_16k: &[f32],
) -> Result<String, String> {
    use whisper_rs::{FullParams, SamplingStrategy};

    // Suppress verbose C-level logs from whisper.cpp / ggml
    unsafe extern "C" fn noop_log(
        _level: u32,
        _text: *const std::ffi::c_char,
        _user_data: *mut std::ffi::c_void,
    ) {
    }
    unsafe {
        whisper_rs::set_log_callback(Some(noop_log), std::ptr::null_mut());
    }

    let mut ctx_guard = state
        .whisper_ctx
        .lock()
        .map_err(|e| format!("Failed to lock whisper context: {}", e))?;

    if ctx_guard.is_none() {
        let model_path = whisper_model_path()?;
        let load_start = Instant::now();
        println!("[OpenTypeless] Loading Whisper model (first use)...");
        let mut ctx_params = WhisperContextParameters::new();
        ctx_params.use_gpu(true);
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or("Invalid model path")?,
            ctx_params,
        )
        .map_err(|e| format!("Failed to load whisper model: {}", e))?;
        *ctx_guard = Some(ctx);
        println!("[OpenTypeless] Whisper model loaded with GPU enabled (took {:.0?})", load_start.elapsed());
    }

    let ctx = ctx_guard.as_ref().unwrap();

    let state_start = Instant::now();
    let mut wh_state = ctx
        .create_state()
        .map_err(|e| format!("Failed to create whisper state: {}", e))?;
    println!("[OpenTypeless] Whisper state created: {:.0?}", state_start.elapsed());

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(None); // auto-detect language
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_print_progress(false);
    params.set_single_segment(true);
    params.set_no_timestamps(true);       // skip timestamp tokens → faster decode
    params.set_no_context(true);          // don't use previous context
    params.set_temperature_inc(-1.0);     // disable fallback decoding passes
    params.set_n_threads(num_cpus() as _); // use all performance cores

    let infer_start = Instant::now();
    wh_state
        .full(params, samples_16k)
        .map_err(|e| format!("Whisper inference failed: {}", e))?;
    println!("[OpenTypeless] Whisper wh_state.full() done: {:.0?}", infer_start.elapsed());

    let num_segments = wh_state.full_n_segments();

    let mut text = String::new();
    for i in 0..num_segments {
        if let Some(seg) = wh_state.get_segment(i) {
            if let Ok(s) = seg.to_str_lossy() {
                text.push_str(&s);
            }
        }
    }

    Ok(text.trim().to_string())
}

/// Stop recording, transcribe, and return the text + 16 kHz samples for history.
fn do_stop_recording(state: &AppState) -> Result<(String, Vec<f32>), String> {
    let sample_rate = state
        .sample_rate
        .lock()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No microphone available".to_string())?;

    // Atomically claim the "stopper" role — only one thread (hotkey or
    // auto-stop timer) can successfully stop a recording session.
    if state
        .is_recording
        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("目前未在錄音".to_string());
    }

    let samples: Vec<f32> = {
        let buf = state.buffer.lock().map_err(|e| e.to_string())?;
        buf.clone()
    };

    if samples.is_empty() {
        return Err("沒有錄到任何聲音".to_string());
    }

    println!(
        "[OpenTypeless] Raw captured: {} samples @ {} Hz = {:.2}s",
        samples.len(),
        sample_rate,
        samples.len() as f64 / sample_rate as f64
    );

    let t0 = Instant::now();
    let mut samples_16k = if sample_rate != 16000 {
        let resampled = resample(&samples, sample_rate, 16000);
        println!("[OpenTypeless] Resample {} Hz → 16 kHz: {:.0?}", sample_rate, t0.elapsed());
        resampled
    } else {
        samples
    };

    // Strip leading silence so the model doesn't hallucinate filler words
    // ("恩", "嗯") for the quiet period before the user starts speaking.
    // We look for the first 10-ms window whose RMS energy exceeds -40 dB,
    // then keep 100 ms of audio before that onset as context.
    const SILENCE_RMS_THRESHOLD: f32 = 0.01; // ~-40 dB
    const WINDOW: usize = 160;               // 10 ms at 16 kHz
    const LOOKBACK: usize = 1600;            // 100 ms at 16 kHz

    let speech_onset = samples_16k
        .windows(WINDOW)
        .position(|w| {
            let rms = (w.iter().map(|&s| s * s).sum::<f32>() / WINDOW as f32).sqrt();
            rms > SILENCE_RMS_THRESHOLD
        })
        .unwrap_or(0);

    let trim_start = speech_onset.saturating_sub(LOOKBACK);
    if trim_start > 0 {
        println!(
            "[OpenTypeless] Trimmed {:.0} ms of leading silence (onset at {:.0} ms)",
            trim_start as f64 / 16.0,
            speech_onset as f64 / 16.0
        );
        samples_16k = samples_16k[trim_start..].to_vec();
    }

    // Strip trailing silence — scan backwards for the last window above threshold,
    // then keep 100 ms of audio after the last speech as context.
    if samples_16k.len() > WINDOW {
        let total = samples_16k.len();
        let last_speech = samples_16k
            .windows(WINDOW)
            .rposition(|w| {
                let rms = (w.iter().map(|&s| s * s).sum::<f32>() / WINDOW as f32).sqrt();
                rms > SILENCE_RMS_THRESHOLD
            })
            .map(|pos| pos + WINDOW) // end of the last active window
            .unwrap_or(total);

        let trim_end = (last_speech + LOOKBACK).min(total);
        if trim_end < total {
            println!(
                "[OpenTypeless] Trimmed {:.0} ms of trailing silence",
                (total - trim_end) as f64 / 16.0
            );
            samples_16k.truncate(trim_end);
        }
    }

    println!("[OpenTypeless] Audio after trim: {:.2}s ({} samples)", samples_16k.len() as f64 / 16000.0, samples_16k.len());

    let whisper_start = Instant::now();
    println!("[OpenTypeless] Transcribing via Whisper (local)...");
    let text = transcribe_with_cached_whisper(state, &samples_16k)?;
    println!("[OpenTypeless] Whisper raw: {} (inference took {:.0?})", text, whisper_start.elapsed());

    if text.is_empty() {
        Err("no_speech".to_string())
    } else {
        Ok((text, samples_16k))
    }
}

/// Simple linear interpolation resampler
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);
    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        let frac = src_idx - idx as f64;
        let sample = if idx + 1 < samples.len() {
            samples[idx] as f64 * (1.0 - frac) + samples[idx + 1] as f64 * frac
        } else {
            samples[idx.min(samples.len() - 1)] as f64
        };
        output.push(sample as f32);
    }
    output
}

/// Return the number of available CPU cores (performance cores on Apple Silicon).
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Simulate Cmd+V to paste clipboard content at the current cursor position.
fn paste_with_cmd_v() -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe { macos_ffi::simulate_cmd_v() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Shared logic: stop recording, transcribe, copy/paste, and hide the overlay.
/// Called by both the hotkey handler and the auto-stop timer.
fn stop_transcribe_and_paste(app: &AppHandle) {
    // Atomically claim the "processor" role — only one caller (hotkey or
    // auto-stop timer) can enter the pipeline.  This prevents double-paste
    // when both fire at roughly the same time (~30 s boundary).
    let state = app.state::<AppState>();
    if state
        .is_processing
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        println!("[OpenTypeless] stop_transcribe_and_paste: already processing, skipping");
        return;
    }

    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.emit("recording-status", "transcribing");
    }

    println!("[OpenTypeless] ⏹️ Stopping recording...");

    let app_handle = app.clone();
    std::thread::spawn(move || {
        let pipeline_start = Instant::now();
        let state = app_handle.state::<AppState>();

        let (auto_paste, polish_config, retention_days) = state
            .settings
            .lock()
            .map(|s| (s.auto_paste, s.polish.clone(), s.history_retention_days))
            .unwrap_or((true, polisher::PolishConfig::default(), 0));

        match do_stop_recording(&state) {
            Ok((text, samples_16k)) => {
                let transcribe_elapsed = pipeline_start.elapsed();
                println!("[OpenTypeless] ✅ Transcribed: {} (took {:.0?})", text, transcribe_elapsed);
                let raw_text = text.clone();
                let audio_duration_secs = samples_16k.len() as f64 / 16000.0;

                // ── AI Polishing ──
                let mut polish_config = polish_config;
                // Inject API key from keychain for cloud mode
                if polish_config.enabled && polish_config.mode == polisher::PolishMode::Cloud {
                    if let Ok(key) = keychain::load(polish_config.cloud.provider.as_key()) {
                        polish_config.cloud.api_key = key;
                    }
                }
                let (final_text, reasoning) = if polish_config.enabled {
                    let model_dir = models_dir();
                    if polisher::is_polish_ready(&model_dir, &polish_config) {
                        if let Some(overlay) = app_handle.get_webview_window("overlay") {
                            let _ = overlay.emit("recording-status", "polishing");
                        }
                        let mode_label = match polish_config.mode {
                            polisher::PolishMode::Cloud => format!("Cloud ({})", polish_config.cloud.model_id),
                            polisher::PolishMode::Local => format!("Local ({})", polish_config.model.display_name()),
                        };
                        println!("[OpenTypeless] ✨ Polishing with {}...", mode_label);

                        let context = state
                            .captured_context
                            .lock()
                            .ok()
                            .and_then(|mut c| c.take())
                            .unwrap_or_default();

                        let polish_start = Instant::now();
                        let result = polisher::polish_text(
                            &state.llm_model,
                            &model_dir,
                            &polish_config,
                            &context,
                            &text,
                        );
                        println!("[OpenTypeless] ✨ Polished: {:?} (took {:.0?})", result.text, polish_start.elapsed());
                        (result.text, result.reasoning)
                    } else {
                        println!("[OpenTypeless] Polish enabled but not ready (model missing or no API key), skipping");
                        (text, None)
                    }
                } else {
                    (text, None)
                };
                let text = final_text;

                // Emit result to main window so the Test wizard can use it
                if let Some(main_win) = app_handle.get_webview_window("main") {
                    let _ = main_win.emit("transcription-result", &text);
                }

                let clipboard_ok = match arboard::Clipboard::new() {
                    Ok(mut clipboard) => {
                        if let Err(e) = clipboard.set_text(&text) {
                            eprintln!("[OpenTypeless] Clipboard error: {}", e);
                            false
                        } else {
                            true
                        }
                    }
                    Err(e) => {
                        eprintln!("[OpenTypeless] Clipboard init error: {}", e);
                        false
                    }
                };

                if clipboard_ok {
                    // Wait for the pasteboard change to propagate to the target app.
                    // 30 ms was occasionally too short on loaded systems; 100 ms is safe.
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    if auto_paste {
                        let pasted = paste_with_cmd_v();
                        if pasted {
                            println!("[OpenTypeless] 📋 Auto-pasted at cursor");
                            if let Some(overlay) = app_handle.get_webview_window("overlay") {
                                let _ = overlay.emit("recording-status", "pasted");
                            }
                        } else {
                            println!("[OpenTypeless] 📋 Copied to clipboard (paste simulation failed)");
                            if let Some(overlay) = app_handle.get_webview_window("overlay") {
                                let _ = overlay.emit("recording-status", "copied");
                            }
                        }
                    } else {
                        println!("[OpenTypeless] 📋 Copied to clipboard (auto-paste disabled)");
                        if let Some(overlay) = app_handle.get_webview_window("overlay") {
                            let _ = overlay.emit("recording-status", "copied");
                        }
                    }
                }

                println!("[OpenTypeless] ⏱️ Total pipeline: {:.0?} (from stop-recording to paste-done)", pipeline_start.elapsed());

                // ── Save to history ──
                {
                    let entry_id = history::generate_id();
                    let stt_model = "Whisper large-v3-turbo-zh-TW".to_string();
                    let polish_model_name = if polish_config.enabled {
                        match polish_config.mode {
                            polisher::PolishMode::Cloud => {
                                format!("{} (Cloud/{})", polish_config.cloud.model_id, polish_config.cloud.provider.as_key())
                            }
                            polisher::PolishMode::Local => {
                                format!("{} (Local)", polish_config.model.display_name())
                            }
                        }
                    } else {
                        "None".to_string()
                    };
                    let has_audio = history::save_audio_wav(&audio_dir(), &entry_id, &samples_16k);
                    let entry = history::HistoryEntry {
                        id: entry_id,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as i64,
                        text: text.clone(),
                        raw_text,
                        reasoning,
                        stt_model,
                        polish_model: polish_model_name,
                        duration_secs: audio_duration_secs,
                        has_audio,
                    };
                    history::add_entry(&history_dir(), &audio_dir(), entry, retention_days);
                    println!("[OpenTypeless] 📝 History entry saved (audio={})", has_audio);
                }
            }
            Err(ref e) if e == "no_speech" => {
                println!("[OpenTypeless] No speech detected, skipping (took {:.0?})", pipeline_start.elapsed());
            }
            Err(e) => {
                eprintln!("[OpenTypeless] Transcription error: {} (after {:.0?})", e, pipeline_start.elapsed());
                if let Some(overlay) = app_handle.get_webview_window("overlay") {
                    let _ = overlay.emit("recording-status", "error");
                }
            }
        }

        // Pipeline is done — allow the hotkey to start a new recording.
        state.is_processing.store(false, Ordering::SeqCst);

        std::thread::sleep(std::time::Duration::from_millis(1500));
        let app_for_hide = app_handle.clone();
        let _ = app_handle.run_on_main_thread(move || {
            if let Some(overlay) = app_for_hide.get_webview_window("overlay") {
                #[cfg(target_os = "macos")]
                if let Ok(ns_win) = overlay.ns_window() {
                    unsafe {
                        macos_ffi::hide_window(ns_win);
                    }
                }
                #[cfg(not(target_os = "macos"))]
                let _ = overlay.hide();
            }
        });
    });
}

// ── Permissions ─────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod permissions {
    use std::ffi::c_void;

    // AVFoundation — AVCaptureDevice authorizationStatusForMediaType:
    #[link(name = "AVFoundation", kind = "framework")]
    extern "C" {}

    // ApplicationServices — AXIsProcessTrusted()
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }

    extern "C" {
        fn objc_getClass(name: *const u8) -> *mut c_void;
        fn sel_registerName(name: *const u8) -> *mut c_void;
        fn objc_msgSend();
    }

    /// Returns microphone authorization status:
    /// 0 = notDetermined, 1 = restricted, 2 = denied, 3 = authorized
    pub fn microphone_auth_status() -> i64 {
        unsafe {
            let cls = objc_getClass(b"AVCaptureDevice\0".as_ptr());
            if cls.is_null() {
                return 0;
            }
            let sel = sel_registerName(b"authorizationStatusForMediaType:\0".as_ptr());

            // AVMediaTypeAudio = @"soun"
            // We need an NSString. Use CFSTR-equivalent via NSString stringWithUTF8String:
            let ns_string_cls = objc_getClass(b"NSString\0".as_ptr());
            let sel_str = sel_registerName(b"stringWithUTF8String:\0".as_ptr());
            let make_str: unsafe extern "C" fn(*mut c_void, *mut c_void, *const u8) -> *mut c_void =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let media_type = make_str(ns_string_cls, sel_str, b"soun\0".as_ptr());

            let send: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i64 =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(cls, sel, media_type)
        }
    }

    /// Request microphone access (triggers the system prompt if undetermined).
    pub fn request_microphone_access() {
        unsafe {
            let cls = objc_getClass(b"AVCaptureDevice\0".as_ptr());
            if cls.is_null() {
                return;
            }
            let sel = sel_registerName(b"requestAccessForMediaType:completionHandler:\0".as_ptr());

            let ns_string_cls = objc_getClass(b"NSString\0".as_ptr());
            let sel_str = sel_registerName(b"stringWithUTF8String:\0".as_ptr());
            let make_str: unsafe extern "C" fn(*mut c_void, *mut c_void, *const u8) -> *mut c_void =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let media_type = make_str(ns_string_cls, sel_str, b"soun\0".as_ptr());

            // completionHandler is a block. We pass a minimal no-op block.
            // Block layout: isa, flags, reserved, invoke, descriptor
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
}

#[derive(Serialize)]
struct PermissionStatus {
    microphone: String,
    accessibility: bool,
}

#[tauri::command]
fn check_permissions() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        let mic_status = permissions::microphone_auth_status();
        let mic = match mic_status {
            3 => "granted",
            2 => "denied",
            1 => "denied", // restricted treated as denied
            _ => "undetermined",
        };
        PermissionStatus {
            microphone: mic.to_string(),
            accessibility: permissions::accessibility_trusted(),
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
fn open_permission_settings(permission_type: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let url = match permission_type.as_str() {
            "microphone" => {
                // First, trigger the system permission prompt if undetermined
                let status = permissions::microphone_auth_status();
                if status == 0 {
                    // undetermined — trigger the system prompt
                    permissions::request_microphone_access();
                    return Ok(());
                }
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
            }
            "accessibility" => {
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

// ── App Entry ───────────────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            cancel_recording,
            set_test_mode,
            set_context_override,
            get_settings,
            save_settings,
            update_hotkey,
            reset_settings,
            get_default_prompt,
            get_default_prompt_rules,
            test_polish,
            get_mic_status,
            check_model_status,
            download_model,
            check_llm_model_status,
            download_llm_model,
            save_api_key,
            get_api_key,
            get_history,
            delete_history_entry,
            clear_all_history,
            export_history_audio,
            get_history_storage_path,
            check_permissions,
            open_permission_settings,
        ])
        .setup(|app| {
            // ── Hide Dock icon (menu-bar-only app) ──
            #[cfg(target_os = "macos")]
            unsafe {
                macos_ffi::set_accessory_policy();
            }

            // ── Load settings ──
            let settings = load_settings();
            let hotkey_str = settings.hotkey.clone();

            // ── Pre-initialise audio pipeline ──
            // Spawn a persistent audio thread that builds the cpal stream once.
            // This eliminates ~400-500 ms of per-recording device-enum + stream-build latency.
            let is_recording = Arc::new(AtomicBool::new(false));
            let buffer = Arc::new(Mutex::new(Vec::new()));
            let (mic_available, sample_rate) =
                match spawn_audio_thread(Arc::clone(&buffer), Arc::clone(&is_recording)) {
                    Ok(sr) => (true, Some(sr)),
                    Err(e) => {
                        eprintln!("[OpenTypeless] Audio init failed: {}", e);
                        (false, None)
                    }
                };

            app.manage(AppState {
                is_recording,
                is_processing: AtomicBool::new(false),
                buffer,
                sample_rate: Mutex::new(sample_rate),
                settings: Mutex::new(settings.clone()),
                mic_available: AtomicBool::new(mic_available),
                whisper_ctx: Mutex::new(None),
                llm_model: Mutex::new(None),
                captured_context: Mutex::new(None),
                context_override: Mutex::new(None),
                test_mode: AtomicBool::new(false),
                last_hotkey_time: Mutex::new(Instant::now() - std::time::Duration::from_secs(1)),
            });

            // ── Auto-show settings when model is missing ──
            if !models_dir().join("ggml-large-v3-turbo-zh-TW.bin").exists() {
                show_settings_window(app.handle());
            }

            // ── Pre-warm models in background ──
            // Loading Whisper + LLM lazily costs ~3s on first use.
            // Eagerly loading them at startup makes the first transcription instant.
            {
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    let warmup_start = Instant::now();
                    let state = app_handle.state::<AppState>();

                    // Pre-warm Whisper
                    if let Ok(model_path) = whisper_model_path() {
                        let mut ctx_guard = state.whisper_ctx.lock().unwrap();
                        if ctx_guard.is_none() {
                            println!("[OpenTypeless] Pre-warming Whisper model...");
                            let mut ctx_params = WhisperContextParameters::new();
                            ctx_params.use_gpu(true);
                            match WhisperContext::new_with_params(
                                model_path.to_str().unwrap_or_default(),
                                ctx_params,
                            ) {
                                Ok(ctx) => {
                                    *ctx_guard = Some(ctx);
                                    println!("[OpenTypeless] Whisper model pre-warmed ({:.0?})", warmup_start.elapsed());
                                }
                                Err(e) => {
                                    eprintln!("[OpenTypeless] Whisper pre-warm failed: {}", e);
                                }
                            }
                        }
                    }

                    // Pre-warm LLM (only for local mode)
                    let polish_config = state.settings.lock()
                        .map(|s| s.polish.clone())
                        .unwrap_or_default();
                    if polish_config.enabled && polish_config.mode == polisher::PolishMode::Local {
                        let model_dir = models_dir();
                        if polisher::model_file_exists(&model_dir, &polish_config.model) {
                            let llm_start = Instant::now();
                            println!("[OpenTypeless] Pre-warming LLM ({})...", polish_config.model.display_name());
                            polisher::ensure_model_loaded(&state.llm_model, &model_dir, &polish_config);
                            println!("[OpenTypeless] LLM pre-warmed ({:.0?})", llm_start.elapsed());
                        }
                    }

                    println!("[OpenTypeless] All models pre-warmed ({:.0?} total)", warmup_start.elapsed());
                });
            }

            // ── System Tray ──
            let settings_i =
                MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
            let quit_i =
                MenuItem::with_id(app, "quit", "Quit OpenTypeless", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

            let tooltip_label = hotkey_display_label(&hotkey_str);
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png")).unwrap())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip(format!("OpenTypeless – {} to record", tooltip_label))
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => {
                        show_settings_window(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        show_settings_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // ── Window close → hide ──
            if let Some(main_window) = app.get_webview_window("main") {
                let win = main_window.clone();
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });
            }

            // ── Configure overlay as non-activating floating panel ──
            #[cfg(target_os = "macos")]
            if let Some(overlay) = app.get_webview_window("overlay") {
                if let Ok(ns_win) = overlay.ns_window() {
                    unsafe { macos_ffi::setup_overlay(ns_win); }
                }
            }

            // ── Global Shortcut ──
            #[cfg(desktop)]
            {
                let shortcut = parse_hotkey_string(&hotkey_str)
                    .unwrap_or(Shortcut::new(Some(Modifiers::ALT | Modifiers::SUPER), Code::KeyR));

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, _shortcut, event| {
                            if event.state() != ShortcutState::Pressed {
                                return;
                            }

                            let state = app.state::<AppState>();

                            // In test mode, only emit the event — skip recording entirely
                            if state.test_mode.load(Ordering::SeqCst) {
                                if let Some(main_win) = app.get_webview_window("main") {
                                    let _ = main_win.emit("hotkey-activated", true);
                                }
                                return;
                            }

                            // Debounce: ignore key-repeat events from macOS.
                            // Key-repeat fires additional Pressed events ~500 ms after
                            // the initial press; a 300 ms guard prevents accidental
                            // start-then-immediate-stop toggles.
                            {
                                let now = Instant::now();
                                if let Ok(mut last) = state.last_hotkey_time.lock() {
                                    if now.duration_since(*last) < std::time::Duration::from_millis(300) {
                                        return;
                                    }
                                    *last = now;
                                }
                            }

                            // Block re-entry while the transcription pipeline is running.
                            // This prevents accidental double-paste from key repeats or
                            // rapid double-presses of the hotkey.
                            if state.is_processing.load(Ordering::SeqCst) {
                                return;
                            }

                            let is_recording = state.is_recording.load(Ordering::SeqCst);

                            if !is_recording {
                                // ── Start Recording ──
                                //
                                // CRITICAL: start capturing audio FIRST, before any
                                // UI work.  The stream is always-on, so this just
                                // flips a flag — true zero latency.
                                // Capture frontmost app context BEFORE starting recording,
                                // while the user is still in their target app.
                                let captured_ctx = state.context_override.lock()
                                    .ok()
                                    .and_then(|ctx| ctx.clone())
                                    .unwrap_or_else(|| context_detect::detect_frontmost_app());

                                match do_start_recording(&state) {
                                    Ok(()) => {
                                        println!("[OpenTypeless] 🎙️ Recording started");

                                        // Store captured context for later use by polisher
                                        if let Ok(mut ctx) = state.captured_context.lock() {
                                            *ctx = Some(captured_ctx);
                                        }

                                        // Notify the main (settings) window so the Test wizard can react
                                        if let Some(main_win) = app.get_webview_window("main") {
                                            let _ = main_win.emit("hotkey-activated", true);
                                        }

                                        // Now show the overlay (non-blocking from audio's perspective)
                                        if let Some(overlay) = app.get_webview_window("overlay") {
                                            let _ = overlay.emit("recording-status", "recording");
                                            let _ = overlay.emit("recording-max-duration", MAX_RECORDING_SECS);
                                            if let Ok(Some(monitor)) = overlay.current_monitor() {
                                                let screen = monitor.size();
                                                let scale = monitor.scale_factor();
                                                let win_w = 300.0;
                                                let win_h = 52.0;
                                                let x = (screen.width as f64 / scale - win_w) / 2.0;
                                                let y = screen.height as f64 / scale - win_h - 80.0;
                                                let _ = overlay.set_position(
                                                    tauri::PhysicalPosition::new(
                                                        (x * scale) as i32,
                                                        (y * scale) as i32,
                                                    ),
                                                );
                                            }
                                            #[cfg(target_os = "macos")]
                                            if let Ok(ns_win) = overlay.ns_window() {
                                                unsafe { macos_ffi::show_no_activate(ns_win); }
                                            }
                                            #[cfg(not(target_os = "macos"))]
                                            let _ = overlay.show();
                                        }

                                        // Spawn monitoring thread for audio level visualisation
                                        let app_for_monitor = app.clone();
                                        std::thread::spawn(move || {
                                            let state = app_for_monitor.state::<AppState>();
                                            let sr = state.sample_rate.lock().ok().and_then(|v| *v).unwrap_or(44100) as usize;
                                            let recording_start = Instant::now();

                                            // Stream audio levels at ~50 ms intervals
                                            const NUM_BARS: usize = 20;
                                            let samples_per_bar = sr / 20; // 50 ms of audio

                                            while state.is_recording.load(Ordering::SeqCst) {
                                                // Auto-stop when max duration is reached
                                                if recording_start.elapsed().as_secs() >= MAX_RECORDING_SECS {
                                                    println!("[OpenTypeless] ⏱️ Max recording duration reached ({}s)", MAX_RECORDING_SECS);
                                                    stop_transcribe_and_paste(&app_for_monitor);
                                                    return;
                                                }
                                                let levels: Vec<f32> = if let Ok(buf) = state.buffer.lock() {
                                                    if buf.is_empty() {
                                                        vec![0.0; NUM_BARS]
                                                    } else {
                                                        let total = NUM_BARS * samples_per_bar;
                                                        let start = buf.len().saturating_sub(total);
                                                        let mut bars: Vec<f32> = buf[start..]
                                                            .chunks(samples_per_bar)
                                                            .map(|chunk| {
                                                                let rms = (chunk.iter().map(|&s| s * s).sum::<f32>()
                                                                    / chunk.len() as f32)
                                                                    .sqrt();
                                                                (rms * 6.0).min(1.0)
                                                            })
                                                            .collect();
                                                        while bars.len() < NUM_BARS {
                                                            bars.insert(0, 0.0);
                                                        }
                                                        bars
                                                    }
                                                } else {
                                                    vec![0.0; NUM_BARS]
                                                };

                                                if let Some(ov) = app_for_monitor.get_webview_window("overlay") {
                                                    let _ = ov.emit("audio-levels", &levels);
                                                }
                                                std::thread::sleep(std::time::Duration::from_millis(50));
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "[OpenTypeless] Failed to start recording: {}",
                                            e
                                        );
                                        // Hide overlay on failure
                                        if let Some(overlay) = app.get_webview_window("overlay") {
                                            #[cfg(target_os = "macos")]
                                            if let Ok(ns_win) = overlay.ns_window() {
                                                unsafe { macos_ffi::hide_window(ns_win); }
                                            }
                                            #[cfg(not(target_os = "macos"))]
                                            let _ = overlay.hide();
                                        }
                                    }
                                }
                            } else {
                                // ── Stop Recording + Transcribe ──
                                stop_transcribe_and_paste(app);
                            }
                        })
                        .build(),
                )?;

                app.global_shortcut().register(shortcut)?;
                let label = hotkey_display_label(&hotkey_str);
                println!(
                    "[OpenTypeless] {} global shortcut registered",
                    label
                );
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn show_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_size(tauri::LogicalSize::new(960.0, 720.0));
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
