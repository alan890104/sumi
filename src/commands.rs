use crate::audio;
use crate::credentials;
use crate::hotkey::{hotkey_display_label, parse_hotkey_string};
use crate::platform;
use crate::polisher::{self, PolishModelInfo};
use crate::settings::{self, Settings};
use crate::stt::SttMode;
use crate::whisper_models::{self, WhisperModel, WhisperModelInfo, SystemInfo};
use crate::{history, AppState};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};

/// Load an API key, checking the in-memory cache first before falling back
/// to the credential store.
pub fn get_cached_api_key(cache: &Mutex<HashMap<String, String>>, provider: &str) -> String {
    if let Ok(map) = cache.lock() {
        if let Some(key) = map.get(provider) {
            return key.clone();
        }
    }
    match credentials::load(provider) {
        Ok(key) => {
            if let Ok(mut map) = cache.lock() {
                map.insert(provider.to_string(), key.clone());
            }
            key
        }
        Err(_) => String::new(),
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    new_settings: Settings,
) -> Result<(), String> {
    let mut current = state.settings.lock().map_err(|e| e.to_string())?;
    current.auto_paste = new_settings.auto_paste;
    current.polish = new_settings.polish;
    current.history_retention_days = new_settings.history_retention_days;
    current.stt = new_settings.stt;
    // Keep cloud.language in sync with top-level language
    current.stt.cloud.language = current.stt.language.clone();
    current.edit_hotkey = new_settings.edit_hotkey;
    current.onboarding_completed = new_settings.onboarding_completed;
    settings::save_settings_to_disk(&current);
    Ok(())
}

#[tauri::command]
pub fn update_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    new_hotkey: String,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let shortcut =
        parse_hotkey_string(&new_hotkey).ok_or_else(|| "Invalid hotkey string".to_string())?;

    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| format!("Failed to register shortcut: {}", e))?;

    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.hotkey = new_hotkey.clone();
    settings::save_settings_to_disk(&settings);

    if let Some(ref edit_hk) = settings.edit_hotkey {
        if let Some(edit_shortcut) = parse_hotkey_string(edit_hk) {
            let _ = app.global_shortcut().register(edit_shortcut);
        }
    }

    let label = hotkey_display_label(&new_hotkey);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(&format!("Sumi – {} to record", label)));
    }

    println!(
        "[Sumi] Hotkey updated to: {} ({})",
        new_hotkey, label
    );
    Ok(())
}

#[tauri::command]
pub fn update_edit_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    new_edit_hotkey: Option<String>,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;

    if let Some(ref hk) = new_edit_hotkey {
        if !hk.is_empty() {
            let _ = parse_hotkey_string(hk)
                .ok_or_else(|| "Invalid edit hotkey string".to_string())?;
        }
    }
    settings.edit_hotkey = new_edit_hotkey.filter(|s| !s.is_empty());

    let primary = parse_hotkey_string(&settings.hotkey)
        .ok_or_else(|| "Invalid primary hotkey".to_string())?;
    app.global_shortcut()
        .register(primary)
        .map_err(|e| format!("Failed to register primary shortcut: {}", e))?;

    if let Some(ref edit_hk) = settings.edit_hotkey {
        if let Some(shortcut) = parse_hotkey_string(edit_hk) {
            app.global_shortcut()
                .register(shortcut)
                .map_err(|e| format!("Failed to register edit shortcut: {}", e))?;
            println!("[Sumi] Edit hotkey registered: {}", edit_hk);
        }
    }

    settings::save_settings_to_disk(&settings);
    println!("[Sumi] Edit hotkey updated to: {:?}", settings.edit_hotkey);
    Ok(())
}

#[tauri::command]
pub fn trigger_undo(app: AppHandle) -> Result<(), String> {
    let app_handle = app.clone();
    std::thread::spawn(move || {
        platform::simulate_undo();
        println!("[Sumi] ↩️ Undo triggered from overlay");
        let app_for_hide = app_handle.clone();
        let _ = app_handle.run_on_main_thread(move || {
            if let Some(overlay) = app_for_hide.get_webview_window("overlay") {
                platform::hide_overlay(&overlay);
            }
        });
    });
    Ok(())
}

#[tauri::command]
pub fn reset_settings(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let defaults = Settings::default();
    let default_hotkey = defaults.hotkey.clone();

    {
        let mut current = state.settings.lock().map_err(|e| e.to_string())?;
        *current = defaults;
    }

    settings::save_settings_to_disk(&Settings::default());

    let shortcut = parse_hotkey_string(&default_hotkey)
        .ok_or_else(|| "Invalid default hotkey string".to_string())?;

    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| format!("Failed to register shortcut: {}", e))?;

    let label = hotkey_display_label(&default_hotkey);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(&format!("Sumi – {} to record", label)));
    }

    println!("[Sumi] Settings reset to defaults (hotkey: {})", label);
    Ok(())
}

