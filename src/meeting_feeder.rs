//! Shared meeting feeder loop.
//!
//! All three meeting feeders (Qwen3-ASR, Whisper, Cloud) follow the same
//! structure: 2 s tick → audio drain → VAD silence detection → transcribe
//! on silence → WAL append → emit event.  The only difference is the actual
//! STT call, which is injected via a closure.

use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::segment_spacing::SpacingState;

/// Read the last ~200 characters from the WAL file for use as a context prompt.
fn read_wal_context(history_dir: &std::path::Path, note_id: &Option<String>) -> String {
    if let Some(ref id) = note_id {
        let full = crate::meeting_notes::read_wal(history_dir, id);
        let trimmed = full.trim_end();
        // Use char-aware slicing to avoid panics on multi-byte UTF-8 (Chinese, etc.)
        let char_count = trimmed.chars().count();
        if char_count > 200 {
            trimmed.char_indices()
                .nth(char_count - 200)
                .map(|(i, _)| &trimmed[i..])
                .unwrap_or(trimmed)
                .to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        String::new()
    }
}

/// Persist a delta to the WAL file and emit a `meeting-note-updated` event.
fn persist_and_emit(
    app: &AppHandle,
    state: &crate::AppState,
    history_dir: &std::path::Path,
    note_id: &Option<String>,
    delta: &str,
) {
    if delta.is_empty() {
        return;
    }
    if let Some(ref id) = note_id {
        crate::meeting_notes::append_wal(history_dir, id, delta);
        let duration = state
            .meeting_start_time
            .lock()
            .ok()
            .and_then(|st| *st)
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);
        let _ = app.emit(
            "meeting-note-updated",
            serde_json::json!({
                "id": id,
                "delta": delta,
                "duration_secs": duration,
            }),
        );
    }
}

/// Shared meeting feeder loop.
///
/// `label` is used for log messages (e.g. `"qwen3-meeting"`).
///
/// `max_segment_samples` enables force-flushing segments that exceed a
/// duration cap (e.g. `Some(120 * 16_000)` for 120 s).  Pass `None` to
/// disable.
///
/// `transcribe` receives VAD-filtered 16 kHz samples and the previous WAL
/// context string (~200 chars).  It should return the transcribed text
/// segment, or an empty string on failure / no speech.
pub(crate) fn run_meeting_feeder(
    app: AppHandle,
    session_id: u64,
    label: &str,
    max_segment_samples: Option<usize>,
    mut transcribe: impl FnMut(&[f32], &str) -> String,
) {
    let state = app.state::<crate::AppState>();
    let sr = state
        .sample_rate
        .lock()
        .ok()
        .and_then(|v| *v)
        .unwrap_or(44100);

    let history_dir = crate::settings::history_dir();
    let note_id: Option<String> = state
        .active_meeting_note_id
        .lock()
        .ok()
        .and_then(|nid| nid.clone());

    let mut chunk_buf: Vec<f32> = Vec::new();
    let mut silence_count: u32 = 0;
    let mut had_speech_since_reset = false;
    const RMS_FALLBACK: f32 = 0.003;

    let mut spacing = SpacingState::new();

    let mut last_tail: usize = 0;
    let waveform_keep: usize = sr as usize * 2;

    // ── Main loop ────────────────────────────────────────────────────────

    loop {
        {
            let guard = state
                .feeder_stop_mu
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let _ = state
                .feeder_stop_cv
                .wait_timeout(guard, Duration::from_millis(2000));
        }

        if !state.is_recording.load(Ordering::SeqCst) {
            break;
        }

        // Session guard.
        let current_session = state.meeting_session.load(Ordering::SeqCst);
        if current_session != session_id {
            tracing::warn!(
                "[{label}] stale feeder (session {session_id} vs current {current_session}) — aborting",
            );
            return;
        }

        // Drain new audio delta; partial-drain keeping waveform_keep for the
        // audio-level monitor.
        let delta_raw: Vec<f32> = {
            let mut buf = state
                .buffer
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let tail = last_tail.min(buf.len());
            let delta = buf[tail..].to_vec();
            last_tail = buf.len();
            if last_tail > waveform_keep {
                let trim = last_tail - waveform_keep;
                buf.drain(..trim);
                last_tail -= trim;
            }
            delta
        };

        if delta_raw.is_empty() {
            if had_speech_since_reset {
                silence_count += 1;
            }
        } else {
            let delta_16k = if sr != 16000 {
                crate::audio::resample(&delta_raw, sr, 16000)
            } else {
                delta_raw
            };

            if crate::transcribe::has_speech_vad(&state.vad_ctx, &delta_16k, RMS_FALLBACK) {
                silence_count = 0;
                had_speech_since_reset = true;
            } else if had_speech_since_reset {
                silence_count += 1;
            }

            chunk_buf.extend_from_slice(&delta_16k);
        }

        // Transcribe segment on silence (≥ 2 s) or max segment length exceeded.
        let force_flush = max_segment_samples.map_or(false, |max| chunk_buf.len() >= max);
        if ((silence_count >= 1 && had_speech_since_reset) || force_flush)
            && !chunk_buf.is_empty()
        {
            if force_flush {
                tracing::info!(
                    "[{label}] segment reached {}s — force-flushing",
                    chunk_buf.len() / 16_000
                );
            }

            let prev_text = read_wal_context(&history_dir, &note_id);

            let stt_samples = crate::transcribe::filter_with_vad(&state.vad_ctx, &chunk_buf)
                .unwrap_or_else(|_| chunk_buf.clone());

            let seg_text = if stt_samples.is_empty() {
                String::new()
            } else {
                transcribe(&stt_samples, &prev_text)
            };

            let tick_delta = spacing.build_tick_delta(&seg_text);
            persist_and_emit(&app, &state, &history_dir, &note_id, &tick_delta);

            chunk_buf.clear();
            silence_count = 0;
            had_speech_since_reset = false;
        }
    }

    // ── Post-loop guards ─────────────────────────────────────────────────

    let current_session = state.meeting_session.load(Ordering::SeqCst);
    if current_session != session_id {
        tracing::warn!(
            "[{label}] stale feeder (session {session_id} vs current {current_session}) — aborting post-loop",
        );
        return;
    }
    if state.meeting_cancelled.load(Ordering::SeqCst) {
        tracing::warn!("[{label}] feeder cancelled — partial transcript already persisted to file");
        return;
    }

    // Flush remaining chunk (audio received after the last silence window).
    if !chunk_buf.is_empty() {
        let prev_text = read_wal_context(&history_dir, &note_id);

        let stt_samples = crate::transcribe::filter_with_vad(&state.vad_ctx, &chunk_buf)
            .unwrap_or_else(|_| chunk_buf.clone());

        let seg_text = if stt_samples.is_empty() {
            String::new()
        } else {
            transcribe(&stt_samples, &prev_text)
        };

        if !seg_text.is_empty() {
            let final_delta = spacing.build_final_delta(&seg_text);
            persist_and_emit(&app, &state, &history_dir, &note_id, &final_delta);
        }
    }

    tracing::info!("[{label}] feeder finished — transcript persisted to WAL file");

    // Signal completion. stop_meeting_mode reads the transcript from the WAL file.
    state.meeting_active.store(false, Ordering::SeqCst);
}
