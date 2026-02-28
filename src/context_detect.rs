use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppContext {
    pub app_name: String,
    pub bundle_id: String,
    pub url: String,
    /// Original terminal app name when `app_name` was enriched with a CLI tool name.
    /// Empty when no enrichment occurred.
    #[serde(default)]
    pub terminal_host: String,
}

// ── Terminal subprocess detection ────────────────────────────────────────────

/// Known terminal emulator bundle IDs and their display names.
#[cfg(target_os = "macos")]
const TERMINAL_BUNDLE_IDS: &[(&str, &str)] = &[
    ("com.apple.Terminal", "Terminal"),
    ("com.googlecode.iterm2", "iTerm2"),
    ("com.mitchellh.ghostty", "Ghostty"),
    ("dev.warp.Warp-Stable", "Warp"),
    ("com.github.warp-terminal", "Warp"),
];

/// A CLI tool that can be detected inside a terminal.
#[cfg(target_os = "macos")]
struct CliTool {
    process_names: &'static [&'static str],
    title_keywords: &'static [&'static str],
    display_name: &'static str,
}

/// Known CLI tools to detect inside terminals.
#[cfg(target_os = "macos")]
const CLI_TOOLS: &[CliTool] = &[
    // AI coding assistants
    CliTool { process_names: &["claude"], title_keywords: &["claude code"], display_name: "Claude Code" },
    CliTool { process_names: &["gemini"], title_keywords: &["gemini"], display_name: "Gemini CLI" },
    CliTool { process_names: &["codex"], title_keywords: &["codex"], display_name: "Codex CLI" },
    CliTool { process_names: &["aider"], title_keywords: &["aider"], display_name: "Aider" },
    // Terminal editors
    CliTool { process_names: &["nvim"], title_keywords: &["neovim", "nvim"], display_name: "Neovim" },
    CliTool { process_names: &["vim"], title_keywords: &[], display_name: "Vim" },
    CliTool { process_names: &["emacs"], title_keywords: &["emacs"], display_name: "Emacs" },
    CliTool { process_names: &["hx"], title_keywords: &["helix"], display_name: "Helix" },
];

/// Detect the frontmost macOS application and, if it's a known browser, the current URL.
/// If it's a terminal, attempt to detect a known CLI tool running inside it.
#[cfg(target_os = "macos")]
pub fn detect_frontmost_app() -> AppContext {
    let (app_name, bundle_id) = get_frontmost_app_info();
    let url = get_browser_url(&bundle_id);

    let mut ctx = AppContext {
        app_name,
        bundle_id,
        url,
        terminal_host: String::new(),
    };

    // Enrich terminal apps with subprocess detection
    if let Some(terminal_name) = lookup_terminal(&ctx.bundle_id) {
        if let Some(tool_name) = detect_terminal_subprocess(&ctx.bundle_id, terminal_name) {
            ctx.terminal_host = ctx.app_name.clone();
            ctx.app_name = tool_name;
        }
    }

    ctx
}

#[cfg(target_os = "windows")]
pub fn detect_frontmost_app() -> AppContext {
    let app_name = get_foreground_app_name_windows();
    AppContext {
        app_name,
        bundle_id: String::new(),
        url: String::new(),
        terminal_host: String::new(),
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn detect_frontmost_app() -> AppContext {
    AppContext::default()
}

/// Get the foreground application's executable name on Windows.
#[cfg(target_os = "windows")]
fn get_foreground_app_name_windows() -> String {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::Foundation::CloseHandle;

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return String::new();
        }

        let mut pid: u32 = 0;
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return String::new();
        }

        let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) => h,
            Err(_) => return String::new(),
        };

        let mut buf = [0u16; 260];
        let mut size = buf.len() as u32;
        let ok = windows::Win32::System::Threading::QueryFullProcessImageNameW(
            handle,
            windows::Win32::System::Threading::PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);

        if ok.is_ok() {
            let path = String::from_utf16_lossy(&buf[..size as usize]);
            // Extract just the filename without extension
            path.rsplit('\\')
                .next()
                .unwrap_or("")
                .strip_suffix(".exe")
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        }
    }
}

