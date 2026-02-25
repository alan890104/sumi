use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Mutex;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;

use crate::context_detect::AppContext;

// ── Config ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolishConfig {
    pub enabled: bool,
    #[serde(default)]
    pub model: PolishModel,
    #[serde(default)]
    pub output_language: OutputLanguage,
    #[serde(default)]
    pub custom_prompt: Option<String>,
    #[serde(default)]
    pub mode: PolishMode,
    #[serde(default)]
    pub cloud: CloudConfig,
    #[serde(default = "default_prompt_rules")]
    pub prompt_rules: Vec<PromptRule>,
    #[serde(default)]
    pub dictionary: DictionaryConfig,
}

impl Default for PolishConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: PolishModel::default(),
            output_language: OutputLanguage::default(),
            custom_prompt: None,
            mode: PolishMode::default(),
            cloud: CloudConfig::default(),
            prompt_rules: default_prompt_rules(),
            dictionary: DictionaryConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PolishMode {
    #[default]
    Local,
    Cloud,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CloudProvider {
    Groq,
    OpenRouter,
    OpenAi,
    Gemini,
    Custom,
}

impl Default for CloudProvider {
    fn default() -> Self {
        Self::Groq
    }
}

impl CloudProvider {
    /// Returns the snake_case identifier matching the serde serialization.
    pub fn as_key(&self) -> &'static str {
        match self {
            CloudProvider::Groq => "groq",
            CloudProvider::OpenRouter => "open_router",
            CloudProvider::OpenAi => "open_ai",
            CloudProvider::Gemini => "gemini",
            CloudProvider::Custom => "custom",
        }
    }

    pub fn default_endpoint(&self) -> &'static str {
        match self {
            CloudProvider::Groq => "https://api.groq.com/openai/v1/chat/completions",
            CloudProvider::OpenRouter => "https://openrouter.ai/api/v1/chat/completions",
            CloudProvider::OpenAi => "https://api.openai.com/v1/chat/completions",
            CloudProvider::Gemini => "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
            CloudProvider::Custom => "",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    #[serde(default)]
    pub provider: CloudProvider,
    #[serde(skip)]
    pub api_key: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub model_id: String,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            provider: CloudProvider::default(),
            api_key: String::new(),
            endpoint: String::new(),
            model_id: "qwen/qwen3-32b".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PolishModel {
    #[default]
    LlamaTaiwan,
    Qwen25,
}

impl PolishModel {
    pub fn filename(&self) -> &'static str {
        match self {
            PolishModel::LlamaTaiwan => "Llama-3-Taiwan-8B-Instruct.Q4_K_M.gguf",
            PolishModel::Qwen25 => "qwen2.5-7b-instruct-q4_k_m.gguf",
        }
    }

    pub fn download_url(&self) -> &'static str {
        match self {
            PolishModel::LlamaTaiwan => {
                "https://huggingface.co/QuantFactory/Llama-3-Taiwan-8B-Instruct-GGUF/resolve/main/Llama-3-Taiwan-8B-Instruct.Q4_K_M.gguf"
            }
            PolishModel::Qwen25 => {
                "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf"
            }
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PolishModel::LlamaTaiwan => "Llama 3 Taiwan 8B",
            PolishModel::Qwen25 => "Qwen 2.5 7B",
        }
    }

    /// Chat template name recognized by llama.cpp
    fn chat_template_name(&self) -> &'static str {
        match self {
            PolishModel::LlamaTaiwan => "llama3",
            PolishModel::Qwen25 => "chatml",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OutputLanguage {
    #[default]
    TraditionalChinese,
    SimplifiedChinese,
    English,
    Japanese,
    Korean,
    Auto,
}

impl OutputLanguage {
    fn label(&self) -> &'static str {
        match self {
            OutputLanguage::TraditionalChinese => "Traditional Chinese (繁體中文)",
            OutputLanguage::SimplifiedChinese => "Simplified Chinese (简体中文)",
            OutputLanguage::English => "English",
            OutputLanguage::Japanese => "Japanese (日本語)",
            OutputLanguage::Korean => "Korean (한국어)",
            OutputLanguage::Auto => "the same language the user spoke in",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    AppName,
    BundleId,
    Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptRule {
    pub name: String,
    pub match_type: MatchType,
    pub match_value: String,
    pub prompt: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub term: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub entries: Vec<DictionaryEntry>,
}

impl Default for DictionaryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            entries: Vec::new(),
        }
    }
}

// ── Cached model ────────────────────────────────────────────────────────────

pub struct LlmModelCache {
    backend: LlamaBackend,
    model: LlamaModel,
    loaded_path: PathBuf,
}

// LlamaBackend is Send+Sync, LlamaModel is Send+Sync
unsafe impl Send for LlmModelCache {}

/// Returns built-in preset prompt rules for common apps.
/// Rule prompts contain only app-specific instructions — they are appended
/// to the base prompt at runtime by `build_system_prompt()`.
pub fn default_prompt_rules() -> Vec<PromptRule> {
    vec![
        PromptRule {
            name: "Gmail".to_string(),
            match_type: MatchType::Url,
            match_value: "mail.google.com".to_string(),
            prompt: "Restructure the spoken content into proper email format (greeting, body, sign-off).\n\
                     Use a professional, clear, and polite tone.\n\
                     Reply with ONLY the email text, nothing else."
                .to_string(),
            enabled: true,
        },
        PromptRule {
            name: "Notion".to_string(),
            match_type: MatchType::AppName,
            match_value: "Notion".to_string(),
            prompt: "Organize the spoken content into clear, readable prose or bullet points as appropriate.\n\
                     Use a clean, well-structured style suitable for documentation."
                .to_string(),
            enabled: true,
        },
    ]
}

/// Returns the base prompt template with `{language}` placeholder.
/// This contains universal STT processing rules applied to all transcriptions.
pub fn base_prompt_template() -> String {
    "Clean up speech-to-text output. Fix recognition errors, grammar, and punctuation. \
     Remove fillers and repetitions. If the speaker corrects themselves, keep only the final intent. \
     Preserve meaning and tone. Output in {language}. Reply with ONLY the cleaned text."
        .to_string()
}

/// Resolve a prompt template by replacing the `{language}` placeholder.
pub fn resolve_prompt(template: &str, language: &OutputLanguage) -> String {
    let lang_rule = language.label();
    template.replace("{language}", lang_rule).trim().to_string()
}

/// Format app context information into a single descriptive line.
fn format_app_context(context: &AppContext) -> String {
    if context.app_name.is_empty() {
        return String::new();
    }
    let mut line = format!("App: {}", context.app_name);
    if !context.url.is_empty() {
        line.push_str(&format!(" ({})", context.url));
    }
    line
}

/// Find the first matching prompt rule for the given app context.
fn find_matching_rule<'a>(rules: &'a [PromptRule], context: &AppContext) -> Option<&'a str> {
    for rule in rules {
        if !rule.enabled || rule.match_value.is_empty() {
            continue;
        }
        let matched = match rule.match_type {
            MatchType::AppName => context
                .app_name
                .to_lowercase()
                .contains(&rule.match_value.to_lowercase()),
            MatchType::BundleId => context.bundle_id == rule.match_value,
            MatchType::Url => {
                !context.url.is_empty()
                    && context
                        .url
                        .to_lowercase()
                        .contains(&rule.match_value.to_lowercase())
            }
        };
        if matched {
            println!("[OpenTypeless] Prompt rule matched: \"{}\"", rule.name);
            return Some(&rule.prompt);
        }
    }
    None
}