#[tauri::command]
pub fn get_default_prompt() -> String {
    polisher::base_prompt_template()
}

#[tauri::command]
pub fn get_default_prompt_rules() -> Vec<polisher::PromptRule> {
    polisher::default_prompt_rules()
}

#[tauri::command]
pub fn save_api_key(state: State<'_, AppState>, provider: String, key: String) -> Result<(), String> {
    if key.is_empty() {
        credentials::delete(&provider)?;
        if let Ok(mut map) = state.api_key_cache.lock() {
            map.remove(&provider);
        }
    } else {
        credentials::save(&provider, &key)?;
        if let Ok(mut map) = state.api_key_cache.lock() {
            map.insert(provider, key);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_api_key(state: State<'_, AppState>, provider: String) -> Result<String, String> {
    Ok(get_cached_api_key(&state.api_key_cache, &provider))
}

#[derive(Serialize)]
pub struct HistoryPage {
    pub entries: Vec<history::HistoryEntry>,
    pub has_more: bool,
}

#[tauri::command]
pub fn get_history_page(before_timestamp: Option<i64>, limit: Option<u32>) -> HistoryPage {
    let limit = limit.unwrap_or(10);
    let (entries, has_more) =
        history::load_history_page(&settings::history_dir(), before_timestamp, limit);
    HistoryPage { entries, has_more }
}

#[tauri::command]
pub fn get_history() -> Vec<history::HistoryEntry> {
    history::load_history(&settings::history_dir())
}

#[tauri::command]
pub fn delete_history_entry(id: String) -> Result<(), String> {
    history::delete_entry(&settings::history_dir(), &settings::audio_dir(), &id);
    Ok(())
}

#[tauri::command]
pub fn export_history_audio(id: String) -> Result<String, String> {
    let dest = history::export_audio(&settings::audio_dir(), &id)?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn clear_all_history() -> Result<(), String> {
    history::clear_all(&settings::history_dir(), &settings::audio_dir());
    Ok(())
}

#[tauri::command]
pub fn get_history_storage_path() -> String {
    settings::base_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn get_app_icon(bundle_id: String) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        get_app_icon_macos(&bundle_id)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = bundle_id;
        Err("Not supported on this platform".to_string())
    }
}

#[cfg(target_os = "macos")]
fn get_app_icon_macos(bundle_id: &str) -> Result<String, String> {
    use base64::Engine;
    use std::ffi::c_void;

    extern "C" {
        fn sel_registerName(name: *const u8) -> *mut c_void;
        fn objc_getClass(name: *const u8) -> *mut c_void;
        fn objc_msgSend();
    }

    unsafe {
        let send_void: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let send_obj_obj: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());

        let ws_cls = objc_getClass(c"NSWorkspace".as_ptr().cast());
        if ws_cls.is_null() {
            return Err("NSWorkspace not found".to_string());
        }
        let workspace = send_void(ws_cls, sel_registerName(c"sharedWorkspace".as_ptr().cast()));
        if workspace.is_null() {
            return Err("sharedWorkspace is null".to_string());
        }

        let nsstring_cls = objc_getClass(c"NSString".as_ptr().cast());
        if nsstring_cls.is_null() {
            return Err("NSString class not found".to_string());
        }
        let c_bundle = std::ffi::CString::new(bundle_id.as_bytes())
            .map_err(|_| "Invalid bundle_id".to_string())?;
        let send_cstr: unsafe extern "C" fn(*mut c_void, *mut c_void, *const i8) -> *mut c_void =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let ns_bundle_id = send_cstr(
            nsstring_cls,
            sel_registerName(c"stringWithUTF8String:".as_ptr().cast()),
            c_bundle.as_ptr(),
        );
        if ns_bundle_id.is_null() {
            return Err("Failed to create NSString".to_string());
        }

        let app_url = send_obj_obj(
            workspace,
            sel_registerName(c"URLForApplicationWithBundleIdentifier:".as_ptr().cast()),
            ns_bundle_id,
        );
        if app_url.is_null() {
            return Err("App not found for bundle_id".to_string());
        }

        let app_path = send_void(app_url, sel_registerName(c"path".as_ptr().cast()));
        if app_path.is_null() {
            return Err("Failed to get app path".to_string());
        }

        let icon = send_obj_obj(
            workspace,
            sel_registerName(c"iconForFile:".as_ptr().cast()),
            app_path,
        );
        if icon.is_null() {
            return Err("Failed to get icon".to_string());
        }

        #[repr(C)]
        struct NSSize {
            width: f64,
            height: f64,
        }
        let send_set_size: unsafe extern "C" fn(*mut c_void, *mut c_void, NSSize) =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        send_set_size(
            icon,
            sel_registerName(c"setSize:".as_ptr().cast()),
            NSSize {
                width: 32.0,
                height: 32.0,
            },
        );

        let tiff_data = send_void(icon, sel_registerName(c"TIFFRepresentation".as_ptr().cast()));
        if tiff_data.is_null() {
            return Err("Failed to get TIFF data".to_string());
        }

        let bitmap_cls = objc_getClass(c"NSBitmapImageRep".as_ptr().cast());
        if bitmap_cls.is_null() {
            return Err("NSBitmapImageRep not found".to_string());
        }
        let bitmap_rep = send_obj_obj(
            bitmap_cls,
            sel_registerName(c"imageRepWithData:".as_ptr().cast()),
            tiff_data,
        );
        if bitmap_rep.is_null() {
            return Err("Failed to create bitmap rep".to_string());
        }

        let send_png: unsafe extern "C" fn(*mut c_void, *mut c_void, u64, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let empty_dict_cls = objc_getClass(c"NSDictionary".as_ptr().cast());
        let empty_dict = send_void(empty_dict_cls, sel_registerName(c"dictionary".as_ptr().cast()));
        let png_data = send_png(
            bitmap_rep,
            sel_registerName(c"representationUsingType:properties:".as_ptr().cast()),
            4,
            empty_dict,
        );
        if png_data.is_null() {
            return Err("Failed to create PNG data".to_string());
        }

        let send_len: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u64 =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let send_bytes: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const u8 =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());

        let len = send_len(png_data, sel_registerName(c"length".as_ptr().cast())) as usize;
        let bytes_ptr = send_bytes(png_data, sel_registerName(c"bytes".as_ptr().cast()));
        if bytes_ptr.is_null() || len == 0 {
            return Err("PNG data is empty".to_string());
        }

        let bytes = std::slice::from_raw_parts(bytes_ptr, len);
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        Ok(format!("data:image/png;base64,{}", b64))
    }
}

#[derive(Serialize)]
pub struct TestPolishResult {
    current_result: String,
    edited_result: String,
}

#[tauri::command]
pub fn test_polish(
    state: State<'_, AppState>,
    test_text: String,
    custom_prompt: String,
) -> Result<TestPolishResult, String> {
    let mut config = state.settings.lock().map_err(|e| e.to_string())?.polish.clone();
    let model_dir = settings::models_dir();

    if config.mode == polisher::PolishMode::Cloud {
        let key = get_cached_api_key(&state.api_key_cache, config.cloud.provider.as_key());
        if !key.is_empty() {
            config.cloud.api_key = key;
        }
    }

    let default_tmpl = polisher::base_prompt_template();
    let default_system_prompt = polisher::resolve_prompt(&default_tmpl);

    let custom_system_prompt = polisher::resolve_prompt(&custom_prompt);

    let default_result = polisher::polish_with_prompt(
        &state.llm_model,
        &model_dir,
        &config,
        &default_system_prompt,
        &test_text,
        &state.http_client,
    )?;

    let custom_result = polisher::polish_with_prompt(
        &state.llm_model,
        &model_dir,
        &config,
        &custom_system_prompt,
        &test_text,
        &state.http_client,
    )?;

    Ok(TestPolishResult {
        current_result: default_result,
        edited_result: custom_result,
    })
}

// ── Voice Add Rule ────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct GeneratedRule {
    name: String,
    match_type: String,
    match_value: String,
    prompt: String,
}

fn parse_generated_rule(raw: &str) -> Result<GeneratedRule, String> {
    let stripped = raw.trim();
    let stripped = if stripped.starts_with("```") {
        let s = stripped
            .trim_start_matches("```json")
            .trim_start_matches("```");
        s.strip_suffix("```").unwrap_or(s)
    } else {
        stripped
    }
    .trim();

    let start = stripped.find('{').ok_or("No JSON object found in LLM response")?;
    let end = stripped.rfind('}').ok_or("No closing brace found in LLM response")?;
    if end <= start {
        return Err("Invalid JSON structure".to_string());
    }
    let json_str = &stripped[start..=end];

    let val: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("JSON parse error: {e}"))?;

    let name = val
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let match_type = val
        .get("match_type")
        .and_then(|v| v.as_str())
        .unwrap_or("app_name")
        .to_string();
    let match_value = val
        .get("match_value")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let prompt = val
        .get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let match_type = match match_type.as_str() {
        "app_name" | "bundle_id" | "url" => match_type,
        _ => "app_name".to_string(),
    };

    Ok(GeneratedRule {
        name,
        match_type,
        match_value,
        prompt,
    })
}

