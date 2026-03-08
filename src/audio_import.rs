//! Audio file import for Meeting Notes.
//!
//! Decodes audio files (WAV, MP3, M4A, OGG, FLAC) using symphonia,
//! resamples to 16 kHz mono, and runs chunked STT to produce a
//! meeting note transcript.

use std::path::Path;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager};

use crate::meeting_notes;

/// One ASR segment with its timestamps and optional word-level timestamps.
/// For Whisper: one per Whisper-native segment (with proper timestamps).
/// For Qwen3-ASR/Cloud: one per 30 s chunk (no sub-segmentation).
struct ImportSegment {
    start: f64,
    end: f64,
    text: String,
    words: Vec<meeting_notes::WordTs>,
}

/// Closure that transcribes a chunk of 16 kHz audio and returns per-segment results.
///
/// Parameters: `(samples_16khz, audio_start_secs, prev_context)`.
/// Returns a `Vec<ImportSegment>` — multiple segments for Whisper (with native
/// timestamps), or a single segment for Qwen3-ASR/Cloud.
type ImportTranscribeFn =
    Box<dyn FnMut(&[f32], f64, &str) -> Vec<ImportSegment> + Send>;
use crate::settings;

/// Assign speaker labels to ASR segments by merging with diarization segments.
///
/// Ported from WhisperX's `assign_word_speakers`: for each word (or segment
/// when word timestamps are unavailable), find the diarization segment with
/// the largest time overlap and assign its speaker label.
///
/// For Whisper DTW timestamps where `word.e == word.s` (point timestamps),
/// this degenerates to a containment check: `dia.start <= word.s < dia.end`.
fn assign_word_speakers(
    segments: &[ImportSegment],
    diar_segs: &[(f64, f64, String)],
) -> Vec<meeting_notes::WalSegment> {
    if diar_segs.is_empty() {
        return segments
            .iter()
            .filter(|s| !s.text.is_empty())
            .map(|s| meeting_notes::WalSegment {
                speaker: String::new(),
                start: s.start,
                end: s.end,
                text: s.text.clone(),
                words: s.words.clone(),
            })
            .collect();
    }

    let mut result: Vec<meeting_notes::WalSegment> = Vec::new();

    for seg in segments {
        if seg.text.is_empty() {
            continue;
        }

        if seg.words.is_empty() {
            // No word timestamps (Qwen3-ASR, most cloud providers).
            // Assign the entire segment to the speaker with the most overlap.
            let speaker = find_best_speaker(seg.start, seg.end, diar_segs);
            result.push(meeting_notes::WalSegment {
                speaker,
                start: seg.start,
                end: seg.end,
                text: seg.text.clone(),
                words: vec![],
            });
            continue;
        }

        // Word-level speaker assignment (WhisperX algorithm).
        // Group consecutive words by speaker to produce sub-segments.
        let mut current_speaker = String::new();
        let mut current_words: Vec<meeting_notes::WordTs> = Vec::new();
        let mut sub_start = seg.start;

        for word in &seg.words {
            let word_speaker = if (word.e - word.s).abs() < 1e-6 {
                find_containing_speaker(word.s, diar_segs)
            } else {
                find_best_speaker(word.s, word.e, diar_segs)
            };

            if word_speaker != current_speaker && !current_words.is_empty() {
                let sub_end = current_words.last().map(|w| w.e.max(w.s)).unwrap_or(sub_start);
                let text = current_words
                    .iter()
                    .map(|w| w.w.as_str())
                    .collect::<String>()
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    result.push(meeting_notes::WalSegment {
                        speaker: current_speaker.clone(),
                        start: sub_start,
                        end: sub_end,
                        text,
                        words: std::mem::take(&mut current_words),
                    });
                } else {
                    current_words.clear();
                }
                sub_start = word.s;
            }

            current_speaker = word_speaker;
            current_words.push(word.clone());
        }

        // Flush remaining words.
        if !current_words.is_empty() {
            let sub_end = current_words.last().map(|w| w.e.max(w.s)).unwrap_or(sub_start);
            let text = current_words
                .iter()
                .map(|w| w.w.as_str())
                .collect::<String>()
                .trim()
                .to_string();
            if !text.is_empty() {
                result.push(meeting_notes::WalSegment {
                    speaker: current_speaker,
                    start: sub_start,
                    end: sub_end,
                    text,
                    words: current_words,
                });
            }
        }
    }

    result
}