/// Format dictionary entries into a prompt block for the AI model.
fn format_dictionary_prompt(dictionary: &DictionaryConfig) -> String {
    if !dictionary.enabled {
        return String::new();
    }
    let active: Vec<&str> = dictionary
        .entries
        .iter()
        .filter(|e| e.enabled && !e.term.is_empty())
        .map(|e| e.term.as_str())
        .collect();
    if active.is_empty() {
        return String::new();
    }
    let mut block = String::from(
        "\n\nThe following are user-defined proper nouns. \
         When you encounter homophones or similar-sounding words, \
         automatically apply the correct form based on context:",
    );
    for term in &active {
        block.push_str(&format!("\n• {}", term));
    }
    block
}

/// Build the system prompt for polishing.
///
/// Composition: base prompt (or custom override) + matched rule context
/// + dictionary block + app context info.
fn build_system_prompt(config: &PolishConfig, context: &AppContext) -> String {
    // 1. Base prompt (or custom_prompt override)
    let base_tmpl = base_prompt_template();
    let base = config.custom_prompt.as_deref().unwrap_or(&base_tmpl);
    let mut prompt = resolve_prompt(base, &config.output_language);

    // 2. Append matched rule's context prompt
    if let Some(rule_prompt) = find_matching_rule(&config.prompt_rules, context) {
        prompt.push_str("\n\n");
        prompt.push_str(rule_prompt);
    }

    // 3. Append dictionary block
    prompt.push_str(&format_dictionary_prompt(&config.dictionary));

    // 4. Append app context info
    let context_line = format_app_context(context);
    if !context_line.is_empty() {
        prompt.push_str("\n\n");
        prompt.push_str(&context_line);
    }

    prompt
}

