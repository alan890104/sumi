use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: i64,
    pub text: String,
    pub raw_text: String,
    pub stt_model: String,
    pub polish_model: String,
    pub duration_secs: f64,
    pub has_audio: bool,
}

fn history_path(history_dir: &Path) -> PathBuf {
    history_dir.join("history.json")
}

fn audio_path(audio_dir: &Path, id: &str) -> PathBuf {
    audio_dir.join(format!("{}.wav", id))
}

pub fn load_history(history_dir: &Path) -> Vec<HistoryEntry> {
    let path = history_path(history_dir);
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_history(history_dir: &Path, entries: &[HistoryEntry]) {
    let path = history_path(history_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(entries) {
        let _ = std::fs::write(&path, json);
    }
}

pub fn add_entry(history_dir: &Path, audio_dir: &Path, entry: HistoryEntry, retention_days: u32) {
    let mut entries = load_history(history_dir);
    entries.insert(0, entry);
    if retention_days > 0 {
        cleanup_expired(&mut entries, audio_dir, retention_days);
    }
    save_history(history_dir, &entries);
}

/// Remove entries older than `retention_days` and delete their audio files.
fn cleanup_expired(entries: &mut Vec<HistoryEntry>, audio_dir: &Path, retention_days: u32) {
    use std::time::SystemTime;
    let now_millis = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let cutoff = now_millis - (retention_days as i64) * 86_400_000;

    let mut i = 0;
    while i < entries.len() {
        if entries[i].timestamp < cutoff {
            let removed = entries.remove(i);
            let wav = audio_path(audio_dir, &removed.id);
            if wav.exists() {
                let _ = std::fs::remove_file(&wav);
            }
        } else {
            i += 1;
        }
    }
}

pub fn delete_entry(history_dir: &Path, audio_dir: &Path, id: &str) {
    let mut entries = load_history(history_dir);
    entries.retain(|e| e.id != id);
    save_history(history_dir, &entries);

    let wav = audio_path(audio_dir, id);
    if wav.exists() {
        let _ = std::fs::remove_file(&wav);
    }
}

/// Delete all history entries and remove the entire audio directory.
pub fn clear_all(history_dir: &Path, audio_dir: &Path) {
    save_history(history_dir, &[]);

    if audio_dir.exists() {
        let _ = std::fs::remove_dir_all(audio_dir);
    }
}

pub fn save_audio_wav(audio_dir: &Path, id: &str, samples_16k: &[f32]) -> bool {
    if std::fs::create_dir_all(audio_dir).is_err() {
        return false;
    }
    let path = audio_path(audio_dir, id);
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    match hound::WavWriter::create(&path, spec) {
        Ok(mut writer) => {
            for &s in samples_16k {
                let clamped = s.clamp(-1.0, 1.0);
                let val = (clamped * 32767.0) as i16;
                if writer.write_sample(val).is_err() {
                    return false;
                }
            }
            writer.finalize().is_ok()
        }
        Err(_) => false,
    }
}

pub fn export_audio(audio_dir: &Path, id: &str) -> Result<PathBuf, String> {
    let src = audio_path(audio_dir, id);
    if !src.exists() {
        return Err("Audio file not found".to_string());
    }
    let downloads = dirs::download_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("Downloads")
    });
    let _ = std::fs::create_dir_all(&downloads);
    let dest = downloads.join(format!("{}.wav", id));
    std::fs::copy(&src, &dest).map_err(|e| format!("Failed to copy audio: {}", e))?;
    Ok(dest)
}

pub fn generate_id() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();

    // Convert to local-ish components for a readable ID
    // We'll use a simple timestamp-based format
    let ts = chrono_free_format(secs);
    format!("{}_{:03}", ts, millis)
}

/// Format seconds-since-epoch as YYYYMMDD_HHMMSS without chrono dependency.
fn chrono_free_format(epoch_secs: u64) -> String {
    // Use libc localtime for the conversion
    #[cfg(unix)]
    {
        let t = epoch_secs as libc::time_t;
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        unsafe {
            libc::localtime_r(&t, &mut tm);
        }
        format!(
            "{:04}{:02}{:02}_{:02}{:02}{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec,
        )
    }
    #[cfg(not(unix))]
    {
        format!("{}", epoch_secs)
    }
}