#[tauri::command]
pub fn generate_rule_from_description(
    state: State<'_, AppState>,
    description: String,
) -> Result<GeneratedRule, String> {
    let mut config = state
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .polish
        .clone();
    let model_dir = settings::models_dir();

    if config.mode == polisher::PolishMode::Cloud {
        let key = get_cached_api_key(&state.api_key_cache, config.cloud.provider.as_key());
        if !key.is_empty() {
            config.cloud.api_key = key;
        }
    }

    if !polisher::is_polish_ready(&model_dir, &config) {
        return Err("LLM not configured".to_string());
    }

    let lang_hint = "the same language the user uses";

    let system_prompt = format!(
        r#"You are a JSON generator. The user will describe a prompt rule for a speech-to-text app. Your job is to convert the description into a structured JSON object.

Return ONLY a single JSON object with these fields:
- "name": a short descriptive name for the rule (max 30 chars)
- "match_type": one of "app_name", "bundle_id", or "url"
- "match_value": the value to match against (e.g. app name, bundle ID, or URL pattern)
- "prompt": the detailed instruction for AI polishing when this rule matches

If the user mentions a specific app, use "app_name" as match_type and the app name as match_value.
If the user mentions a website or URL, use "url" as match_type.
If you cannot determine the match target, leave match_value empty and use "app_name".

Write the "name" and "prompt" fields in {lang_hint}.
Do NOT include any explanation, only the JSON object."#
    );

    let result = polisher::polish_with_prompt(
        &state.llm_model,
        &model_dir,
        &config,
        &system_prompt,
        &description,
        &state.http_client,
    )?;

    parse_generated_rule(&result)
}

