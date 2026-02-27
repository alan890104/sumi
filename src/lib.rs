mod audio;
mod commands;
mod context_detect;
mod credentials;
mod history;
mod hotkey;
mod permissions;
pub mod platform;
mod polisher;
pub mod settings;
pub mod stt;
mod transcribe;
pub mod whisper_models;

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use whisper_rs::WhisperContextParameters;

use commands::get_cached_api_key;
use hotkey::{hotkey_display_label, parse_hotkey_string};
use settings::{load_settings, models_dir, history_dir, audio_dir, Settings};
use stt::{SttConfig, SttMode};

const MAX_RECORDING_SECS: u64 = 30;

// ── Preview Payload (sent to overlay) ────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
struct PreviewPayload {
    text: String,
    hotkey: String,
}

// ── Pending Preview ─────────────────────────────────────────────────────────

pub struct PendingPreview {
    pub text: String,
    pub raw_text: String,
    pub reasoning: Option<String>,
    pub samples_16k: Vec<f32>,
    pub audio_duration_secs: f64,
    pub stt_config_snapshot: SttConfig,
    pub polish_config_snapshot: polisher::PolishConfig,
    pub stt_elapsed_ms: u64,
    pub polish_elapsed_ms: Option<u64>,
    pub pipeline_start: Instant,
    pub chars_per_sec: f64,
    pub history_context: context_detect::AppContext,
    pub is_edit: bool,
    pub auto_paste: bool,
    pub retention_days: u32,
}

// ── App State ───────────────────────────────────────────────────────────────

pub struct AppState {
    pub is_recording: Arc<AtomicBool>,
    pub is_processing: AtomicBool,
    pub buffer: Arc<Mutex<Vec<f32>>>,
    pub sample_rate: Mutex<Option<u32>>,
    pub settings: Mutex<Settings>,
    pub mic_available: AtomicBool,
    pub whisper_ctx: Mutex<Option<transcribe::WhisperContextCache>>,
    pub llm_model: Mutex<Option<polisher::LlmModelCache>>,
    pub captured_context: Mutex<Option<context_detect::AppContext>>,
    pub context_override: Mutex<Option<context_detect::AppContext>>,
    pub test_mode: AtomicBool,
    pub voice_rule_mode: AtomicBool,
    pub last_hotkey_time: Mutex<Instant>,
    pub http_client: reqwest::blocking::Client,
    pub api_key_cache: Mutex<HashMap<String, String>>,
    pub edit_mode: AtomicBool,
    pub edit_selected_text: Mutex<Option<String>>,
    pub edit_text_override: Mutex<Option<String>>,
    pub saved_clipboard: Mutex<Option<String>>,
    pub pending_preview: Mutex<Option<PendingPreview>>,
    pub vad_ctx: Mutex<Option<transcribe::VadContextCache>>,
}

/// Restore original clipboard content from saved_clipboard.
fn restore_clipboard(state: &AppState) {
    if let Ok(mut saved) = state.saved_clipboard.lock() {
        if let Some(original) = saved.take() {
            std::thread::sleep(std::time::Duration::from_millis(200));
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&original);
            }
        }
    }
}

/// Hide overlay after a delay (in ms). 0 means hide immediately.
fn hide_overlay_delayed(app: &AppHandle, delay_ms: u64) {
    let app_handle = app.clone();
    std::thread::spawn(move || {
        if delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
        let app_for_hide = app_handle.clone();
        let _ = app_handle.run_on_main_thread(move || {
            if let Some(overlay) = app_for_hide.get_webview_window("overlay") {
                platform::hide_overlay(&overlay);
            }
        });
    });
}

/// Confirm a pending preview: copy to clipboard, optionally paste, save history.
pub fn do_confirm_preview(app: &AppHandle, edited_text: Option<String>) {
    let state = app.state::<AppState>();
    let preview = state.pending_preview.lock().ok().and_then(|mut p| p.take());
    let Some(preview) = preview else {
        println!("[Sumi] confirm_preview: no pending preview");
        return;
    };

    let text = edited_text.unwrap_or(preview.text);

    let clipboard_ok = match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            if let Err(e) = clipboard.set_text(&text) {
                eprintln!("[Sumi] Clipboard error: {}", e);
                false
            } else {
                true
            }
        }
        Err(e) => {
            eprintln!("[Sumi] Clipboard init error: {}", e);
            false
        }
    };

    if clipboard_ok {
        std::thread::sleep(std::time::Duration::from_millis(100));

        if preview.auto_paste {
            let pasted = platform::simulate_paste();
            if pasted {
                println!("[Sumi] 📋 Preview confirmed → auto-pasted");
                if let Some(overlay) = app.get_webview_window("overlay") {
                    let _ = overlay.emit("recording-status", "pasted");
                }
            } else {
                println!("[Sumi] 📋 Preview confirmed → copied (paste failed)");
                if let Some(overlay) = app.get_webview_window("overlay") {
                    let _ = overlay.emit("recording-status", "copied");
                }
            }
        } else {
            println!("[Sumi] 📋 Preview confirmed → copied (auto-paste disabled)");
            if let Some(overlay) = app.get_webview_window("overlay") {
                let _ = overlay.emit("recording-status", "copied");
            }
        }
    }

    if preview.is_edit {
        restore_clipboard(&state);
    }

    // Save history
    let total_elapsed_ms = preview.pipeline_start.elapsed().as_millis() as u64;
    let stt_model = match preview.stt_config_snapshot.mode {
        SttMode::Cloud => {
            format!("{} (Cloud/{})", preview.stt_config_snapshot.cloud.model_id, preview.stt_config_snapshot.cloud.provider.as_key())
        }
        SttMode::Local => preview.stt_config_snapshot.whisper_model.display_name().to_string(),
    };
    let polish_model_name = if preview.polish_config_snapshot.enabled {
        match preview.polish_config_snapshot.mode {
            polisher::PolishMode::Cloud => {
                format!("{} (Cloud/{})", preview.polish_config_snapshot.cloud.model_id, preview.polish_config_snapshot.cloud.provider.as_key())
            }
            polisher::PolishMode::Local => {
                format!("{} (Local)", preview.polish_config_snapshot.model.display_name())
            }
        }
    } else {
        "None".to_string()
    };

    if !preview.is_edit {
        let entry_id = history::generate_id();
        let has_audio = history::save_audio_wav(&audio_dir(), &entry_id, &preview.samples_16k);
        let entry = history::HistoryEntry {
            id: entry_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
            text: text.clone(),
            raw_text: preview.raw_text,
            reasoning: preview.reasoning,
            stt_model,
            polish_model: polish_model_name,
            duration_secs: preview.audio_duration_secs,
            has_audio,
            stt_elapsed_ms: preview.stt_elapsed_ms,
            polish_elapsed_ms: preview.polish_elapsed_ms,
            total_elapsed_ms,
            app_name: preview.history_context.app_name.clone(),
            bundle_id: preview.history_context.bundle_id.clone(),
            chars_per_sec: preview.chars_per_sec,
        };
        history::add_entry(&history_dir(), &audio_dir(), entry, preview.retention_days);
        println!("[Sumi] 📝 Preview → history entry saved");
    }

    hide_overlay_delayed(app, 1500);
}

