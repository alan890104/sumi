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
}

impl Default for Settings {
    fn default() -> Self {
        let (hotkey, edit_hotkey) = if is_debug() {
            ("Alt+Super+KeyZ".to_string(), Some("Control+Alt+Super+KeyZ".to_string()))
        } else {
            ("Alt+KeyZ".to_string(), Some("Control+Alt+KeyZ".to_string()))
        };
        Self {
            hotkey,
            auto_paste: true,
            polish: polisher::PolishConfig::default(),
            history_retention_days: 0,
            language: None,
            stt: SttConfig::default(),
            edit_hotkey,
            onboarding_completed: false,
        }
    }
}

// ── Consolidated data directory: ~/.sumi (release) or ~/.sumi-dev (debug) ────

pub const fn is_debug() -> bool {
    cfg!(debug_assertions)
}

pub fn base_dir() -> PathBuf {
    let dir_name = if is_debug() { ".sumi-dev" } else { ".sumi" };
    dirs::home_dir()
        .expect("no home dir")
        .join(dir_name)
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
    let is_new_install = !path.exists();
    let mut settings = if !is_new_install {
        match std::fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[Sumi] Settings file corrupted ({}), using defaults", e);
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        }
    } else {
        Settings::default()
    };
    settings.stt.migrate_language();

    // Resolve "auto" STT language to system locale (covers new installs AND
    // edge cases where the file existed but language was never properly set).
    // Also initialise polish cloud model_id for new installs from the same locale.
    if is_new_install || settings.stt.language == "auto" || settings.stt.language.is_empty() {
        if let Some(locale) = crate::whisper_models::detect_system_language() {
            if is_new_install {
                settings.polish.cloud.model_id =
                    polisher::CloudConfig::default_model_id_for_locale(&locale).to_string();
            }
            let lang = crate::stt::locale_to_stt_language(&locale);
            if lang != "auto" {
                settings.stt.language = lang.clone();
                settings.stt.cloud.language = lang;
            }
        }
    }

    // Initialise UI language from system locale when not explicitly set.
    // Covers new installs AND upgrades from older versions that never populated
    // settings.language.  Without this the frontend falls back to
    // navigator.language in WKWebView, which returns "en" when the app bundle
    // lacks localisation resources (no .lproj directories).
    if settings.language.is_none() {
        if let Some(locale) = crate::whisper_models::detect_system_language() {
            let lang = crate::stt::locale_to_stt_language(&locale);
            if lang != "auto" {
                settings.language = Some(lang);
            }
        }
    }

    // Ensure polish cloud model_id is never empty (e.g. upgraded from older version).
    if settings.polish.cloud.model_id.is_empty() {
        let locale = crate::whisper_models::detect_system_language().unwrap_or_default();
        settings.polish.cloud.model_id =
            polisher::CloudConfig::default_model_id_for_locale(&locale).to_string();
    }

    // Persist any detected/migrated values back to disk so they survive
    // even if the app exits without a frontend save.
    save_settings_to_disk(&settings);

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