#[tauri::command]
pub fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    audio::do_start_recording(
        &state.is_recording,
        &state.mic_available,
        &state.sample_rate,
        &state.buffer,
        &state.is_recording,
    )
}

#[tauri::command]
pub fn stop_recording(state: State<'_, AppState>) -> Result<String, String> {
    let mut stt_config = state.settings.lock().map_err(|e| e.to_string())?.stt.clone();
    if stt_config.mode == SttMode::Cloud {
        let key = get_cached_api_key(&state.api_key_cache, stt_config.cloud.provider.as_key());
        if !key.is_empty() {
            stt_config.cloud.api_key = key;
        }
    }
    let stt_language = stt_config.language.clone();
    let dictionary_terms: Vec<String> = state
        .settings
        .lock()
        .map(|s| {
            s.polish
                .dictionary
                .entries
                .iter()
                .filter(|e| e.enabled && !e.term.is_empty())
                .map(|e| e.term.clone())
                .collect()
        })
        .unwrap_or_default();
    audio::do_stop_recording(
        &state.is_recording,
        &state.sample_rate,
        &state.buffer,
        &state.whisper_ctx,
        &state.http_client,
        &stt_config,
        &stt_language,
        "",
        &dictionary_terms,
        &state.vad_ctx,
        stt_config.vad_enabled,
    )
    .map(|(text, _samples)| text)
}

#[tauri::command]
pub fn set_test_mode(state: State<'_, AppState>, enabled: bool) {
    state.test_mode.store(enabled, Ordering::SeqCst);
}

#[tauri::command]
pub fn set_voice_rule_mode(state: State<'_, AppState>, enabled: bool) {
    state.voice_rule_mode.store(enabled, Ordering::SeqCst);
}

