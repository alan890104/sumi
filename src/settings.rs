use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::polisher;
use crate::stt::SttConfig;

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
    #[serde(default)]
    pub stt: SttConfig,
    /// Optional hotkey for "Edit by Voice" — select text, speak editing instruction.
    #[serde(default)]
    pub edit_hotkey: Option<String>,
    /// Whether the onboarding wizard has been completed. `false` triggers the setup overlay.
    #[serde(default)]
    pub onboarding_completed: bool,
    /// When true, show a preview overlay after transcription instead of pasting immediately.
    #[serde(default)]
    pub preview_before_paste: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Alt+KeyZ".to_string(),
            auto_paste: true,
            polish: polisher::PolishConfig::default(),
            history_retention_days: 0,
            language: None,
            stt: SttConfig::default(),
            edit_hotkey: Some("Control+Alt+KeyZ".to_string()),
            onboarding_completed: false,
            preview_before_paste: false,
        }
    }
}

// ── Consolidated data directory: ~/.sumi ─────────────────────────────────────

pub fn base_dir() -> PathBuf {
    dirs::home_dir()
        .expect("no home dir")
        .join(".sumi")
}

pub fn config_dir() -> PathBuf {
    base_dir().join("config")
}

pub fn models_dir() -> PathBuf {
    base_dir().join("models")
}

pub fn history_dir() -> PathBuf {
    base_dir().join("history")
}

pub fn audio_dir() -> PathBuf {
    base_dir().join("audio")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load_settings() -> Settings {
    let path = settings_path();
    let mut settings = if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    } else {
        Settings::default()
    };
    settings.stt.migrate_language();
    settings
}

pub fn save_settings_to_disk(settings: &Settings) {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(&path, json);
    }
}
