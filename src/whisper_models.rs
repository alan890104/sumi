use serde::{Deserialize, Serialize};

use crate::settings::models_dir;
use crate::system_info::{SystemInfo, detect_system_language};

// ── WhisperModel enum ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhisperModel {
    #[default]
    LargeV3Turbo,
    LargeV3TurboQ5,
    Medium,
    Small,
    Base,
    LargeV3TurboZhTw,
}

impl WhisperModel {
    pub fn filename(&self) -> &'static str {
        match self {
            Self::LargeV3Turbo => "ggml-large-v3-turbo.bin",
            Self::LargeV3TurboQ5 => "ggml-large-v3-turbo-q5_0.bin",
            Self::Medium => "ggml-medium.bin",
            Self::Small => "ggml-small.bin",
            Self::Base => "ggml-base.bin",
            Self::LargeV3TurboZhTw => "ggml-large-v3-turbo-zh-TW.bin",
        }
    }

    /// Returns the download URL for this model, or `None` if it's a custom/legacy model
    /// with no public URL.
    pub fn download_url(&self) -> Option<&'static str> {
        match self {
            Self::LargeV3Turbo => Some(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
            ),
            Self::LargeV3TurboQ5 => Some(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
            ),
            Self::Medium => Some(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
            ),
            Self::Small => Some(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            ),
            Self::Base => Some(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
            ),
            Self::LargeV3TurboZhTw => Some(
                "https://huggingface.co/Alkd/whisper-large-v3-turbo-zh-TW/resolve/main/ggml-model.bin",
            ),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::LargeV3Turbo => "Whisper Turbo",
            Self::LargeV3TurboQ5 => "Whisper Turbo Lite",
            Self::Medium => "Whisper Medium",
            Self::Small => "Whisper Small",
            Self::Base => "Whisper Base",
            Self::LargeV3TurboZhTw => "Whisper Turbo TW",
        }
    }

    pub fn size_bytes(&self) -> u64 {
        match self {
            Self::LargeV3Turbo => 1_620_000_000,
            Self::LargeV3TurboQ5 => 547_000_000,
            Self::Medium => 1_530_000_000,
            Self::Small => 488_000_000,
            Self::Base => 148_000_000,
            Self::LargeV3TurboZhTw => 1_600_000_000,
        }
    }

    pub fn languages(&self) -> &'static [&'static str] {
        match self {
            Self::LargeV3TurboZhTw => &["zh-TW"],
            _ => &["multilingual"],
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::LargeV3Turbo => "Highest multilingual accuracy",
            Self::LargeV3TurboQ5 => "High quality, compact size (quantized)",
            Self::Medium => "Balanced speed and quality",
            Self::Small => "Lightweight and fast",
            Self::Base => "Fastest, smallest footprint",
            Self::LargeV3TurboZhTw => "Best for Traditional Chinese",
        }
    }

    pub fn all() -> &'static [WhisperModel] {
        &[
            Self::LargeV3Turbo,
            Self::LargeV3TurboQ5,
            Self::Base,
            Self::LargeV3TurboZhTw,
        ]
    }
}

// ── WhisperModelInfo (for frontend serialization) ────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct WhisperModelInfo {
    pub id: WhisperModel,
    pub display_name: &'static str,
    pub description: &'static str,
    pub size_bytes: u64,
    pub languages: &'static [&'static str],
    pub downloaded: bool,
    pub file_size_on_disk: u64,
    pub is_active: bool,
}

impl WhisperModelInfo {
    pub fn from_model(model: &WhisperModel, active_model: &WhisperModel) -> Self {
        let dir = models_dir();
        let path = dir.join(model.filename());
        let (downloaded, file_size_on_disk) = match std::fs::metadata(&path) {
            Ok(m) => (true, m.len()),
            Err(_) => (false, 0),
        };
        Self {
            id: model.clone(),
            display_name: model.display_name(),
            description: model.description(),
            size_bytes: model.size_bytes(),
            languages: model.languages(),
            downloaded,
            file_size_on_disk,
            is_active: model == active_model,
        }
    }
}

/// Recommend a model based on system info and language preference.
///
/// Effective memory selection:
/// - Apple Silicon → system RAM (unified memory shared with GPU)
/// - CUDA enabled + discrete GPU with >= 2 GB VRAM → GPU VRAM
/// - Otherwise → system RAM
pub fn recommend_model(system: &SystemInfo, settings_language: Option<&str>) -> WhisperModel {
    let lang = settings_language
        .map(|l| l.to_lowercase())
        .or_else(detect_system_language)
        .unwrap_or_default();

    let prefers_zh_tw = lang.starts_with("zh-tw") || lang.starts_with("zh_tw")
        || lang.starts_with("zh-hant") || lang.starts_with("zh_hant");
    let _prefers_zh = lang.starts_with("zh") || lang == "chinese";

    let ram_gb = system.total_ram_bytes as f64 / 1_073_741_824.0;
    let vram_gb = system.gpu_vram_bytes as f64 / 1_073_741_824.0;
    let disk_gb = system.available_disk_bytes as f64 / 1_073_741_824.0;

    let effective_gb = if system.is_apple_silicon {
        ram_gb
    } else if system.has_cuda && vram_gb >= 2.0 {
        vram_gb
    } else {
        ram_gb
    };

    if effective_gb >= 8.0 && disk_gb >= 3.0 {
        if prefers_zh_tw {
            return WhisperModel::LargeV3TurboZhTw;
        }
        WhisperModel::LargeV3Turbo
    } else if effective_gb >= 4.0 && disk_gb >= 1.0 {
        WhisperModel::LargeV3TurboQ5
    } else {
        WhisperModel::Base
    }
}
