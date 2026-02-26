use serde::{Deserialize, Serialize};

use crate::settings::models_dir;

// ── WhisperModel enum ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhisperModel {
    #[default]
    LargeV3Turbo,
    LargeV3TurboQ5,
    BelleZh,
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
            Self::BelleZh => "ggml-belle-whisper-large-v3-turbo-zh.bin",
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
            Self::BelleZh => Some(
                "https://huggingface.co/alikia2x/belle-whisper-large-v3-turbo-zh-ggml/resolve/main/ggml-model.bin",
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
            Self::BelleZh => "Belle Simplified Chinese",
            Self::Medium => "Whisper Medium",
            Self::Small => "Whisper Small",
            Self::Base => "Whisper Base",
            Self::LargeV3TurboZhTw => "Whisper Turbo Traditional Chinese",
        }
    }

    pub fn size_bytes(&self) -> u64 {
        match self {
            Self::LargeV3Turbo => 1_620_000_000,
            Self::LargeV3TurboQ5 => 547_000_000,
            Self::BelleZh => 1_600_000_000,
            Self::Medium => 1_530_000_000,
            Self::Small => 488_000_000,
            Self::Base => 148_000_000,
            Self::LargeV3TurboZhTw => 1_600_000_000,
        }
    }

    pub fn languages(&self) -> &'static [&'static str] {
        match self {
            Self::BelleZh => &["zh"],
            Self::LargeV3TurboZhTw => &["zh-TW"],
            _ => &["multilingual"],
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::LargeV3Turbo => "Highest multilingual accuracy",
            Self::LargeV3TurboQ5 => "High quality, compact size (quantized)",
            Self::BelleZh => "Best for Simplified Chinese",
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
            Self::BelleZh,
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

// ── SystemInfo ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub total_ram_bytes: u64,
    pub available_disk_bytes: u64,
    pub is_apple_silicon: bool,
    pub os: String,
    pub arch: String,
}

/// Detect system information (RAM, disk space, CPU architecture).
pub fn detect_system_info() -> SystemInfo {
    let total_ram_bytes = get_total_ram();
    let available_disk_bytes = get_available_disk_space();
    let arch = std::env::consts::ARCH.to_string();
    let is_apple_silicon = cfg!(target_os = "macos") && arch == "aarch64";

    SystemInfo {
        total_ram_bytes,
        available_disk_bytes,
        is_apple_silicon,
        os: std::env::consts::OS.to_string(),
        arch,
    }
}

/// Recommend a model based on system info and language preference.
pub fn recommend_model(system: &SystemInfo, settings_language: Option<&str>) -> WhisperModel {
    let lang = settings_language
        .map(|l| l.to_lowercase())
        .or_else(|| {
            std::env::var("LANG")
                .or_else(|_| std::env::var("LC_ALL"))
                .ok()
                .map(|l| l.to_lowercase())
        })
        .unwrap_or_default();

    let prefers_zh_tw = lang.starts_with("zh-tw") || lang.starts_with("zh_tw") || lang == "zh-hant";
    let prefers_zh = lang.starts_with("zh") || lang == "chinese";

    let ram_gb = system.total_ram_bytes as f64 / 1_073_741_824.0;
    let disk_gb = system.available_disk_bytes as f64 / 1_073_741_824.0;

    if ram_gb >= 8.0 && disk_gb >= 3.0 {
        if prefers_zh_tw {
            return WhisperModel::LargeV3TurboZhTw;
        }
        if prefers_zh {
            return WhisperModel::BelleZh;
        }
        WhisperModel::LargeV3Turbo
    } else if ram_gb >= 4.0 && disk_gb >= 1.0 {
        WhisperModel::LargeV3TurboQ5
    } else {
        WhisperModel::Base
    }
}

// ── Platform-specific system info helpers ─────────────────────────────────────

#[cfg(unix)]
fn get_total_ram() -> u64 {
    #[cfg(target_os = "macos")]
    {
        use std::mem;
        let mut size: u64 = 0;
        let mut len = mem::size_of::<u64>();
        let mib = [libc::CTL_HW, libc::HW_MEMSIZE];
        let ret = unsafe {
            libc::sysctl(
                mib.as_ptr() as *mut _,
                2,
                &mut size as *mut u64 as *mut _,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if ret == 0 {
            size
        } else {
            0
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        unsafe {
            let info: libc::sysinfo = std::mem::zeroed();
            if libc::sysinfo(&info as *const _ as *mut _) == 0 {
                info.totalram as u64 * info.mem_unit as u64
            } else {
                0
            }
        }
    }
}

#[cfg(not(unix))]
fn get_total_ram() -> u64 {
    0
}

fn get_available_disk_space() -> u64 {
    let models = models_dir();
    // Ensure the directory exists for statvfs
    let _ = std::fs::create_dir_all(&models);

    #[cfg(unix)]
    {
        use std::ffi::CString;
        let path_c = match CString::new(models.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(path_c.as_ptr(), &mut stat) == 0 {
                stat.f_bavail as u64 * stat.f_frsize as u64
            } else {
                0
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = models;
        0
    }
}