/// Cancel a pending preview: discard text, hide overlay.
pub fn do_cancel_preview(app: &AppHandle) {
    let state = app.state::<AppState>();
    let preview = state.pending_preview.lock().ok().and_then(|mut p| p.take());

    if let Some(preview) = preview {
        if preview.is_edit {
            restore_clipboard(&state);
        }
        println!("[Sumi] Preview cancelled");
    }

    hide_overlay_delayed(app, 0);
}

/// Shared logic: stop recording, transcribe, copy/paste, and hide the overlay.
fn stop_transcribe_and_paste(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state
        .is_processing
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        println!("[Sumi] stop_transcribe_and_paste: already processing, skipping");
        return;
    }

    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.emit("recording-status", "transcribing");
    }
    {
        let vrm = app.state::<AppState>();
        if vrm.voice_rule_mode.load(Ordering::SeqCst) {
            if let Some(main_win) = app.get_webview_window("main") {
                let _ = main_win.emit("voice-rule-status", "transcribing");
            }
        }
    }

    println!("[Sumi] ⏹️ Stopping recording...");

    let app_handle = app.clone();
    std::thread::spawn(move || {
        let pipeline_start = Instant::now();
        let state = app_handle.state::<AppState>();

        let (auto_paste, polish_config, retention_days, mut stt_config) = state
            .settings
            .lock()
            .map(|s| (s.auto_paste, s.polish.clone(), s.history_retention_days, s.stt.clone()))
            .unwrap_or((true, polisher::PolishConfig::default(), 0, SttConfig::default()));

        if stt_config.mode == SttMode::Cloud {
            let key = get_cached_api_key(&state.api_key_cache, stt_config.cloud.provider.as_key());
            if !key.is_empty() {
                stt_config.cloud.api_key = key;
            }
        }

        let stt_language = stt_config.language.clone();
        let stt_app_name = state
            .captured_context
            .lock()
            .ok()
            .and_then(|ctx| ctx.as_ref().map(|c| c.app_name.clone()))
            .unwrap_or_default();
        let dictionary_terms: Vec<String> = polish_config
            .dictionary
            .entries
            .iter()
            .filter(|e| e.enabled && !e.term.is_empty())
            .map(|e| e.term.clone())
            .collect();

        match audio::do_stop_recording(
            &state.is_recording,
            &state.sample_rate,
            &state.buffer,
            &state.whisper_ctx,
            &state.http_client,
            &stt_config,
            &stt_language,
            &stt_app_name,
            &dictionary_terms,
            &state.vad_ctx,
            stt_config.vad_enabled,
        ) {
            Ok((text, samples_16k)) => {
                let transcribe_elapsed = pipeline_start.elapsed();
                println!("[Sumi] [timing] stop→transcribed: {:.0?} | text: {}", transcribe_elapsed, text);

                // Voice Rule Mode
                if state.voice_rule_mode.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                    println!("[Sumi] Voice rule mode: emitting transcript to main window");
                    if let Some(main_win) = app_handle.get_webview_window("main") {
                        let _ = main_win.emit("voice-rule-transcript", &text);
                    }
                    state.is_processing.store(false, Ordering::SeqCst);
                    let app_for_hide = app_handle.clone();
                    let _ = app_handle.run_on_main_thread(move || {
                        if let Some(overlay) = app_for_hide.get_webview_window("overlay") {
                            platform::hide_overlay(&overlay);
                        }
                    });
                    return;
                }

                let raw_text = text.clone();
                let audio_duration_secs = samples_16k.len() as f64 / 16000.0;

                let char_count = text.chars().count();
                let stt_secs = transcribe_elapsed.as_secs_f64();
                let chars_per_sec = if stt_secs > 0.0 {
                    char_count as f64 / stt_secs
                } else {
                    0.0
                };
                println!(
                    "[Sumi] [stats] STT output: {} chars in {:.2}s = {:.1} chars/sec",
                    char_count, stt_secs, chars_per_sec
                );

                // AI Polishing
                let mut polish_config = polish_config;
                if polish_config.enabled && polish_config.mode == polisher::PolishMode::Cloud {
                    let key = get_cached_api_key(&state.api_key_cache, polish_config.cloud.provider.as_key());
                    if !key.is_empty() {
                        polish_config.cloud.api_key = key;
                    }
                }
                let stt_elapsed_ms = transcribe_elapsed.as_millis() as u64;

                let history_context = state
                    .captured_context
                    .lock()
                    .ok()
                    .and_then(|c| c.clone())
                    .unwrap_or_default();

                let (final_text, reasoning, polish_elapsed_ms) = if polish_config.enabled {
                    let model_dir = models_dir();
                    if polisher::is_polish_ready(&model_dir, &polish_config) {
                        if let Some(overlay) = app_handle.get_webview_window("overlay") {
                            let _ = overlay.emit("recording-status", "polishing");
                        }
                        let mode_label = match polish_config.mode {
                            polisher::PolishMode::Cloud => format!("Cloud ({})", polish_config.cloud.model_id),
                            polisher::PolishMode::Local => format!("Local ({})", polish_config.model.display_name()),
                        };
                        let context = state
                            .captured_context
                            .lock()
                            .ok()
                            .and_then(|mut c| c.take())
                            .unwrap_or_default();

                        let polish_start = Instant::now();
                        let result = polisher::polish_text(
                            &state.llm_model,
                            &model_dir,
                            &polish_config,
                            &context,
                            &text,
                            &state.http_client,
                        );
                        let p_elapsed = polish_start.elapsed().as_millis() as u64;
                        println!("[Sumi] [timing] polish ({}): {:.0?} | text: {:?}", mode_label, polish_start.elapsed(), result.text);
                        (result.text, result.reasoning, Some(p_elapsed))
                    } else {
                        println!("[Sumi] Polish enabled but not ready (model missing or no API key), skipping");
                        (text, None, None)
                    }
                } else {
                    (text, None, None)
                };
                let text = final_text;

                // Preview mode: store pending preview and show in overlay
                let preview_mode = state.settings.lock()
                    .map(|s| s.preview_before_paste)
                    .unwrap_or(false);

                if preview_mode {
                    *state.pending_preview.lock().unwrap() = Some(PendingPreview {
                        text: text.clone(),
                        raw_text,
                        reasoning,
                        samples_16k,
                        audio_duration_secs,
                        stt_config_snapshot: stt_config,
                        polish_config_snapshot: polish_config,
                        stt_elapsed_ms,
                        polish_elapsed_ms,
                        pipeline_start,
                        chars_per_sec,
                        history_context,
                        is_edit: false,
                        auto_paste,
                        retention_days,
                    });
                    let hotkey_str = state.settings.lock()
                        .map(|s| s.hotkey.clone())
                        .unwrap_or_default();
                    if let Some(overlay) = app_handle.get_webview_window("overlay") {
                        let _ = overlay.emit("recording-status", "preview");
                        let _ = overlay.emit("preview-text", PreviewPayload {
                            text: text.clone(),
                            hotkey: hotkey_str,
                        });
                    }
                    if let Some(main_win) = app_handle.get_webview_window("main") {
                        let _ = main_win.emit("transcription-result", &text);
                    }
                    state.is_processing.store(false, Ordering::SeqCst);
                    println!("[Sumi] Preview mode: waiting for user confirmation");
                    return;
                }

                if let Some(main_win) = app_handle.get_webview_window("main") {
                    let _ = main_win.emit("transcription-result", &text);
                }

                let clipboard_ok = match arboard::Clipboard::new() {
                    Ok(mut clipboard) => {
                        if let Err(e) = clipboard.set_text(&text) {
                            eprintln!("[Sumi] Clipboard error: {}", e);
                            false
                        } else {
                            true
                        }
                    }
                    Err(e) => {
                        eprintln!("[Sumi] Clipboard init error: {}", e);
                        false
                    }
                };

                if clipboard_ok {
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    if auto_paste {
                        let pasted = platform::simulate_paste();
                        if pasted {
                            println!("[Sumi] 📋 Auto-pasted at cursor");
                            if let Some(overlay) = app_handle.get_webview_window("overlay") {
                                let _ = overlay.emit("recording-status", "pasted");
                            }
                        } else {
                            println!("[Sumi] 📋 Copied to clipboard (paste simulation failed)");
                            if let Some(overlay) = app_handle.get_webview_window("overlay") {
                                let _ = overlay.emit("recording-status", "copied");
                            }
                        }
                    } else {
                        println!("[Sumi] 📋 Copied to clipboard (auto-paste disabled)");
                        if let Some(overlay) = app_handle.get_webview_window("overlay") {
                            let _ = overlay.emit("recording-status", "copied");
                        }
                    }
                }

                let total_elapsed_ms = pipeline_start.elapsed().as_millis() as u64;
                println!("[Sumi] [timing] total pipeline: {:.0?}", pipeline_start.elapsed());

                // Save to history
                {
                    let entry_id = history::generate_id();
                    let stt_model = match stt_config.mode {
                        SttMode::Cloud => {
                            format!("{} (Cloud/{})", stt_config.cloud.model_id, stt_config.cloud.provider.as_key())
                        }
                        SttMode::Local => stt_config.whisper_model.display_name().to_string(),
                    };
                    let polish_model_name = if polish_config.enabled {
                        match polish_config.mode {
                            polisher::PolishMode::Cloud => {
                                format!("{} (Cloud/{})", polish_config.cloud.model_id, polish_config.cloud.provider.as_key())
                            }
                            polisher::PolishMode::Local => {
                                format!("{} (Local)", polish_config.model.display_name())
                            }
                        }
                    } else {
                        "None".to_string()
                    };
                    let has_audio = history::save_audio_wav(&audio_dir(), &entry_id, &samples_16k);
                    let entry = history::HistoryEntry {
                        id: entry_id,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as i64,
                        text: text.clone(),
                        raw_text,
                        reasoning,
                        stt_model,
                        polish_model: polish_model_name,
                        duration_secs: audio_duration_secs,
                        has_audio,
                        stt_elapsed_ms,
                        polish_elapsed_ms,
                        total_elapsed_ms,
                        app_name: history_context.app_name.clone(),
                        bundle_id: history_context.bundle_id.clone(),
                        chars_per_sec,
                    };
                    history::add_entry(&history_dir(), &audio_dir(), entry, retention_days);
                    println!("[Sumi] 📝 History entry saved (audio={})", has_audio);
                }
            }
            Err(ref e) if e == "no_speech" => {
                println!("[Sumi] No speech detected, skipping (took {:.0?})", pipeline_start.elapsed());
                if state.voice_rule_mode.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                    if let Some(main_win) = app_handle.get_webview_window("main") {
                        let _ = main_win.emit("voice-rule-transcript", "");
                    }
                }
            }
            Err(e) => {
                eprintln!("[Sumi] Transcription error: {} (after {:.0?})", e, pipeline_start.elapsed());
                if let Some(overlay) = app_handle.get_webview_window("overlay") {
                    let _ = overlay.emit("recording-status", "error");
                }
                state.voice_rule_mode.store(false, Ordering::SeqCst);
            }
        }

        state.is_processing.store(false, Ordering::SeqCst);

        std::thread::sleep(std::time::Duration::from_millis(1500));
        let app_for_hide = app_handle.clone();
        let _ = app_handle.run_on_main_thread(move || {
            if let Some(overlay) = app_for_hide.get_webview_window("overlay") {
                platform::hide_overlay(&overlay);
            }
        });
    });
}