/// Run an AppleScript snippet and return its stdout (trimmed), or empty string on failure.
#[cfg(target_os = "macos")]
fn run_osascript(script: &str) -> String {
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

/// Check if a bundle ID is a known terminal emulator. Returns the terminal display name.
#[cfg(target_os = "macos")]
fn lookup_terminal(bundle_id: &str) -> Option<&'static str> {
    TERMINAL_BUNDLE_IDS
        .iter()
        .find(|(bid, _)| *bid == bundle_id)
        .map(|(_, name)| *name)
}

/// Match a process list or title string against known CLI tools.
#[cfg(target_os = "macos")]
fn match_cli_tool_by_processes(process_list: &str) -> Option<&'static str> {
    let lower = process_list.to_lowercase();
    // Split on commas (AppleScript list format: "login, -zsh, claude, caffeinate")
    let procs: Vec<&str> = lower.split(',').map(|s| s.trim().trim_start_matches('-')).collect();
    for tool in CLI_TOOLS {
        for pname in tool.process_names {
            if procs.iter().any(|p| *p == *pname) {
                return Some(tool.display_name);
            }
        }
    }
    None
}

/// Match a window/tab title against known CLI tools.
#[cfg(target_os = "macos")]
fn match_cli_tool_by_title(title: &str) -> Option<&'static str> {
    let lower = title.to_lowercase();
    for tool in CLI_TOOLS {
        // Check title keywords
        for kw in tool.title_keywords {
            if lower.contains(kw) {
                return Some(tool.display_name);
            }
        }
        // Also check process names as a fallback (title often contains the command name)
        for pname in tool.process_names {
            if lower.contains(pname) {
                return Some(tool.display_name);
            }
        }
    }
    None
}

/// Detect a known CLI tool running inside the given terminal.
/// Uses a 3-tier detection cascade:
/// 1. Terminal.app: process list of selected tab
/// 2. iTerm2: session title
/// 3. Universal fallback: front window title via System Events
#[cfg(target_os = "macos")]
fn detect_terminal_subprocess(bundle_id: &str, _terminal_name: &str) -> Option<String> {
    match bundle_id {
        "com.apple.Terminal" => {
            // Tier 1: Terminal.app — get process list of the selected tab
            let output = run_osascript(
                r#"tell application "Terminal" to get processes of selected tab of front window"#,
            );
            if !output.is_empty() {
                if let Some(tool) = match_cli_tool_by_processes(&output) {
                    println!("[Sumi] Terminal subprocess detected: {} (processes: {})", tool, output);
                    return Some(tool.to_string());
                }
            }
        }
        "com.googlecode.iterm2" => {
            // Tier 2: iTerm2 — get session name (often reflects the running command)
            let output = run_osascript(
                r#"tell application "iTerm2" to get name of current session of current tab of current window"#,
            );
            if !output.is_empty() {
                if let Some(tool) = match_cli_tool_by_title(&output) {
                    println!("[Sumi] iTerm2 subprocess detected: {} (title: {})", tool, output);
                    return Some(tool.to_string());
                }
            }
        }
        _ => {}
    }

    // Tier 3: Universal fallback — check the front window title via System Events
    let output = run_osascript(
        r#"tell application "System Events" to get name of front window of (first process whose frontmost is true)"#,
    );
    if !output.is_empty() {
        if let Some(tool) = match_cli_tool_by_title(&output) {
            println!("[Sumi] Terminal subprocess detected via window title: {} (title: {})", tool, output);
            return Some(tool.to_string());
        }
    }

    None
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

#[cfg(target_os = "macos")]
use crate::platform::macos::nsstring_to_string;

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

    run_osascript(script)
}

#[cfg(not(target_os = "macos"))]
fn get_browser_url(_bundle_id: &str) -> String {
    String::new()
}
