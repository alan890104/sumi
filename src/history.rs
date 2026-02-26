use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: i64,
    pub text: String,
    pub raw_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    pub stt_model: String,
    pub polish_model: String,
    pub duration_secs: f64,
    pub has_audio: bool,
    pub stt_elapsed_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polish_elapsed_ms: Option<u64>,
    pub total_elapsed_ms: u64,
}

fn db_path(history_dir: &Path) -> PathBuf {
    history_dir.join("history.db")
}

fn audio_path(audio_dir: &Path, id: &str) -> PathBuf {
    audio_dir.join(format!("{}.wav", id))
}

fn open_db(history_dir: &Path) -> Result<Connection, rusqlite::Error> {
    let _ = std::fs::create_dir_all(history_dir);
    let conn = Connection::open(db_path(history_dir))?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            id               TEXT PRIMARY KEY,
            timestamp        INTEGER NOT NULL,
            text             TEXT NOT NULL,
            raw_text         TEXT NOT NULL,
            reasoning        TEXT,
            stt_model        TEXT NOT NULL,
            polish_model     TEXT NOT NULL,
            duration_secs    REAL NOT NULL,
            has_audio        INTEGER NOT NULL DEFAULT 0,
            stt_elapsed_ms   INTEGER NOT NULL DEFAULT 0,
            polish_elapsed_ms INTEGER,
            total_elapsed_ms INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_history_timestamp ON history(timestamp DESC);",
    )?;
    Ok(conn)
}

/// Delete old `history.json` and clear leftover audio from the JSON era.
/// Idempotent — safe to call on every startup.
pub fn migrate_from_json(history_dir: &Path, audio_dir: &Path) {
    let json_path = history_dir.join("history.json");
    if json_path.exists() {
        println!("[Voxink] Migrating: removing legacy history.json");
        let _ = std::fs::remove_file(&json_path);
        if audio_dir.exists() {
            let _ = std::fs::remove_dir_all(audio_dir);
        }
    }
}

pub fn load_history(history_dir: &Path) -> Vec<HistoryEntry> {
    let conn = match open_db(history_dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[Voxink] Failed to open history DB: {}", e);
            return Vec::new();
        }
    };
    let mut stmt = match conn.prepare(
        "SELECT id, timestamp, text, raw_text, reasoning, stt_model, polish_model,
                duration_secs, has_audio, stt_elapsed_ms, polish_elapsed_ms, total_elapsed_ms
         FROM history ORDER BY timestamp DESC",
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[Voxink] Failed to prepare history query: {}", e);
            return Vec::new();
        }
    };
    let rows = stmt.query_map([], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            text: row.get(2)?,
            raw_text: row.get(3)?,
            reasoning: row.get(4)?,
            stt_model: row.get(5)?,
            polish_model: row.get(6)?,
            duration_secs: row.get(7)?,
            has_audio: row.get::<_, i32>(8)? != 0,
            stt_elapsed_ms: row.get::<_, i64>(9).unwrap_or(0) as u64,
            polish_elapsed_ms: row.get::<_, Option<i64>>(10).ok().flatten().map(|v| v as u64),
            total_elapsed_ms: row.get::<_, i64>(11).unwrap_or(0) as u64,
        })
    });
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(e) => {
            eprintln!("[Voxink] Failed to query history: {}", e);
            Vec::new()
        }
    }
}

pub fn add_entry(history_dir: &Path, audio_dir: &Path, entry: HistoryEntry, retention_days: u32) {
    let conn = match open_db(history_dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[Voxink] Failed to open history DB for insert: {}", e);
            return;
        }
    };
    let has_audio_int: i32 = if entry.has_audio { 1 } else { 0 };
    let polish_ms: Option<i64> = entry.polish_elapsed_ms.map(|v| v as i64);
    if let Err(e) = conn.execute(
        "INSERT OR REPLACE INTO history
            (id, timestamp, text, raw_text, reasoning, stt_model, polish_model,
             duration_secs, has_audio, stt_elapsed_ms, polish_elapsed_ms, total_elapsed_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            entry.id,
            entry.timestamp,
            entry.text,
            entry.raw_text,
            entry.reasoning,
            entry.stt_model,
            entry.polish_model,
            entry.duration_secs,
            has_audio_int,
            entry.stt_elapsed_ms as i64,
            polish_ms,
            entry.total_elapsed_ms as i64,
        ],
    ) {
        eprintln!("[Voxink] Failed to insert history entry: {}", e);
    }
    if retention_days > 0 {
        cleanup_expired(&conn, audio_dir, retention_days);
    }
}

fn cleanup_expired(conn: &Connection, audio_dir: &Path, retention_days: u32) {
    let now_millis = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let cutoff = now_millis - (retention_days as i64) * 86_400_000;

    // Collect IDs of expired entries that have audio, so we can delete their WAV files.
    let ids: Vec<String> = {
        let mut stmt = match conn.prepare(
            "SELECT id FROM history WHERE timestamp < ?1 AND has_audio = 1",
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        stmt.query_map(params![cutoff], |row| row.get(0))
            .ok()
            .map(|iter| iter.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
    };
    for id in &ids {
        let wav = audio_path(audio_dir, id);
        if wav.exists() {
            let _ = std::fs::remove_file(&wav);
        }
    }
    let _ = conn.execute("DELETE FROM history WHERE timestamp < ?1", params![cutoff]);
}

pub fn delete_entry(history_dir: &Path, audio_dir: &Path, id: &str) {
    if let Ok(conn) = open_db(history_dir) {
        let _ = conn.execute("DELETE FROM history WHERE id = ?1", params![id]);
    }
    let wav = audio_path(audio_dir, id);
    if wav.exists() {
        let _ = std::fs::remove_file(&wav);
    }
}

pub fn clear_all(history_dir: &Path, audio_dir: &Path) {
    if let Ok(conn) = open_db(history_dir) {
        let _ = conn.execute("DELETE FROM history", []);
    }
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

    let ts = chrono_free_format(secs);
    format!("{}_{:03}", ts, millis)
}

/// Format seconds-since-epoch as YYYYMMDD_HHMMSS without chrono dependency.
fn chrono_free_format(epoch_secs: u64) -> String {
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