/// Edit-by-voice pipeline: stop recording, transcribe instruction, edit text, replace.
fn stop_edit_and_replace(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state
        .is_processing
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        println!("[Sumi] stop_edit_and_replace: already processing, skipping");
        return;
    }

    state.edit_mode.store(false, Ordering::SeqCst);

    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.emit("recording-status", "transcribing");
    }

    println!("[Sumi] ⏹️ Stopping edit-by-voice recording...");

    let app_handle = app.clone();
    std::thread::spawn(move || {
        let pipeline_start = Instant::now();
        let state = app_handle.state::<AppState>();

        let (auto_paste, polish_config, retention_days, mut stt_config, preview_mode) = state
            .settings
            .lock()
            .map(|s| (s.auto_paste, s.polish.clone(), s.history_retention_days, s.stt.clone(), s.preview_before_paste))
            .unwrap_or((true, polisher::PolishConfig::default(), 0, SttConfig::default(), false));

        if stt_config.mode == SttMode::Cloud {
            let key = get_cached_api_key(&state.api_key_cache, stt_config.cloud.provider.as_key());
            if !key.is_empty() {
                stt_config.cloud.api_key = key;
            }
        }

        let selected_text = state
            .edit_selected_text
            .lock()
            .ok()
            .and_then(|mut t| t.take())
            .unwrap_or_default();

        if selected_text.is_empty() {
            eprintln!("[Sumi] Edit-by-voice: no selected text");
            if let Some(overlay) = app_handle.get_webview_window("overlay") {
                let _ = overlay.emit("recording-status", "error");
            }
            state.is_processing.store(false, Ordering::SeqCst);
            restore_clipboard(&state);
            hide_overlay_delayed(&app_handle, 1500);
            return;
        }

        let edit_stt_language = stt_config.language.clone();
        let edit_app_name = state
            .captured_context
            .lock()
            .ok()
            .and_then(|ctx| ctx.as_ref().map(|c| c.app_name.clone()))
            .unwrap_or_default();
        let edit_dict_terms: Vec<String> = polish_config
            .dictionary
            .entries
            .iter()
            .filter(|e| e.enabled && !e.term.is_empty())
            .map(|e| e.term.clone())
            .collect();

        match audio::do_stop_recording(
            &state.is_recording,
            &state.sample_rate,
            &state.buffer,
            &state.whisper_ctx,
            &state.http_client,
            &stt_config,
            &edit_stt_language,
            &edit_app_name,
            &edit_dict_terms,
            &state.vad_ctx,
            stt_config.vad_enabled,
        ) {
            Ok((instruction, _samples)) => {
                println!("[Sumi] Edit instruction: {:?}", instruction);

                if let Some(overlay) = app_handle.get_webview_window("overlay") {
                    let _ = overlay.emit("recording-status", "polishing");
                }

                let mut polish_config = polish_config;
                if polish_config.mode == polisher::PolishMode::Cloud {
                    let key = get_cached_api_key(
                        &state.api_key_cache,
                        polish_config.cloud.provider.as_key(),
                    );
                    if !key.is_empty() {
                        polish_config.cloud.api_key = key;
                    }
                }

                let model_dir = models_dir();
                if !polisher::is_polish_ready(&model_dir, &polish_config) {
                    eprintln!("[Sumi] Edit-by-voice: LLM not configured");
                    if let Some(overlay) = app_handle.get_webview_window("overlay") {
                        let _ = overlay.emit("recording-status", "error");
                    }
                    state.is_processing.store(false, Ordering::SeqCst);
                    restore_clipboard(&state);
                    hide_overlay_delayed(&app_handle, 1500);
                    return;
                }

                match polisher::edit_text_by_instruction(
                    &state.llm_model,
                    &model_dir,
                    &polish_config,
                    &selected_text,
                    &instruction,
                    &state.http_client,
                ) {
                    Ok(edited_text) => {
                        println!(
                            "[Sumi] Edit result: {:?} (took {:.0?})",
                            edited_text,
                            pipeline_start.elapsed()
                        );

                        let history_context = state
                            .captured_context
                            .lock()
                            .ok()
                            .and_then(|c| c.clone())
                            .unwrap_or_default();

                        if preview_mode {
                            *state.pending_preview.lock().unwrap() = Some(PendingPreview {
                                text: edited_text.clone(),
                                raw_text: selected_text.clone(),
                                reasoning: None,
                                samples_16k: vec![],
                                audio_duration_secs: 0.0,
                                stt_config_snapshot: stt_config.clone(),
                                polish_config_snapshot: polish_config.clone(),
                                stt_elapsed_ms: 0,
                                polish_elapsed_ms: Some(pipeline_start.elapsed().as_millis() as u64),
                                pipeline_start,
                                chars_per_sec: 0.0,
                                history_context,
                                is_edit: true,
                                auto_paste,
                                retention_days,
                            });
                            let hotkey_str = state.settings.lock()
                                .map(|s| s.hotkey.clone())
                                .unwrap_or_default();
                            if let Some(overlay) = app_handle.get_webview_window("overlay") {
                                let _ = overlay.emit("recording-status", "preview");
                                let _ = overlay.emit("preview-text", PreviewPayload {
                                    text: edited_text.clone(),
                                    hotkey: hotkey_str,
                                });
                            }
                            state.is_processing.store(false, Ordering::SeqCst);
                            println!("[Sumi] Preview mode (edit): waiting for user confirmation");
                            return;
                        }

                        let clipboard_ok = match arboard::Clipboard::new() {
                            Ok(mut clipboard) => clipboard.set_text(&edited_text).is_ok(),
                            Err(_) => false,
                        };

                        if clipboard_ok {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            platform::simulate_paste();
                            println!("[Sumi] ✏️ Edited text pasted");
                        }

                        restore_clipboard(&state);

                        if let Some(overlay) = app_handle.get_webview_window("overlay") {
                            let _ = overlay.emit("recording-status", "edited");
                        }

                        state.is_processing.store(false, Ordering::SeqCst);
                        hide_overlay_delayed(&app_handle, 5500);
                    }
                    Err(e) => {
                        eprintln!("[Sumi] Edit-by-voice LLM error: {}", e);
                        if let Some(overlay) = app_handle.get_webview_window("overlay") {
                            let _ = overlay.emit("recording-status", "error");
                        }
                        state.is_processing.store(false, Ordering::SeqCst);
                        restore_clipboard(&state);
                        hide_overlay_delayed(&app_handle, 1500);
                    }
                }
            }
            Err(ref e) if e == "no_speech" => {
                println!("[Sumi] Edit-by-voice: no speech detected");
                state.is_processing.store(false, Ordering::SeqCst);
                restore_clipboard(&state);
                hide_overlay_delayed(&app_handle, 0);
            }
            Err(e) => {
                eprintln!("[Sumi] Edit-by-voice transcription error: {}", e);
                if let Some(overlay) = app_handle.get_webview_window("overlay") {
                    let _ = overlay.emit("recording-status", "error");
                }
                state.is_processing.store(false, Ordering::SeqCst);
                restore_clipboard(&state);
                hide_overlay_delayed(&app_handle, 1500);
            }
        }
    });
}