/// Find the diarization segment that contains a point timestamp.
/// Falls back to the nearest segment if none contains the point.
fn find_containing_speaker(t: f64, diar_segs: &[(f64, f64, String)]) -> String {
    // Exact containment.
    for (s, e, spk) in diar_segs {
        if *s <= t && t < *e {
            return spk.clone();
        }
    }
    // Fallback: nearest segment by distance to midpoint.
    diar_segs
        .iter()
        .min_by(|(s1, e1, _), (s2, e2, _)| {
            let d1 = if t < *s1 { s1 - t } else if t > *e1 { t - e1 } else { 0.0 };
            let d2 = if t < *s2 { s2 - t } else if t > *e2 { t - e2 } else { 0.0 };
            d1.partial_cmp(&d2).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, _, spk)| spk.clone())
        .unwrap_or_default()
}

/// Find the speaker with the largest time overlap for a given interval [start, end).
/// WhisperX's intersection-duration algorithm.
fn find_best_speaker(start: f64, end: f64, diar_segs: &[(f64, f64, String)]) -> String {
    let mut best_speaker = String::new();
    let mut best_overlap = 0.0_f64;

    for (ds, de, spk) in diar_segs {
        let overlap = (end.min(*de) - start.max(*ds)).max(0.0);
        if overlap > best_overlap {
            best_overlap = overlap;
            best_speaker = spk.clone();
        }
    }

    if best_speaker.is_empty() {
        // No overlap — find nearest segment.
        find_containing_speaker((start + end) / 2.0, diar_segs)
    } else {
        best_speaker
    }
}

/// Renumber speaker labels so that the first speaker to appear in the
/// transcript is SPEAKER_00, the second is SPEAKER_01, etc.
fn renumber_speakers_chronologically(segments: &mut [meeting_notes::WalSegment]) {
    // Build mapping: old label → new label, ordered by first appearance.
    let mut seen: Vec<String> = Vec::new();
    for seg in segments.iter() {
        if !seg.speaker.is_empty() && !seen.contains(&seg.speaker) {
            seen.push(seg.speaker.clone());
        }
    }
    if seen.len() <= 1 {
        return; // 0 or 1 speakers — nothing to renumber.
    }
    // Apply mapping.
    for seg in segments.iter_mut() {
        if let Some(pos) = seen.iter().position(|s| s == &seg.speaker) {
            seg.speaker = format!("SPEAKER_{:02}", pos);
        }
    }
}

/// Decode an audio file to mono f32 samples.
/// Returns (samples, sample_rate, duration_secs).
fn decode_audio_file(path: &str) -> Result<(Vec<f32>, u32, f64), String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let path = Path::new(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "wav" | "mp3" | "m4a" | "aac" | "ogg" | "flac" => {}
        "mp4" | "mov" | "mkv" | "avi" | "webm" => {
            return Err("video_not_supported".to_string());
        }
        _ => {
            return Err(format!("unsupported_format:{ext}"));
        }
    }

    let file =
        std::fs::File::open(path).map_err(|e| format!("Failed to open file: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    hint.with_extension(&ext);

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Failed to probe audio format: {e}"))?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("No audio track found in file")?;

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or("Could not determine sample rate")?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Failed to create decoder: {e}"))?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(symphonia::core::errors::Error::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => return Err(format!("Failed to read packet: {e}")),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                let num_frames = decoded.frames();
                // Read channel count from the decoded buffer spec, NOT
                // track metadata. M4A files often omit channel info in
                // codec params, causing unwrap_or(1) to treat stereo
                // interleaved [L,R,L,R,...] data as mono → garbled audio.
                let ch = spec.channels.count();
                let mut sample_buf =
                    SampleBuffer::<f32>::new(num_frames as u64, spec);
                sample_buf.copy_interleaved_ref(decoded);

                let samples = sample_buf.samples();
                if ch <= 1 {
                    all_samples.extend_from_slice(samples);
                } else {
                    for chunk in samples.chunks(ch) {
                        all_samples
                            .push(chunk.iter().sum::<f32>() / ch as f32);
                    }
                }
            }
            Err(symphonia::core::errors::Error::DecodeError(e)) => {
                tracing::warn!("[import] decode error (skipping packet): {e}");
            }
            Err(e) => return Err(format!("Failed to decode audio: {e}")),
        }
    }

    if all_samples.is_empty() {
        return Err("No audio data found in file".to_string());
    }

    let duration_secs = all_samples.len() as f64 / sample_rate as f64;
    Ok((all_samples, sample_rate, duration_secs))
}