/// Polish transcribed text using a local LLM.
///
/// This function is meant to be called from a background thread.
/// It lazy-loads the model on first use and reuses it across calls.
///
/// On any error, returns the original text unchanged (graceful fallback).
pub fn polish_text(
    llm_cache: &Mutex<Option<LlmModelCache>>,
    model_dir: &std::path::Path,
    config: &PolishConfig,
    context: &AppContext,
    raw_text: &str,
) -> String {
    if raw_text.trim().is_empty() {
        return raw_text.to_string();
    }

    match polish_text_inner(llm_cache, model_dir, config, context, raw_text) {
        Ok(polished) => {
            // Safety: if output is empty or suspiciously long, use original
            if polished.trim().is_empty() {
                println!("[OpenTypeless] Polish returned empty, using original");
                return raw_text.to_string();
            }
            let raw_chars = raw_text.chars().count();
            let polished_chars = polished.chars().count();
            if polished_chars > raw_chars * 3 + 200 {
                println!(
                    "[OpenTypeless] Polish output too long ({} vs {} chars), likely hallucination — using original",
                    polished_chars,
                    raw_chars
                );
                return raw_text.to_string();
            }
            polished
        }
        Err(e) => {
            eprintln!("[OpenTypeless] Polish error: {} — using original text", e);
            raw_text.to_string()
        }
    }
}

fn polish_text_inner(
    llm_cache: &Mutex<Option<LlmModelCache>>,
    model_dir: &std::path::Path,
    config: &PolishConfig,
    context: &AppContext,
    raw_text: &str,
) -> Result<String, String> {
    let system_prompt = build_system_prompt(config, context);
    match config.mode {
        PolishMode::Cloud => run_cloud_inference(&config.cloud, &system_prompt, raw_text),
        PolishMode::Local => run_llm_inference(llm_cache, model_dir, config, &system_prompt, raw_text),
    }
}

/// Run cloud LLM inference via an OpenAI-compatible chat completions API.
fn run_cloud_inference(
    cloud: &CloudConfig,
    system_prompt: &str,
    raw_text: &str,
) -> Result<String, String> {
    if cloud.api_key.is_empty() {
        return Err("Cloud API key is not set".to_string());
    }

    let endpoint = if cloud.endpoint.is_empty() {
        cloud.provider.default_endpoint().to_string()
    } else {
        cloud.endpoint.clone()
    };

    if endpoint.is_empty() {
        return Err("Cloud API endpoint is not set".to_string());
    }

    let model_id = if cloud.model_id.is_empty() {
        return Err("Cloud model ID is not set".to_string());
    } else {
        &cloud.model_id
    };

    let body = serde_json::json!({
        "model": model_id,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": raw_text }
        ],
        "temperature": 0.3,
        "max_tokens": 512
    });

    println!("[OpenTypeless] Cloud polish: {} via {}", model_id, endpoint);
    let start = std::time::Instant::now();

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;

    let body_str = serde_json::to_string(&body).map_err(|e| format!("Serialize body: {}", e))?;

    let resp = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", cloud.api_key))
        .header("Content-Type", "application/json")
        .body(body_str)
        .send()
        .map_err(|e| format!("Cloud API request failed: {}", e))?;

    let status = resp.status();
    let resp_text = resp.text().map_err(|e| format!("Read response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Cloud API returned HTTP {}: {}", status, resp_text));
    }

    let json: serde_json::Value =
        serde_json::from_str(&resp_text).map_err(|e| format!("Parse response JSON: {}", e))?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| format!("Unexpected response format: {}", resp_text))?;

    println!(
        "[OpenTypeless] Cloud polish done: {:.0?}",
        start.elapsed()
    );

    Ok(content.trim().to_string())
}