#[tauri::command]
pub fn set_context_override(
    state: State<'_, AppState>,
    app_name: String,
    bundle_id: String,
    url: String,
) -> Result<(), String> {
    use crate::context_detect;
    if let Ok(mut ctx) = state.context_override.lock() {
        if app_name.is_empty() && bundle_id.is_empty() && url.is_empty() {
            *ctx = None;
        } else {
            *ctx = Some(context_detect::AppContext { app_name, bundle_id, url });
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_edit_text_override(state: State<'_, AppState>, text: String) {
    if let Ok(mut ov) = state.edit_text_override.lock() {
        *ov = if text.is_empty() { None } else { Some(text) };
    }
}

#[tauri::command]
pub fn cancel_recording(app: AppHandle, state: State<'_, AppState>) {
    state.is_recording.store(false, Ordering::SeqCst);
    if let Some(overlay) = app.get_webview_window("overlay") {
        platform::hide_overlay(&overlay);
    }
}

#[derive(Serialize)]
pub struct MicStatus {
    connected: bool,
    default_device: Option<String>,
    devices: Vec<String>,
}

#[tauri::command]
pub fn get_mic_status(state: State<'_, AppState>) -> MicStatus {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let default_device = host.default_input_device().and_then(|d| d.name().ok());
    let devices: Vec<String> = host
        .input_devices()
        .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default();

    // Auto-reconnect: if mic was unavailable at startup but devices exist now, try to connect.
    let mut connected = state.mic_available.load(Ordering::SeqCst);
    if !connected && !devices.is_empty() {
        if audio::try_reconnect_audio(
            &state.mic_available,
            &state.sample_rate,
            &state.buffer,
            &state.is_recording,
        ).is_ok() {
            connected = true;
        }
    }

    MicStatus {
        connected,
        default_device,
        devices,
    }
}

// ── Model download ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ModelStatus {
    engine: String,
    model_exists: bool,
}

#[tauri::command]
pub fn check_model_status() -> ModelStatus {
    let model_exists = settings::models_dir()
        .join("ggml-large-v3-turbo-zh-TW.bin")
        .exists();
    ModelStatus {
        engine: "whisper".to_string(),
        model_exists,
    }
}

#[tauri::command]
pub fn download_model(app: AppHandle) -> Result<(), String> {
    use std::io::Read as _;

    let dir = settings::models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let model_path = dir.join("ggml-large-v3-turbo-zh-TW.bin");
    if model_path.exists() {
        let _ = app.emit("model-download-progress", serde_json::json!({
            "status": "complete",
            "downloaded": 0u64,
            "total": 0u64,
            "percent": 100.0
        }));
        return Ok(());
    }

    let tmp_path = model_path.with_extension("bin.part");
    let _ = std::fs::remove_file(&tmp_path);

    std::thread::spawn(move || {
        let url = "https://huggingface.co/Alkd/whisper-large-v3-turbo-zh-TW/resolve/main/ggml-model.bin";
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to create HTTP client: {}", e)
                }));
                return;
            }
        };

        let resp = match client.get(url).send() {
            Ok(r) => r,
            Err(e) => {
                let _ = app.emit("model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Download request failed: {}", e)
                }));
                return;
            }
        };

        if !resp.status().is_success() {
            let _ = app.emit("model-download-progress", serde_json::json!({
                "status": "error",
                "message": format!("Download returned HTTP {}", resp.status())
            }));
            return;
        }

        let total = resp.content_length().unwrap_or(0);

        let mut file = match std::fs::File::create(&tmp_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = app.emit("model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to create temp file: {}", e)
                }));
                return;
            }
        };

        let mut downloaded: u64 = 0;
        let mut buf = [0u8; 65536];
        let mut last_emit = Instant::now();
        let mut reader = resp;

        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = app.emit("model-download-progress", serde_json::json!({
                        "status": "error",
                        "message": format!("Download read error: {}", e)
                    }));
                    return;
                }
            };

            if let Err(e) = std::io::Write::write_all(&mut file, &buf[..n]) {
                let _ = app.emit("model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to write to disk: {}", e)
                }));
                return;
            }

            downloaded += n as u64;

            if last_emit.elapsed() >= std::time::Duration::from_millis(100) {
                let percent = if total > 0 {
                    (downloaded as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                let _ = app.emit("model-download-progress", serde_json::json!({
                    "status": "downloading",
                    "downloaded": downloaded,
                    "total": total,
                    "percent": percent
                }));
                last_emit = Instant::now();
            }
        }

        drop(file);
        if let Err(e) = std::fs::rename(&tmp_path, &model_path) {
            let _ = app.emit("model-download-progress", serde_json::json!({
                "status": "error",
                "message": format!("Failed to rename temp file: {}", e)
            }));
            return;
        }

        if let Some(app_state) = app.try_state::<AppState>() {
            if let Ok(mut ctx) = app_state.whisper_ctx.lock() {
                *ctx = None;
                println!("[Sumi] Whisper context cache invalidated after model download");
            }
        }

        let _ = app.emit("model-download-progress", serde_json::json!({
            "status": "complete",
            "downloaded": downloaded,
            "total": total,
            "percent": 100.0
        }));
        println!("[Sumi] Whisper model downloaded: {:?}", model_path);
    });

    Ok(())
}

// ── LLM Model management ────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct LlmModelStatus {
    model: String,
    model_exists: bool,
    model_size_bytes: u64,
}

#[tauri::command]
pub fn check_llm_model_status(state: State<'_, AppState>) -> LlmModelStatus {
    let settings = state.settings.lock().unwrap();
    let model = &settings.polish.model;
    let dir = settings::models_dir();
    let (exists, size) = polisher::model_file_status(&dir, model);
    LlmModelStatus {
        model: model.display_name().to_string(),
        model_exists: exists,
        model_size_bytes: size,
    }
}