/// Run the import pipeline: decode → resample → chunk → STT → WAL → finalize.
///
/// Called on a blocking thread from the Tauri command.
pub fn run_import(app: &AppHandle, file_path: &str) -> Result<String, String> {
    let state = app.state::<crate::AppState>();

    // Prevent concurrent imports.
    if state
        .import_active
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("import_already_active".to_string());
    }
    state.import_cancelled.store(false, Ordering::SeqCst);

    let result = run_import_inner(app, file_path);

    state.import_active.store(false, Ordering::SeqCst);
    result
}

fn run_import_inner(app: &AppHandle, file_path: &str) -> Result<String, String> {
    let state = app.state::<crate::AppState>();

    // ── Step 1: Decode audio file ──
    tracing::info!("[import] decoding: {}", file_path);
    let _ = app.emit(
        "import-progress",
        serde_json::json!({ "status": "decoding", "progress": 0.0 }),
    );

    let (samples, sample_rate, duration_secs) = decode_audio_file(file_path)?;
    let decoded_rms = crate::audio::rms(&samples);
    tracing::info!(
        "[import] decoded: {:.1}s @ {} Hz, {} samples (mono), RMS={:.5}",
        duration_secs,
        sample_rate,
        samples.len(),
        decoded_rms
    );

    // ── Step 2: Resample to 16 kHz ──
    let samples_16k = if sample_rate != 16000 {
        crate::audio::resample(&samples, sample_rate, 16000)
    } else {
        samples
    };

    let total_samples = samples_16k.len();
    if total_samples == 0 {
        return Err("No audio data after resampling".to_string());
    }

    // ── Step 3: Determine STT engine ──
    let stt_config = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.stt.clone()
    };
    let language = stt_config.language.clone();

    let model_label = match stt_config.mode {
        crate::stt::SttMode::Cloud => {
            format!(
                "Cloud (Import) – {}",
                stt_config.cloud.provider.as_key()
            )
        }
        crate::stt::SttMode::Local => match stt_config.local_engine {
            crate::stt::LocalSttEngine::Qwen3Asr => {
                format!(
                    "Qwen3-ASR (Import) – {}",
                    stt_config.qwen3_asr_model.display_name()
                )
            }
            crate::stt::LocalSttEngine::Whisper => {
                format!(
                    "Whisper (Import) – {}",
                    stt_config.whisper_model.display_name()
                )
            }
        },
    };

    // ── Step 4: Create meeting note ──
    let note_id = crate::history::generate_id();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let title = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported Audio")
        .to_string();

    let note = meeting_notes::MeetingNote {
        id: note_id.clone(),
        title,
        transcript: String::new(),
        created_at: now,
        updated_at: now,
        duration_secs,
        stt_model: model_label,
        is_recording: true, // Use WAL-based flow
        word_count: 0,
        summary: String::new(),
        audio_path: None,
    };

    let history_dir = settings::history_dir();
    meeting_notes::create_note(&history_dir, &note)
        .map_err(|e| format!("Failed to create note: {e}"))?;

    let _ = app.emit(
        "meeting-note-created",
        serde_json::json!({ "id": note_id, "note": note }),
    );

    // ── Step 5: Build transcribe closure ──
    // Returns Vec<ImportSegment> — one per Whisper segment (with native
    // timestamps) or one per chunk (Qwen3-ASR / Cloud).
    let mut transcribe: ImportTranscribeFn = match stt_config.mode {
        crate::stt::SttMode::Local => match stt_config.local_engine {
            crate::stt::LocalSttEngine::Qwen3Asr => {
                let model = stt_config.qwen3_asr_model.clone();
                let lang = language.clone();
                let app_c = app.clone();
                Box::new(move |samples: &[f32], start_secs, _prev| {
                    let st = app_c.state::<crate::AppState>();
                    let text = crate::qwen3_asr::transcribe_with_cached_qwen3_asr(
                        &st.qwen3_asr_ctx,
                        samples,
                        &model,
                        &lang,
                    )
                    .unwrap_or_default();
                    let end_secs = start_secs + samples.len() as f64 / 16_000.0;
                    if text.is_empty() {
                        vec![]
                    } else {
                        vec![ImportSegment { start: start_secs, end: end_secs, text, words: vec![] }]
                    }
                })
            }
            crate::stt::LocalSttEngine::Whisper => {
                let lang = language.clone();
                let app_c = app.clone();
                Box::new(move |samples, audio_start_secs, prev_text| {
                    let st = app_c.state::<crate::AppState>();
                    let ctx_guard =
                        st.whisper_ctx.lock().unwrap_or_else(|e| e.into_inner());
                    // Use per-segment function with proper Whisper timestamps.
                    let whisper_segs = crate::whisper_streaming::transcribe_import_segments(
                        &ctx_guard,
                        samples,
                        &lang,
                        if prev_text.is_empty() { None } else { Some(prev_text) },
                        audio_start_secs,
                    )
                    .unwrap_or_default();
                    whisper_segs
                        .into_iter()
                        .map(|ws| ImportSegment {
                            start: ws.start,
                            end: ws.end,
                            text: ws.text,
                            words: ws.words,
                        })
                        .collect()
                })
            }
        },
        crate::stt::SttMode::Cloud => {
            let mut cloud = stt_config.cloud.clone();
            cloud.language = language.clone();
            let key = crate::commands::get_cached_api_key(
                &state.api_key_cache,
                cloud.provider.as_key(),
            );
            if !key.is_empty() {
                cloud.api_key = key;
            }
            let app_c = app.clone();
            Box::new(move |samples: &[f32], start_secs, prev_text| {
                let st = app_c.state::<crate::AppState>();
                let prompt = if prev_text.is_empty() { None } else { Some(prev_text) };
                let text = crate::stt::run_cloud_stt(
                    &cloud,
                    samples,
                    &st.http_client,
                    prompt,
                )
                .unwrap_or_else(|e| {
                    tracing::warn!("[import] cloud STT failed: {e}");
                    String::new()
                });
                let end_secs = start_secs + samples.len() as f64 / 16_000.0;
                if text.is_empty() {
                    vec![]
                } else {
                    vec![ImportSegment { start: start_secs, end: end_secs, text, words: vec![] }]
                }
            })
        }
    };

    // ── Phase 1: ASR in 30 s chunks (progress 0–70%) ──
    //
    // WhisperX-style pipeline: transcribe the entire file first, collecting
    // per-segment timestamps and word-level timestamps, then diarize, then
    // merge by timestamps.
    //
    // Each 30 s chunk may produce multiple Whisper segments with their own
    // native start/end times (when using Whisper with no_timestamps=false).
    // Qwen3-ASR / Cloud produce one segment per chunk.
    let chunk_size = 30 * 16_000; // 30 s at 16 kHz
    let mut all_segments: Vec<ImportSegment> = Vec::new();

    for chunk_start in (0..total_samples).step_by(chunk_size) {
        if state.import_cancelled.load(Ordering::SeqCst) {
            tracing::info!("[import] cancelled by user during ASR phase");
            let _ = meeting_notes::delete_note(&history_dir, &note_id);
            let _ = app.emit("meeting-note-finalized", serde_json::json!({ "id": note_id }));
            return Err("cancelled".to_string());
        }

        let chunk_end = (chunk_start + chunk_size).min(total_samples);
        let chunk = &samples_16k[chunk_start..chunk_end];
        let start_secs = chunk_start as f64 / 16_000.0;

        let prev_text = {
            let full = meeting_notes::read_wal(&history_dir, &note_id);
            meeting_notes::wal_text_for_context(&full, 200)
        };

        let segments = if chunk.is_empty() {
            vec![]
        } else {
            transcribe(chunk, start_secs, &prev_text)
        };

        // Write each segment to WAL immediately (no speaker label yet) so the
        // frontend can show incremental progress and crash recovery works.
        for seg in &segments {
            let text = crate::maybe_convert_zh(&seg.text, &language);
            if !text.is_empty() {
                let wal_seg = meeting_notes::WalSegment {
                    speaker: String::new(),
                    start: seg.start,
                    end: seg.end,
                    text: text.clone(),
                    words: seg.words.clone(),
                };
                meeting_notes::append_wal(&history_dir, &note_id, &wal_seg);
                let _ = app.emit(
                    "meeting-note-updated",
                    serde_json::json!({
                        "id": note_id,
                        "delta": text,
                        "speaker": "",
                        "start": seg.start,
                        "end": seg.end,
                        "duration_secs": duration_secs,
                    }),
                );
            }
        }

        // Collect segments with zh-converted text for the merge phase.
        for mut seg in segments {
            seg.text = crate::maybe_convert_zh(&seg.text, &language);
            if !seg.text.is_empty() {
                all_segments.push(seg);
            }
        }

        // Progress: 0–70% for ASR.
        let asr_progress = chunk_end as f64 / total_samples as f64 * 0.7;
        let _ = app.emit(
            "import-progress",
            serde_json::json!({
                "id": note_id,
                "progress": asr_progress,
                "status": "transcribing",
            }),
        );
    }

    tracing::info!(
        "[import] ASR complete: {} segments from {:.1}s audio",
        all_segments.len(),
        duration_secs
    );

    // ── Phase 2: Diarization on full file (progress 70–95%) ──
    let emb_path = settings::diarization_model_path();
    let seg_path = settings::segmentation_model_path();
    let diar_segs: Vec<(f64, f64, String)> = if emb_path.exists() && seg_path.exists() {
        if state.import_cancelled.load(Ordering::SeqCst) {
            tracing::info!("[import] cancelled by user before diarization");
            let _ = meeting_notes::delete_note(&history_dir, &note_id);
            let _ = app.emit("meeting-note-finalized", serde_json::json!({ "id": note_id }));
            return Err("cancelled".to_string());
        }

        match crate::diarization::DiarizationEngine::new(&emb_path, Some(&seg_path)) {
            Ok(mut engine) => {
                tracing::info!("[import] diarization engine loaded");
                let _ = app.emit(
                    "import-progress",
                    serde_json::json!({ "status": "diarizing", "progress": 0.7 }),
                );
                let app_c = app.clone();
                let segs = engine.diarize_full(&samples_16k, Some(&move |done, total| {
                    // Map diarization progress to 70–95%.
                    let p = 0.7 + (done as f64 / total as f64) * 0.25;
                    let _ = app_c.emit(
                        "import-progress",
                        serde_json::json!({
                            "status": "diarizing",
                            "progress": p,
                        }),
                    );
                }));
                tracing::info!(
                    "[import] diarization: {} segments, {} speakers",
                    segs.len(),
                    segs.iter()
                        .map(|(_, _, s)| s.as_str())
                        .collect::<std::collections::HashSet<_>>()
                        .len()
                );
                segs
            }
            Err(e) => {
                tracing::warn!("[import] failed to load diarization engine: {e}");
                vec![]
            }
        }
    } else {
        tracing::info!("[import] diarization models not found, running without speaker labels");
        vec![]
    };

    // ── Phase 3: Merge ASR + diarization by timestamps (WhisperX-style) ──
    // Rewrite WAL with speaker labels assigned from diarization segments.
    if !diar_segs.is_empty() {
        let _ = app.emit(
            "import-progress",
            serde_json::json!({ "status": "merging", "progress": 0.95 }),
        );

        let mut merged = assign_word_speakers(&all_segments, &diar_segs);

        // Post-process: renumber speakers by chronological first appearance
        // so that the first person to speak is always SPEAKER_00.
        renumber_speakers_chronologically(&mut merged);

        // Phase 4: Rewrite WAL with speaker-labeled segments.
        let mut wal_content = String::new();
        for seg in &merged {
            if let Ok(line) = serde_json::to_string(seg) {
                wal_content.push_str(&line);
                wal_content.push('\n');
            }
        }
        meeting_notes::write_wal(&history_dir, &note_id, &wal_content);

        // Notify frontend to re-render with speaker labels.
        let _ = app.emit(
            "meeting-note-refresh",
            serde_json::json!({ "id": note_id }),
        );

        let unique_speakers: std::collections::HashSet<_> =
            merged.iter().map(|s| s.speaker.as_str()).collect();
        tracing::info!(
            "[import] merge complete: {} segments, {} speakers",
            merged.len(),
            unique_speakers.len()
        );
    }

    // ── Step 7: Finalize ──
    let transcript = meeting_notes::read_wal(&history_dir, &note_id);
    meeting_notes::finalize_note(&history_dir, &note_id, &transcript, duration_secs)
        .map_err(|e| format!("Failed to finalize note: {e}"))?;
    meeting_notes::remove_wal(&history_dir, &note_id);

    let _ = app.emit(
        "meeting-note-finalized",
        serde_json::json!({ "id": note_id }),
    );
    let _ = app.emit(
        "import-progress",
        serde_json::json!({
            "id": note_id,
            "progress": 1.0,
            "status": "complete",
        }),
    );

    tracing::info!(
        "[import] complete: {} chars, {:.1}s audio",
        transcript.len(),
        duration_secs
    );
    Ok(note_id)
}