/// Run LLM inference with the given system prompt and user text.
/// Handles model loading/caching, tokenization, and sampling.
fn run_llm_inference(
    llm_cache: &Mutex<Option<LlmModelCache>>,
    model_dir: &std::path::Path,
    config: &PolishConfig,
    system_prompt: &str,
    raw_text: &str,
) -> Result<String, String> {
    let model_path = model_dir.join(config.model.filename());
    if !model_path.exists() {
        return Err(format!(
            "Model file not found: {}",
            model_path.display()
        ));
    }

    // Ensure model is loaded (lazy init)
    {
        let mut cache = llm_cache.lock().map_err(|e| e.to_string())?;
        let needs_reload = match cache.as_ref() {
            Some(c) => c.loaded_path != model_path,
            None => true,
        };
        if needs_reload {
            let load_start = std::time::Instant::now();
            println!(
                "[OpenTypeless] Loading LLM: {}",
                config.model.display_name()
            );

            let mut backend = LlamaBackend::init().map_err(|e| format!("Backend init: {}", e))?;
            backend.void_logs();

            let model_params = LlamaModelParams::default().with_n_gpu_layers(99);
            let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)
                .map_err(|e| format!("Model load: {}", e))?;

            println!("[OpenTypeless] LLM loaded with GPU offload (took {:.0?})", load_start.elapsed());
            *cache = Some(LlmModelCache {
                backend,
                model,
                loaded_path: model_path.clone(),
            });
        }
    }

    let user_prompt = raw_text.to_string();

    let cache = llm_cache.lock().map_err(|e| e.to_string())?;
    let cache_ref = cache.as_ref().ok_or("LLM not loaded")?;

    let template_name = config.model.chat_template_name();
    let template = LlamaChatTemplate::new(template_name)
        .map_err(|e| format!("Chat template: {}", e))?;

    let messages = vec![
        LlamaChatMessage::new("system".to_string(), system_prompt.to_string())
            .map_err(|e| format!("System message: {}", e))?,
        LlamaChatMessage::new("user".to_string(), user_prompt)
            .map_err(|e| format!("User message: {}", e))?,
    ];

    let formatted = cache_ref
        .model
        .apply_chat_template(&template, &messages, true)
        .map_err(|e| format!("Apply template: {}", e))?;

    // Create a new context for this request.
    // Disable flash-attention to avoid GGML symbol collision between whisper-rs
    // and llama-cpp-2 (both bundle their own ggml with Metal, causing assertion
    // failures in the flash-attention kernel when symbols are resolved to the
    // wrong library).
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(2048))
        .with_n_batch(512)
        .with_flash_attention_policy(0); // LLAMA_FLASH_ATTN_TYPE_DISABLED

    let mut ctx = cache_ref
        .model
        .new_context(&cache_ref.backend, ctx_params)
        .map_err(|e| format!("Context create: {}", e))?;

    // Tokenize the formatted prompt
    let tokenize_start = std::time::Instant::now();
    let tokens = cache_ref
        .model
        .str_to_token(&formatted, llama_cpp_2::model::AddBos::Never)
        .map_err(|e| format!("Tokenize: {}", e))?;
    println!("[OpenTypeless] LLM tokenized: {} tokens ({:.0?})", tokens.len(), tokenize_start.elapsed());

    if tokens.is_empty() {
        return Err("Empty tokenization result".to_string());
    }

    // Feed prompt tokens in a batch
    let prompt_start = std::time::Instant::now();
    let mut batch = LlamaBatch::new(ctx.n_ctx() as usize, 1);
    let last_idx = tokens.len() - 1;
    for (i, &token) in tokens.iter().enumerate() {
        batch
            .add(token, i as i32, &[0], i == last_idx)
            .map_err(|e| format!("Batch add: {}", e))?;
    }

    ctx.decode(&mut batch)
        .map_err(|e| format!("Decode prompt: {}", e))?;
    println!("[OpenTypeless] LLM prompt eval: {:.0?} ({} tokens, {:.1} t/s)", prompt_start.elapsed(), tokens.len(), tokens.len() as f64 / prompt_start.elapsed().as_secs_f64());

    // Sample tokens
    let max_tokens: usize = 512;
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::temp(0.3),
        LlamaSampler::top_p(0.9, 1),
        LlamaSampler::greedy(),
    ]);

    let mut output_tokens: Vec<LlamaToken> = Vec::new();
    let mut n_decoded = tokens.len() as i32;
    let gen_start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(15);

    for _ in 0..max_tokens {
        // Timeout check
        if gen_start.elapsed() > timeout {
            println!("[OpenTypeless] Polish inference timeout (15s)");
            break;
        }

        let new_token = sampler.sample(&ctx, -1);
        sampler.accept(new_token);

        // Check for end of generation
        if cache_ref.model.is_eog_token(new_token) {
            break;
        }

        output_tokens.push(new_token);

        // Decode next token
        batch.clear();
        batch
            .add(new_token, n_decoded, &[0], true)
            .map_err(|e| format!("Batch add: {}", e))?;

        ctx.decode(&mut batch)
            .map_err(|e| format!("Decode: {}", e))?;

        n_decoded += 1;
    }

    let gen_elapsed = gen_start.elapsed();
    println!("[OpenTypeless] LLM generation: {} tokens in {:.0?} ({:.1} t/s)", output_tokens.len(), gen_elapsed, output_tokens.len() as f64 / gen_elapsed.as_secs_f64());

    // Decode output tokens to string
    let mut output = String::new();
    for &token in &output_tokens {
        if let Ok(bytes) = cache_ref.model.token_to_piece_bytes(token, 128, false, None) {
            output.push_str(&String::from_utf8_lossy(&bytes));
        }
    }

    Ok(output.trim().to_string())
}