#[tauri::command]
pub fn download_llm_model(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use std::io::Read as _;

    let dir = settings::models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    let model = settings.polish.model.clone();
    drop(settings);

    let model_path = dir.join(model.filename());
    if model_path.exists() {
        let _ = app.emit("llm-model-download-progress", serde_json::json!({
            "status": "complete",
            "downloaded": 0u64,
            "total": 0u64,
            "percent": 100.0
        }));
        return Ok(());
    }

    let tmp_path = model_path.with_extension("gguf.part");
    let _ = std::fs::remove_file(&tmp_path);

    let url = model.download_url().to_string();

    std::thread::spawn(move || {
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(1800))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("llm-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to create HTTP client: {}", e)
                }));
                return;
            }
        };

        let resp = match client.get(&url).send() {
            Ok(r) => r,
            Err(e) => {
                let _ = app.emit("llm-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Download request failed: {}", e)
                }));
                return;
            }
        };

        if !resp.status().is_success() {
            let _ = app.emit("llm-model-download-progress", serde_json::json!({
                "status": "error",
                "message": format!("Download returned HTTP {}", resp.status())
            }));
            return;
        }

        let total = resp.content_length().unwrap_or(0);

        let mut file = match std::fs::File::create(&tmp_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = app.emit("llm-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to create temp file: {}", e)
                }));
                return;
            }
        };

        let mut downloaded: u64 = 0;
        let mut buf = [0u8; 65536];
        let mut last_emit = Instant::now();
        let mut reader = resp;

        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = app.emit("llm-model-download-progress", serde_json::json!({
                        "status": "error",
                        "message": format!("Download read error: {}", e)
                    }));
                    return;
                }
            };

            if let Err(e) = std::io::Write::write_all(&mut file, &buf[..n]) {
                let _ = app.emit("llm-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to write to disk: {}", e)
                }));
                return;
            }

            downloaded += n as u64;

            if last_emit.elapsed() >= std::time::Duration::from_millis(100) {
                let percent = if total > 0 {
                    (downloaded as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                let _ = app.emit("llm-model-download-progress", serde_json::json!({
                    "status": "downloading",
                    "downloaded": downloaded,
                    "total": total,
                    "percent": percent
                }));
                last_emit = Instant::now();
            }
        }

        drop(file);
        if let Err(e) = std::fs::rename(&tmp_path, &model_path) {
            let _ = app.emit("llm-model-download-progress", serde_json::json!({
                "status": "error",
                "message": format!("Failed to rename temp file: {}", e)
            }));
            return;
        }

        if let Some(app_state) = app.try_state::<AppState>() {
            polisher::invalidate_cache(&app_state.llm_model);
        }

        let _ = app.emit("llm-model-download-progress", serde_json::json!({
            "status": "complete",
            "downloaded": downloaded,
            "total": total,
            "percent": 100.0
        }));
        println!("[Sumi] LLM model downloaded: {:?}", model_path);
    });

    Ok(())
}

// ── Polish model management ──────────────────────────────────────────────────

#[tauri::command]
pub fn list_polish_models(state: State<'_, AppState>) -> Vec<PolishModelInfo> {
    let active_model = state
        .settings
        .lock()
        .map(|s| s.polish.model.clone())
        .unwrap_or_default();
    polisher::PolishModel::all()
        .iter()
        .map(|m| PolishModelInfo::from_model(m, &active_model))
        .collect()
}

#[tauri::command]
pub fn switch_polish_model(state: State<'_, AppState>, model: polisher::PolishModel) -> Result<(), String> {
    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.polish.model = model.clone();
        settings::save_settings_to_disk(&settings);
    }

    // Invalidate LLM model cache so it reloads next time
    polisher::invalidate_cache(&state.llm_model);
    println!(
        "[Sumi] Polish model switched to {}",
        model.display_name()
    );

    Ok(())
}

#[tauri::command]
pub fn download_polish_model(app: AppHandle, model: polisher::PolishModel) -> Result<(), String> {
    use std::io::Read as _;

    let dir = settings::models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let model_path = dir.join(model.filename());
    if model_path.exists() {
        let _ = app.emit("polish-model-download-progress", serde_json::json!({
            "status": "complete",
            "downloaded": 0u64,
            "total": 0u64,
            "percent": 100.0
        }));
        return Ok(());
    }

    let tmp_path = model_path.with_extension("gguf.part");
    let _ = std::fs::remove_file(&tmp_path);

    let url = model.download_url().to_string();

    std::thread::spawn(move || {
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(1800))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("polish-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to create HTTP client: {}", e)
                }));
                return;
            }
        };

        let resp = match client.get(&url).send() {
            Ok(r) => r,
            Err(e) => {
                let _ = app.emit("polish-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Download request failed: {}", e)
                }));
                return;
            }
        };

        if !resp.status().is_success() {
            let _ = app.emit("polish-model-download-progress", serde_json::json!({
                "status": "error",
                "message": format!("Download returned HTTP {}", resp.status())
            }));
            return;
        }

        let total = resp.content_length().unwrap_or(0);

        let mut file = match std::fs::File::create(&tmp_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = app.emit("polish-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to create temp file: {}", e)
                }));
                return;
            }
        };

        let mut downloaded: u64 = 0;
        let mut buf = [0u8; 65536];
        let mut last_emit = Instant::now();
        let mut reader = resp;

        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = app.emit("polish-model-download-progress", serde_json::json!({
                        "status": "error",
                        "message": format!("Download read error: {}", e)
                    }));
                    return;
                }
            };

            if let Err(e) = std::io::Write::write_all(&mut file, &buf[..n]) {
                let _ = app.emit("polish-model-download-progress", serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to write to disk: {}", e)
                }));
                return;
            }

            downloaded += n as u64;

            if last_emit.elapsed() >= std::time::Duration::from_millis(100) {
                let percent = if total > 0 {
                    (downloaded as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                let _ = app.emit("polish-model-download-progress", serde_json::json!({
                    "status": "downloading",
                    "downloaded": downloaded,
                    "total": total,
                    "percent": percent
                }));
                last_emit = Instant::now();
            }
        }

        drop(file);
        if let Err(e) = std::fs::rename(&tmp_path, &model_path) {
            let _ = app.emit("polish-model-download-progress", serde_json::json!({
                "status": "error",
                "message": format!("Failed to rename temp file: {}", e)
            }));
            return;
        }

        if let Some(app_state) = app.try_state::<AppState>() {
            polisher::invalidate_cache(&app_state.llm_model);
        }

        let _ = app.emit("polish-model-download-progress", serde_json::json!({
            "status": "complete",
            "downloaded": downloaded,
            "total": total,
            "percent": 100.0
        }));
        println!("[Sumi] Polish model downloaded: {:?}", model_path);
    });

    Ok(())
}

