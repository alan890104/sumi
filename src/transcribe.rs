use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use whisper_rs::{WhisperContext, WhisperContextParameters};

use crate::settings::models_dir;
use crate::whisper_models::WhisperModel;

/// Cached whisper context that tracks which model file is loaded.
/// When the requested model path differs from the loaded one, the context
/// is automatically reloaded.
pub struct WhisperContextCache {
    pub ctx: WhisperContext,
    pub loaded_path: PathBuf,
}

// WhisperContext is Send but not Sync by default; we guard it with a Mutex.
unsafe impl Send for WhisperContextCache {}

/// Resolve the path to a whisper GGML model file.
/// Returns an error if the model hasn't been downloaded yet.
pub fn whisper_model_path_for(model: &WhisperModel) -> Result<PathBuf, String> {
    let model_path = models_dir().join(model.filename());
    if model_path.exists() {
        Ok(model_path)
    } else {
        Err(format!(
            "Whisper model '{}' not downloaded. Please download it from Settings.",
            model.display_name()
        ))
    }
}

/// Transcribe 16 kHz mono f32 samples using the cached WhisperContext.
/// The context is lazily loaded on first use, and automatically reloaded
/// when the requested model differs from the currently loaded one.
pub fn transcribe_with_cached_whisper(
    whisper_cache: &Mutex<Option<WhisperContextCache>>,
    samples_16k: &[f32],
    model: &WhisperModel,
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

    let model_path = whisper_model_path_for(model)?;

    let mut cache_guard = whisper_cache
        .lock()
        .map_err(|e| format!("Failed to lock whisper context: {}", e))?;

    // Check if we need to (re)load the model
    let needs_reload = match cache_guard.as_ref() {
        Some(c) => c.loaded_path != model_path,
        None => true,
    };

    if needs_reload {
        let load_start = Instant::now();
        println!(
            "[Sumi] Loading Whisper model: {} ...",
            model.display_name()
        );
        let mut ctx_params = WhisperContextParameters::new();
        ctx_params.use_gpu(true);
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or("Invalid model path")?,
            ctx_params,
        )
        .map_err(|e| format!("Failed to load whisper model: {}", e))?;

        *cache_guard = Some(WhisperContextCache {
            ctx,
            loaded_path: model_path.clone(),
        });
        println!(
            "[Sumi] Whisper model loaded with GPU enabled (took {:.0?})",
            load_start.elapsed()
        );
    }

    let cache = cache_guard.as_ref().unwrap();

    let state_start = Instant::now();
    let mut wh_state = cache
        .ctx
        .create_state()
        .map_err(|e| format!("Failed to create whisper state: {}", e))?;
    println!(
        "[Sumi] Whisper state created: {:.0?}",
        state_start.elapsed()
    );

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
    println!(
        "[Sumi] Whisper wh_state.full() done: {:.0?}",
        infer_start.elapsed()
    );

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
