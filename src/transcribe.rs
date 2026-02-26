use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use whisper_rs::{WhisperContext, WhisperContextParameters};

use crate::settings::models_dir;

/// Resolve the path to the whisper GGML model.
/// Returns an error if the model hasn't been downloaded yet.
pub fn whisper_model_path() -> Result<PathBuf, String> {
    let model_path = models_dir().join("ggml-large-v3-turbo-zh-TW.bin");
    if model_path.exists() {
        Ok(model_path)
    } else {
        Err("Whisper model not downloaded. Please download it from Settings.".to_string())
    }
}

/// Transcribe 16 kHz mono f32 samples using the cached WhisperContext.
/// The context is lazily loaded on first use and reused across transcriptions.
pub fn transcribe_with_cached_whisper(
    whisper_ctx: &Mutex<Option<WhisperContext>>,
    samples_16k: &[f32],
) -> Result<String, String> {
    use whisper_rs::{FullParams, SamplingStrategy};

    // Suppress verbose C-level logs from whisper.cpp / ggml
    unsafe extern "C" fn noop_log(
        _level: u32,
        _text: *const std::ffi::c_char,
        _user_data: *mut std::ffi::c_void,
    ) {
    }
    unsafe {
        whisper_rs::set_log_callback(Some(noop_log), std::ptr::null_mut());
    }

    let mut ctx_guard = whisper_ctx
        .lock()
        .map_err(|e| format!("Failed to lock whisper context: {}", e))?;

    if ctx_guard.is_none() {
        let model_path = whisper_model_path()?;
        let load_start = Instant::now();
        println!("[Sumi] Loading Whisper model (first use)...");
        let mut ctx_params = WhisperContextParameters::new();
        ctx_params.use_gpu(true);
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or("Invalid model path")?,
            ctx_params,
        )
        .map_err(|e| format!("Failed to load whisper model: {}", e))?;
        *ctx_guard = Some(ctx);
        println!("[Sumi] Whisper model loaded with GPU enabled (took {:.0?})", load_start.elapsed());
    }

    let ctx = ctx_guard.as_ref().unwrap();

    let state_start = Instant::now();
    let mut wh_state = ctx
        .create_state()
        .map_err(|e| format!("Failed to create whisper state: {}", e))?;
    println!("[Sumi] Whisper state created: {:.0?}", state_start.elapsed());

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(None);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_print_progress(false);
    params.set_single_segment(true);
    params.set_no_timestamps(true);
    params.set_no_context(true);
    params.set_temperature_inc(-1.0);
    params.set_n_threads(num_cpus() as _);

    let infer_start = Instant::now();
    wh_state
        .full(params, samples_16k)
        .map_err(|e| format!("Whisper inference failed: {}", e))?;
    println!("[Sumi] Whisper wh_state.full() done: {:.0?}", infer_start.elapsed());

    let num_segments = wh_state.full_n_segments();

    let mut text = String::new();
    for i in 0..num_segments {
        if let Some(seg) = wh_state.get_segment(i) {
            if let Ok(s) = seg.to_str_lossy() {
                text.push_str(&s);
            }
        }
    }

    Ok(text.trim().to_string())
}

/// Return the number of available CPU cores.
pub fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