// ── Whisper model management ─────────────────────────────────────────────────

#[tauri::command]
pub fn list_whisper_models(state: State<'_, AppState>) -> Vec<WhisperModelInfo> {
    let active_model = state
        .settings
        .lock()
        .map(|s| s.stt.whisper_model.clone())
        .unwrap_or_default();
    WhisperModel::all()
        .iter()
        .map(|m| WhisperModelInfo::from_model(m, &active_model))
        .collect()
}

#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    whisper_models::detect_system_info()
}

#[tauri::command]
pub fn get_whisper_model_recommendation(state: State<'_, AppState>) -> WhisperModel {
    let system = whisper_models::detect_system_info();
    let language = state
        .settings
        .lock()
        .ok()
        .and_then(|s| s.language.clone());
    whisper_models::recommend_model(&system, language.as_deref())
}

#[tauri::command]
pub fn switch_whisper_model(state: State<'_, AppState>, model: WhisperModel) -> Result<(), String> {
    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.stt.whisper_model = model.clone();
        settings::save_settings_to_disk(&settings);
    }

    // Invalidate whisper context cache so it reloads next time
    if let Ok(mut ctx) = state.whisper_ctx.lock() {
        *ctx = None;
        println!(
            "[Sumi] Whisper context cache invalidated after switching to {}",
            model.display_name()
        );
    }

    Ok(())
}

#[tauri::command]
pub fn download_whisper_model(app: AppHandle, model: WhisperModel) -> Result<(), String> {
    use std::io::Read as _;

    let url = model
        .download_url()
        .ok_or_else(|| format!("No download URL for model: {}", model.display_name()))?
        .to_string();

    let dir = settings::models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let model_path = dir.join(model.filename());
    if model_path.exists() {
        let _ = app.emit(
            "whisper-model-download-progress",
            serde_json::json!({
                "status": "complete",
                "downloaded": 0u64,
                "total": 0u64,
                "percent": 100.0
            }),
        );
        return Ok(());
    }

    let tmp_path = model_path.with_extension("bin.part");
    let _ = std::fs::remove_file(&tmp_path);

    // BelleZh downloads as ggml-model.bin but we rename to the canonical filename
    let needs_rename = model == WhisperModel::BelleZh || model == WhisperModel::LargeV3TurboZhTw;
    let _ = needs_rename; // used implicitly — rename always happens via tmp_path → model_path

    std::thread::spawn(move || {
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(1800))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit(
                    "whisper-model-download-progress",
                    serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to create HTTP client: {}", e)
                    }),
                );
                return;
            }
        };

        let resp = match client.get(&url).send() {
            Ok(r) => r,
            Err(e) => {
                let _ = app.emit(
                    "whisper-model-download-progress",
                    serde_json::json!({
                        "status": "error",
                        "message": format!("Download request failed: {}", e)
                    }),
                );
                return;
            }
        };

        if !resp.status().is_success() {
            let _ = app.emit(
                "whisper-model-download-progress",
                serde_json::json!({
                    "status": "error",
                    "message": format!("Download returned HTTP {}", resp.status())
                }),
            );
            return;
        }

        let total = resp.content_length().unwrap_or(0);

        let mut file = match std::fs::File::create(&tmp_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = app.emit(
                    "whisper-model-download-progress",
                    serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to create temp file: {}", e)
                    }),
                );
                return;
            }
        };

        let mut downloaded: u64 = 0;
        let mut buf = [0u8; 65536];
        let mut last_emit = Instant::now();
        let mut reader = resp;

        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = app.emit(
                        "whisper-model-download-progress",
                        serde_json::json!({
                            "status": "error",
                            "message": format!("Download read error: {}", e)
                        }),
                    );
                    return;
                }
            };

            if let Err(e) = std::io::Write::write_all(&mut file, &buf[..n]) {
                let _ = app.emit(
                    "whisper-model-download-progress",
                    serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to write to disk: {}", e)
                    }),
                );
                return;
            }

            downloaded += n as u64;

            if last_emit.elapsed() >= std::time::Duration::from_millis(100) {
                let percent = if total > 0 {
                    (downloaded as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                let _ = app.emit(
                    "whisper-model-download-progress",
                    serde_json::json!({
                        "status": "downloading",
                        "downloaded": downloaded,
                        "total": total,
                        "percent": percent
                    }),
                );
                last_emit = Instant::now();
            }
        }

        drop(file);
        if let Err(e) = std::fs::rename(&tmp_path, &model_path) {
            let _ = app.emit(
                "whisper-model-download-progress",
                serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to rename temp file: {}", e)
                }),
            );
            return;
        }

        // Invalidate whisper context cache
        if let Some(app_state) = app.try_state::<AppState>() {
            if let Ok(mut ctx) = app_state.whisper_ctx.lock() {
                *ctx = None;
                println!("[Sumi] Whisper context cache invalidated after model download");
            }
        }

        let _ = app.emit(
            "whisper-model-download-progress",
            serde_json::json!({
                "status": "complete",
                "downloaded": downloaded,
                "total": total,
                "percent": 100.0
            }),
        );
        println!("[Sumi] Whisper model downloaded: {:?}", model_path);
    });

    Ok(())
}

