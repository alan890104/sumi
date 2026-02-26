use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

pub fn parse_key_code(s: &str) -> Option<Code> {
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

pub fn parse_hotkey_string(s: &str) -> Option<Shortcut> {
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

pub fn hotkey_display_label(s: &str) -> String {
    let parts: Vec<&str> = s.split('+').collect();
    let mut labels = Vec::new();
    for part in &parts {
        let label = match *part {
            #[cfg(target_os = "macos")]
            "Alt" => "⌥",
            #[cfg(target_os = "macos")]
            "Control" => "⌃",
            #[cfg(target_os = "macos")]
            "Shift" => "⇧",
            #[cfg(target_os = "macos")]
            "Super" => "⌘",
            #[cfg(not(target_os = "macos"))]
            "Alt" => "Alt",
            #[cfg(not(target_os = "macos"))]
            "Control" => "Ctrl",
            #[cfg(not(target_os = "macos"))]
            "Shift" => "Shift",
            #[cfg(not(target_os = "macos"))]
            "Super" => "Win",
            other => {
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