// ── App Entry ───────────────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::cancel_recording,
            commands::set_test_mode,
            commands::set_voice_rule_mode,
            commands::set_context_override,
            commands::set_edit_text_override,
            commands::get_settings,
            commands::save_settings,
            commands::update_hotkey,
            commands::reset_settings,
            commands::get_default_prompt,
            commands::get_default_prompt_rules,
            commands::test_polish,
            commands::get_mic_status,
            commands::check_model_status,
            commands::download_model,
            commands::check_llm_model_status,
            commands::download_llm_model,
            commands::save_api_key,
            commands::get_api_key,
            commands::get_history,
            commands::get_history_page,
            commands::delete_history_entry,
            commands::clear_all_history,
            commands::export_history_audio,
            commands::get_history_storage_path,
            commands::get_app_icon,
            permissions::check_permissions,
            permissions::open_permission_settings,
            commands::generate_rule_from_description,
            commands::update_edit_hotkey,
            commands::trigger_undo,
            commands::list_polish_models,
            commands::switch_polish_model,
            commands::download_polish_model,
            commands::list_whisper_models,
            commands::get_system_info,
            commands::get_whisper_model_recommendation,
            commands::switch_whisper_model,
            commands::download_whisper_model,
            commands::confirm_preview,
            commands::cancel_preview,
            commands::check_vad_model_status,
            commands::download_vad_model,
        ])
        .setup(|app| {
            // Hide Dock icon (macOS) / equivalent
            platform::set_app_accessory_mode();

            // Load settings
            let settings = load_settings();
            let hotkey_str = settings.hotkey.clone();

            // Migrate legacy JSON history to SQLite
            history::migrate_from_json(&history_dir(), &audio_dir());

            // Pre-initialise audio pipeline
            let is_recording = Arc::new(AtomicBool::new(false));
            let buffer = Arc::new(Mutex::new(Vec::new()));
            let (mic_available, sample_rate) =
                match audio::spawn_audio_thread(Arc::clone(&buffer), Arc::clone(&is_recording)) {
                    Ok(sr) => (true, Some(sr)),
                    Err(e) => {
                        eprintln!("[Sumi] Audio init failed: {}", e);
                        (false, None)
                    }
                };

            let http_client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("Failed to create shared HTTP client");

            app.manage(AppState {
                is_recording,
                is_processing: AtomicBool::new(false),
                buffer,
                sample_rate: Mutex::new(sample_rate),
                settings: Mutex::new(settings.clone()),
                mic_available: AtomicBool::new(mic_available),
                whisper_ctx: Mutex::new(None),
                llm_model: Mutex::new(None),
                captured_context: Mutex::new(None),
                context_override: Mutex::new(None),
                test_mode: AtomicBool::new(false),
                voice_rule_mode: AtomicBool::new(false),
                last_hotkey_time: Mutex::new(Instant::now() - std::time::Duration::from_secs(1)),
                http_client,
                api_key_cache: Mutex::new(HashMap::new()),
                edit_mode: AtomicBool::new(false),
                edit_selected_text: Mutex::new(None),
                edit_text_override: Mutex::new(None),
                saved_clipboard: Mutex::new(None),
                pending_preview: Mutex::new(None),
                vad_ctx: Mutex::new(None),
            });

            // Migration: if old zh-TW model exists but settings use default (LargeV3Turbo)
            // and the LargeV3Turbo model file doesn't exist, switch to LargeV3TurboZhTw
            {
                let state = app.state::<AppState>();
                let mut settings_guard = state.settings.lock().unwrap();
                if settings_guard.stt.whisper_model == whisper_models::WhisperModel::LargeV3Turbo {
                    let default_path = models_dir().join(whisper_models::WhisperModel::LargeV3Turbo.filename());
                    let legacy_path = models_dir().join("ggml-large-v3-turbo-zh-TW.bin");
                    if !default_path.exists() && legacy_path.exists() {
                        println!("[Sumi] Migrating whisper model setting: LargeV3Turbo → LargeV3TurboZhTw (legacy file exists)");
                        settings_guard.stt.whisper_model = whisper_models::WhisperModel::LargeV3TurboZhTw;
                        settings::save_settings_to_disk(&settings_guard);
                    }
                }
            }

            // Auto-show settings when active whisper model is missing
            {
                let active_model = app.state::<AppState>().settings.lock()
                    .map(|s| s.stt.whisper_model.clone())
                    .unwrap_or_default();
                if !models_dir().join(active_model.filename()).exists() {
                    show_settings_window(app.handle());
                }
            }

            // Pre-warm models in background
            {
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    let warmup_start = Instant::now();
                    let state = app_handle.state::<AppState>();

                    let (stt_mode, whisper_model) = state.settings.lock()
                        .map(|s| (s.stt.mode.clone(), s.stt.whisper_model.clone()))
                        .unwrap_or_default();
                    if stt_mode == SttMode::Local {
                        if let Ok(model_path) = transcribe::whisper_model_path_for(&whisper_model) {
                            let mut ctx_guard = state.whisper_ctx.lock().unwrap();
                            if ctx_guard.is_none() {
                                println!("[Sumi] Pre-warming Whisper model: {}...", whisper_model.display_name());
                                unsafe extern "C" fn noop_log(
                                    _level: u32,
                                    _text: *const std::ffi::c_char,
                                    _user_data: *mut std::ffi::c_void,
                                ) {}
                                unsafe {
                                    whisper_rs::set_log_callback(Some(noop_log), std::ptr::null_mut());
                                }
                                let mut ctx_params = WhisperContextParameters::new();
                                ctx_params.use_gpu(true);
                                match whisper_rs::WhisperContext::new_with_params(
                                    model_path.to_str().unwrap_or_default(),
                                    ctx_params,
                                ) {
                                    Ok(ctx) => {
                                        *ctx_guard = Some(transcribe::WhisperContextCache {
                                            ctx,
                                            loaded_path: model_path.clone(),
                                        });
                                        println!("[Sumi] Whisper model pre-warmed ({:.0?})", warmup_start.elapsed());
                                    }
                                    Err(e) => {
                                        eprintln!("[Sumi] Whisper pre-warm failed: {}", e);
                                    }
                                }
                            }
                        }
                    }

                    let polish_config = state.settings.lock()
                        .map(|s| s.polish.clone())
                        .unwrap_or_default();
                    if polish_config.enabled && polish_config.mode == polisher::PolishMode::Local {
                        let model_dir = models_dir();
                        if polisher::model_file_exists(&model_dir, &polish_config.model) {
                            let llm_start = Instant::now();
                            println!("[Sumi] Pre-warming LLM ({})...", polish_config.model.display_name());
                            polisher::ensure_model_loaded(&state.llm_model, &model_dir, &polish_config);
                            println!("[Sumi] LLM pre-warmed ({:.0?})", llm_start.elapsed());
                        }
                    }

                    println!("[Sumi] All models pre-warmed ({:.0?} total)", warmup_start.elapsed());
                });
            }

            // System Tray
            let settings_i =
                MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
            let quit_i =
                MenuItem::with_id(app, "quit", "Quit Sumi", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

            let tooltip_label = hotkey_display_label(&hotkey_str);
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png")).unwrap())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip(format!("Sumi – {} to record", tooltip_label))
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => {
                        show_settings_window(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        show_settings_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // Window close → hide
            if let Some(main_window) = app.get_webview_window("main") {
                let win = main_window.clone();
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });
            }

            // Configure overlay
            if let Some(overlay) = app.get_webview_window("overlay") {
                platform::setup_overlay_window(&overlay);
            }

            // Global Shortcut
            #[cfg(desktop)]
            {
                let primary_shortcut = parse_hotkey_string(&hotkey_str)
                    .unwrap_or(Shortcut::new(Some(Modifiers::ALT | Modifiers::SUPER), Code::KeyR));
                let edit_shortcut = settings.edit_hotkey.as_deref().and_then(parse_hotkey_string);
                let edit_shortcut_clone = edit_shortcut;

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, shortcut, event| {
                            if event.state() != ShortcutState::Pressed {
                                return;
                            }

                            let state = app.state::<AppState>();

                            let is_edit_hotkey = edit_shortcut_clone
                                .is_some_and(|es| *shortcut == es);

                            if state.test_mode.load(Ordering::SeqCst) {
                                if let Some(main_win) = app.get_webview_window("main") {
                                    let _ = main_win.emit("hotkey-activated", true);
                                }
                                return;
                            }

                            // Debounce
                            {
                                let now = Instant::now();
                                if let Ok(mut last) = state.last_hotkey_time.lock() {
                                    if now.duration_since(*last) < std::time::Duration::from_millis(300) {
                                        return;
                                    }
                                    *last = now;
                                }
                            }

                            if state.is_processing.load(Ordering::SeqCst) {
                                return;
                            }

                            // If there's a pending preview, hotkey confirms it
                            if !is_edit_hotkey {
                                if state.pending_preview.lock().ok().map(|p| p.is_some()).unwrap_or(false) {
                                    do_confirm_preview(app, None);
                                    return;
                                }
                            }

                            let is_recording = state.is_recording.load(Ordering::SeqCst);

                            if !is_recording {
                                // Start Recording

                                // For edit hotkey: check polish readiness before anything else
                                if is_edit_hotkey {
                                    let mut polish_config = state.settings.lock()
                                        .map(|s| s.polish.clone())
                                        .unwrap_or_default();
                                    if polish_config.mode == polisher::PolishMode::Cloud {
                                        let key = get_cached_api_key(
                                            &state.api_key_cache,
                                            polish_config.cloud.provider.as_key(),
                                        );
                                        if !key.is_empty() {
                                            polish_config.cloud.api_key = key;
                                        }
                                    }
                                    let model_dir = models_dir();
                                    if !polish_config.enabled || !polisher::is_polish_ready(&model_dir, &polish_config) {
                                        println!("[Sumi] Edit-by-voice: polish not ready, showing overlay hint");
                                        if let Some(overlay) = app.get_webview_window("overlay") {
                                            let _ = overlay.emit("recording-status", "edit_requires_polish");
                                            if let Ok(Some(monitor)) = overlay.current_monitor() {
                                                let screen = monitor.size();
                                                let scale = monitor.scale_factor();
                                                let win_w = 300.0;
                                                let win_h = 52.0;
                                                let x = (screen.width as f64 / scale - win_w) / 2.0;
                                                let y = screen.height as f64 / scale - win_h - 80.0;
                                                let _ = overlay.set_position(
                                                    tauri::PhysicalPosition::new(
                                                        (x * scale) as i32,
                                                        (y * scale) as i32,
                                                    ),
                                                );
                                            }
                                            platform::show_overlay(&overlay);
                                        }
                                        hide_overlay_delayed(app, 2000);
                                        return;
                                    }

                                    let override_text = state.edit_text_override.lock()
                                        .ok()
                                        .and_then(|mut ov| ov.take());

                                    if let Some(text) = override_text {
                                        if text.is_empty() {
                                            println!("[Sumi] Edit-by-voice: override text is empty, aborting");
                                            return;
                                        }
                                        if let Ok(mut et) = state.edit_selected_text.lock() {
                                            *et = Some(text.clone());
                                        }
                                        state.edit_mode.store(true, Ordering::SeqCst);
                                        println!("[Sumi] ✏️ Edit-by-voice (override): captured {} chars", text.len());
                                    } else {
                                        let original_clipboard = arboard::Clipboard::new()
                                            .ok()
                                            .and_then(|mut cb| cb.get_text().ok());

                                        if let Ok(mut saved) = state.saved_clipboard.lock() {
                                            *saved = original_clipboard;
                                        }

                                        platform::simulate_copy();
                                        std::thread::sleep(std::time::Duration::from_millis(100));

                                        let selected = arboard::Clipboard::new()
                                            .ok()
                                            .and_then(|mut cb| cb.get_text().ok())
                                            .unwrap_or_default();

                                        let saved_text = state.saved_clipboard.lock()
                                            .ok()
                                            .and_then(|s| s.clone())
                                            .unwrap_or_default();

                                        if selected.is_empty() || selected == saved_text {
                                            println!("[Sumi] Edit-by-voice: no text selected, aborting");
                                            restore_clipboard(&state);
                                            return;
                                        }

                                        if let Ok(mut et) = state.edit_selected_text.lock() {
                                            *et = Some(selected.clone());
                                        }
                                        state.edit_mode.store(true, Ordering::SeqCst);
                                        println!("[Sumi] ✏️ Edit-by-voice: captured {} chars", selected.len());
                                    }
                                }

                                let captured_ctx = state.context_override.lock()
                                    .ok()
                                    .and_then(|ctx| ctx.clone())
                                    .unwrap_or_else(context_detect::detect_frontmost_app);

                                match audio::do_start_recording(
                                    &state.is_recording,
                                    &state.mic_available,
                                    &state.sample_rate,
                                    &state.buffer,
                                    &state.is_recording,
                                ) {
                                    Ok(()) => {
                                        println!("[Sumi] 🎙️ Recording started (app: {:?}, bundle: {:?}, url: {:?})",
                                            captured_ctx.app_name, captured_ctx.bundle_id, captured_ctx.url);

                                        if let Ok(mut ctx) = state.captured_context.lock() {
                                            *ctx = Some(captured_ctx);
                                        }

                                        if let Some(main_win) = app.get_webview_window("main") {
                                            let _ = main_win.emit("hotkey-activated", true);
                                            if state.voice_rule_mode.load(Ordering::SeqCst) {
                                                let _ = main_win.emit("voice-rule-status", "recording");
                                            }
                                        }

                                        if let Some(overlay) = app.get_webview_window("overlay") {
                                            let _ = overlay.emit("recording-status", "recording");
                                            let _ = overlay.emit("recording-max-duration", MAX_RECORDING_SECS);
                                            if let Ok(Some(monitor)) = overlay.current_monitor() {
                                                let screen = monitor.size();
                                                let scale = monitor.scale_factor();
                                                let win_w = 300.0;
                                                let win_h = 52.0;
                                                let x = (screen.width as f64 / scale - win_w) / 2.0;
                                                let y = screen.height as f64 / scale - win_h - 80.0;
                                                let _ = overlay.set_position(
                                                    tauri::PhysicalPosition::new(
                                                        (x * scale) as i32,
                                                        (y * scale) as i32,
                                                    ),
                                                );
                                            }
                                            platform::show_overlay(&overlay);
                                        }

                                        // Audio level monitoring thread
                                        let app_for_monitor = app.clone();
                                        std::thread::spawn(move || {
                                            let state = app_for_monitor.state::<AppState>();
                                            let sr = state.sample_rate.lock().ok().and_then(|v| *v).unwrap_or(44100) as usize;
                                            let recording_start = Instant::now();

                                            const NUM_BARS: usize = 20;
                                            let samples_per_bar = sr / 20;

                                            while state.is_recording.load(Ordering::SeqCst) {
                                                if recording_start.elapsed().as_secs() >= MAX_RECORDING_SECS {
                                                    println!("[Sumi] ⏱️ Max recording duration reached ({}s)", MAX_RECORDING_SECS);
                                                    if state.edit_mode.load(Ordering::SeqCst) {
                                                        stop_edit_and_replace(&app_for_monitor);
                                                    } else {
                                                        stop_transcribe_and_paste(&app_for_monitor);
                                                    }
                                                    return;
                                                }
                                                let levels: Vec<f32> = if let Ok(buf) = state.buffer.lock() {
                                                    if buf.is_empty() {
                                                        vec![0.0; NUM_BARS]
                                                    } else {
                                                        let total = NUM_BARS * samples_per_bar;
                                                        let start = buf.len().saturating_sub(total);
                                                        let mut bars: Vec<f32> = buf[start..]
                                                            .chunks(samples_per_bar)
                                                            .map(|chunk| {
                                                                let rms = (chunk.iter().map(|&s| s * s).sum::<f32>()
                                                                    / chunk.len() as f32)
                                                                    .sqrt();
                                                                (rms * 6.0).min(1.0)
                                                            })
                                                            .collect();
                                                        while bars.len() < NUM_BARS {
                                                            bars.insert(0, 0.0);
                                                        }
                                                        bars
                                                    }
                                                } else {
                                                    vec![0.0; NUM_BARS]
                                                };

                                                if let Some(ov) = app_for_monitor.get_webview_window("overlay") {
                                                    let _ = ov.emit("audio-levels", &levels);
                                                }
                                                if state.voice_rule_mode.load(Ordering::SeqCst) {
                                                    if let Some(main_win) = app_for_monitor.get_webview_window("main") {
                                                        let _ = main_win.emit("voice-rule-levels", &levels);
                                                    }
                                                }
                                                std::thread::sleep(std::time::Duration::from_millis(50));
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        eprintln!("[Sumi] Failed to start recording: {}", e);
                                        if is_edit_hotkey {
                                            state.edit_mode.store(false, Ordering::SeqCst);
                                            restore_clipboard(&state);
                                        }
                                        if let Some(overlay) = app.get_webview_window("overlay") {
                                            platform::hide_overlay(&overlay);
                                        }
                                    }
                                }
                            } else {
                                // Stop Recording
                                if state.edit_mode.load(Ordering::SeqCst) {
                                    stop_edit_and_replace(app);
                                } else {
                                    stop_transcribe_and_paste(app);
                                }
                            }
                        })
                        .build(),
                )?;

                app.global_shortcut().register(primary_shortcut)?;
                let label = hotkey_display_label(&hotkey_str);
                println!("[Sumi] {} global shortcut registered", label);

                if let Some(edit_sc) = edit_shortcut {
                    app.global_shortcut().register(edit_sc)?;
                    if let Some(ref edit_hk) = settings.edit_hotkey {
                        println!("[Sumi] {} edit shortcut registered", hotkey_display_label(edit_hk));
                    }
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn show_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_size(tauri::LogicalSize::new(960.0, 720.0));
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