// ── VAD model commands ──────────────────────────────────────────────────────

#[tauri::command]
pub fn check_vad_model_status() -> Result<serde_json::Value, String> {
    let downloaded = crate::transcribe::vad_model_path().exists();
    Ok(serde_json::json!({ "downloaded": downloaded }))
}

#[tauri::command]
pub fn download_vad_model(app: AppHandle) -> Result<(), String> {
    use std::io::Read as _;

    let url = "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v6.2.0.bin";
    let dir = settings::models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let model_path = crate::transcribe::vad_model_path();
    if model_path.exists() {
        let _ = app.emit(
            "vad-model-download-progress",
            serde_json::json!({ "status": "complete" }),
        );
        return Ok(());
    }

    let tmp_path = model_path.with_extension("bin.part");
    let _ = std::fs::remove_file(&tmp_path);

    std::thread::spawn(move || {
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit(
                    "vad-model-download-progress",
                    serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to create HTTP client: {}", e)
                    }),
                );
                return;
            }
        };

        let resp = match client.get(url).send() {
            Ok(r) => r,
            Err(e) => {
                let _ = app.emit(
                    "vad-model-download-progress",
                    serde_json::json!({
                        "status": "error",
                        "message": format!("Download request failed: {}", e)
                    }),
                );
                return;
            }
        };

        if !resp.status().is_success() {
            let _ = app.emit(
                "vad-model-download-progress",
                serde_json::json!({
                    "status": "error",
                    "message": format!("Download returned HTTP {}", resp.status())
                }),
            );
            return;
        }

        let total = resp.content_length().unwrap_or(0);

        let mut file = match std::fs::File::create(&tmp_path) {
            Ok(f) => f,
            Err(e) => {
                let _ = app.emit(
                    "vad-model-download-progress",
                    serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to create temp file: {}", e)
                    }),
                );
                return;
            }
        };

        let mut downloaded: u64 = 0;
        let mut buf = [0u8; 65536];
        let mut reader = resp;

        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = app.emit(
                        "vad-model-download-progress",
                        serde_json::json!({
                            "status": "error",
                            "message": format!("Download read error: {}", e)
                        }),
                    );
                    return;
                }
            };

            if let Err(e) = std::io::Write::write_all(&mut file, &buf[..n]) {
                let _ = app.emit(
                    "vad-model-download-progress",
                    serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to write to disk: {}", e)
                    }),
                );
                return;
            }

            downloaded += n as u64;
        }

        drop(file);
        if let Err(e) = std::fs::rename(&tmp_path, &model_path) {
            let _ = app.emit(
                "vad-model-download-progress",
                serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to rename temp file: {}", e)
                }),
            );
            return;
        }

        // Invalidate VAD context cache so it reloads on next use
        if let Some(app_state) = app.try_state::<AppState>() {
            if let Ok(mut ctx) = app_state.vad_ctx.lock() {
                *ctx = None;
                println!("[Sumi] VAD context cache invalidated after model download");
            }
        }

        let _ = app.emit(
            "vad-model-download-progress",
            serde_json::json!({
                "status": "complete",
                "downloaded": downloaded,
                "total": total
            }),
        );
        println!("[Sumi] VAD model downloaded: {:?}", model_path);
    });

    Ok(())
}