/// Polish text using a specific system prompt (for testing/comparison).
pub fn polish_with_prompt(
    llm_cache: &Mutex<Option<LlmModelCache>>,
    model_dir: &std::path::Path,
    config: &PolishConfig,
    system_prompt: &str,
    raw_text: &str,
) -> Result<String, String> {
    run_llm_inference(llm_cache, model_dir, config, system_prompt, raw_text)
}

/// Ensure the LLM model is loaded into the cache (for pre-warming at startup).
pub fn ensure_model_loaded(
    llm_cache: &Mutex<Option<LlmModelCache>>,
    model_dir: &std::path::Path,
    config: &PolishConfig,
) {
    let model_path = model_dir.join(config.model.filename());
    if !model_path.exists() {
        return;
    }
    let mut cache = match llm_cache.lock() {
        Ok(c) => c,
        Err(_) => return,
    };
    let needs_reload = match cache.as_ref() {
        Some(c) => c.loaded_path != model_path,
        None => true,
    };
    if needs_reload {
        let mut backend = match LlamaBackend::init() {
            Ok(b) => b,
            Err(_) => return,
        };
        backend.void_logs();
        let model_params = LlamaModelParams::default().with_n_gpu_layers(99);
        match LlamaModel::load_from_file(&backend, &model_path, &model_params) {
            Ok(model) => {
                *cache = Some(LlmModelCache {
                    backend,
                    model,
                    loaded_path: model_path,
                });
            }
            Err(e) => {
                eprintln!("[OpenTypeless] LLM pre-warm load error: {}", e);
            }
        }
    }
}

/// Check if polishing is ready to run (either local model exists or cloud API key is set).
pub fn is_polish_ready(model_dir: &std::path::Path, config: &PolishConfig) -> bool {
    match config.mode {
        PolishMode::Cloud => !config.cloud.api_key.is_empty(),
        PolishMode::Local => model_dir.join(config.model.filename()).exists(),
    }
}

/// Check if a model file exists in the given directory.
pub fn model_file_exists(model_dir: &std::path::Path, model: &PolishModel) -> bool {
    model_dir.join(model.filename()).exists()
}

/// Get the file size of a model, or 0 if it doesn't exist.
pub fn model_file_size(model_dir: &std::path::Path, model: &PolishModel) -> u64 {
    let path = model_dir.join(model.filename());
    std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
}

/// Invalidate the cached LLM model so it gets reloaded on next use.
pub fn invalidate_cache(llm_cache: &Mutex<Option<LlmModelCache>>) {
    if let Ok(mut cache) = llm_cache.lock() {
        *cache = None;
        println!("[OpenTypeless] LLM model cache invalidated");
    }
}
