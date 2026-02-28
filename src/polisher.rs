use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub custom_prompt: Option<String>,
    #[serde(default)]
    pub mode: PolishMode,
    #[serde(default)]
    pub cloud: CloudConfig,
    #[serde(
        default = "default_prompt_rules_map",
        deserialize_with = "deserialize_prompt_rules"
    )]
    pub prompt_rules: HashMap<String, Vec<PromptRule>>,
    #[serde(default)]
    pub dictionary: DictionaryConfig,
    /// Enable model reasoning / chain-of-thought (e.g. Qwen3 `<think>` blocks).
    /// When false, `/no_think` is prepended to suppress reasoning.
    #[serde(default)]
    pub reasoning: bool,
}

impl Default for PolishConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: PolishModel::default(),
            custom_prompt: None,
            mode: PolishMode::default(),
            cloud: CloudConfig::default(),
            prompt_rules: default_prompt_rules_map(),
            dictionary: DictionaryConfig::default(),
            reasoning: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PolishMode {
    Local,
    #[default]
    Cloud,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CloudProvider {
    #[serde(rename = "github_models")]
    GitHubModels,
    Groq,
    OpenRouter,
    OpenAi,
    Gemini,
    SambaNova,
    Custom,
}

impl Default for CloudProvider {
    fn default() -> Self {
        Self::GitHubModels
    }
}

impl CloudProvider {
    /// Returns the snake_case identifier matching the serde serialization.
    pub fn as_key(&self) -> &'static str {
        match self {
            CloudProvider::GitHubModels => "github_models",
            CloudProvider::Groq => "groq",
            CloudProvider::OpenRouter => "open_router",
            CloudProvider::OpenAi => "open_ai",
            CloudProvider::Gemini => "gemini",
            CloudProvider::SambaNova => "samba_nova",
            CloudProvider::Custom => "custom",
        }
    }

    pub fn default_endpoint(&self) -> &'static str {
        match self {
            CloudProvider::GitHubModels => "https://models.github.ai/inference/chat/completions",
            CloudProvider::Groq => "https://api.groq.com/openai/v1/chat/completions",
            CloudProvider::OpenRouter => "https://openrouter.ai/api/v1/chat/completions",
            CloudProvider::OpenAi => "https://api.openai.com/v1/chat/completions",
            CloudProvider::Gemini => "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
            CloudProvider::SambaNova => "https://api.sambanova.ai/v1/chat/completions",
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
            model_id: "openai/gpt-4o-mini".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PolishModel {
    #[default]
    LlamaTaiwan,
    Qwen25,
    Qwen3,
}

impl PolishModel {
    pub fn filename(&self) -> &'static str {
        match self {
            PolishModel::LlamaTaiwan => "Llama-3-Taiwan-8B-Instruct.Q4_K_M.gguf",
            PolishModel::Qwen25 => "qwen2.5-7b-instruct-q4_k_m.gguf",
            PolishModel::Qwen3 => "Qwen3-8B-Q4_K_M.gguf",
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
            PolishModel::Qwen3 => {
                "https://huggingface.co/Qwen/Qwen3-8B-GGUF/resolve/main/Qwen3-8B-Q4_K_M.gguf"
            }
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PolishModel::LlamaTaiwan => "Llama 3 Taiwan 8B",
            PolishModel::Qwen25 => "Qwen 2.5 7B",
            PolishModel::Qwen3 => "Qwen 3 8B",
        }
    }

    pub fn size_bytes(&self) -> u64 {
        match self {
            PolishModel::LlamaTaiwan => 4_920_000_000,
            PolishModel::Qwen25 => 4_680_000_000,
            PolishModel::Qwen3 => 5_030_000_000,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PolishModel::LlamaTaiwan => "Best for Traditional Chinese",
            PolishModel::Qwen25 => "Multilingual",
            PolishModel::Qwen3 => "Latest multilingual, thinking/non-thinking",
        }
    }

    pub fn all() -> &'static [PolishModel] {
        &[PolishModel::LlamaTaiwan, PolishModel::Qwen25, PolishModel::Qwen3]
    }

    /// Chat template name recognized by llama.cpp
    fn chat_template_name(&self) -> &'static str {
        match self {
            PolishModel::LlamaTaiwan => "llama3",
            PolishModel::Qwen25 => "chatml",
            PolishModel::Qwen3 => "chatml",
        }
    }
}

// ── PolishModelInfo (for frontend serialization) ─────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct PolishModelInfo {
    pub id: PolishModel,
    pub display_name: &'static str,
    pub description: &'static str,
    pub size_bytes: u64,
    pub downloaded: bool,
    pub file_size_on_disk: u64,
    pub is_active: bool,
}

impl PolishModelInfo {
    pub fn from_model(model: &PolishModel, active_model: &PolishModel) -> Self {
        let dir = crate::settings::models_dir();
        let (downloaded, file_size_on_disk) = model_file_status(&dir, model);
        Self {
            id: model.clone(),
            display_name: model.display_name(),
            description: model.description(),
            size_bytes: model.size_bytes(),
            downloaded,
            file_size_on_disk,
            is_active: model == active_model,
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
    /// Optional icon key for the frontend (e.g. "terminal", "slack"). Auto-detected if None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
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

/// Returns a per-language map with built-in preset prompt rules.
/// Used by serde `#[serde(default = ...)]` and `PolishConfig::default()`.
fn default_prompt_rules_map() -> HashMap<String, Vec<PromptRule>> {
    let mut map = HashMap::new();
    map.insert("auto".to_string(), default_prompt_rules());
    map
}

/// Backwards-compatible deserializer: accepts either a per-language map (new/old format)
/// or a flat array.
fn deserialize_prompt_rules<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, Vec<PromptRule>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Format {
        Map(HashMap<String, Vec<PromptRule>>),
        List(Vec<PromptRule>),
    }

    match Format::deserialize(deserializer)? {
        Format::Map(map) => Ok(map),
        Format::List(list) => {
            let mut map = HashMap::new();
            map.insert("auto".to_string(), list);
            Ok(map)
        }
    }
}

/// Returns built-in preset prompt rules.
pub fn default_prompt_rules() -> Vec<PromptRule> {
    default_prompt_rules_for_lang(None)
}

/// Returns built-in preset prompt rules localised to `lang` (BCP-47).
/// Falls back to English when a translation is not available.
pub fn default_prompt_rules_for_lang(lang: Option<&str>) -> Vec<PromptRule> {
    let is_zh = lang
        .map(|l| l.starts_with("zh"))
        .unwrap_or(false);

    let (code_editor_prompt, ai_cli_prompt, chat_prompt, email_prompt, notion_prompt, slack_prompt, github_prompt, twitter_prompt) = if is_zh {
        (
            "使用者正在程式碼編輯器中工作（可能在寫程式碼、註解、commit 訊息，或與 AI 程式助手對話）。\n\
             完整保留所有程式碼、指令、路徑、變數名稱和技術術語。\n\
             輸出簡潔精確的文字，不要額外解釋。".to_string(),

            "使用者正在終端機中對 AI 程式助手口述提示或訊息。\n\
             語音內容會直接作為 AI 的輸入。\n\
             完整保留所有技術術語、程式碼引用、檔案路徑、變數名稱和指令。\n\
             輸出清晰、結構良好的文字。\n\
             只回覆整理後的文字，不要附加任何其他內容。".to_string(),

            "使用者正在傳送聊天訊息。\n\
             保持輕鬆、自然、口語化的語氣。\n\
             修正語法和贅詞，但保留說話者的個性和語意。\n\
             只回覆整理後的訊息文字，不要附加任何其他內容。".to_string(),

            "將口述內容整理成正式的電子郵件格式（問候語、正文、結尾）。\n\
             使用專業、清晰、有禮貌的語氣。\n\
             只回覆郵件文字，不要附加任何其他內容。".to_string(),

            "使用者正在 Notion 中撰寫內容（筆記、文件或 Wiki）。\n\
             產出乾淨、結構良好的文字，適合用於文件。\n\
             保留說話者所暗示的列表、標題或結構。\n\
             只回覆整理後的文字，不要附加任何其他內容。".to_string(),

            "使用者正在傳送 Slack 訊息。\n\
             保持專業但親切的語氣。\n\
             修正語法和贅詞，保持簡潔。\n\
             只回覆整理後的訊息文字，不要附加任何其他內容。".to_string(),

            "使用者正在 GitHub 上工作（如 PR 說明、Issue、Code Review 留言、Commit 訊息、README 或討論區）。\n\
             重要：無論口述使用什麼語言，一律以英文輸出。\n\
             使用清晰、專業、簡潔的語言，適合軟體協作場景。\n\
             完整保留所有技術術語、程式碼引用、檔案路徑和變數名稱。\n\
             當內容暗示有結構時（列表、標題、程式碼區塊），使用 Markdown 格式。\n\
             只回覆整理後的文字，不要附加任何其他內容。".to_string(),

            "使用者正在 X（Twitter）上撰寫貼文或回覆。\n\
             保持簡潔有力，在短篇幅中追求清晰。\n\
             修正語法但保留說話者的語調和風格。\n\
             只回覆整理後的文字，不要附加任何其他內容。".to_string(),
        )
    } else {
        (
            "The user is working in a code editor (possibly writing code, comments, commit messages, or chatting with an AI coding assistant).\n\
             Preserve all code, commands, paths, variable names, and technical terms exactly as spoken.\n\
             Output concise, precise text. No extra explanation.".to_string(),

            "The user is dictating a prompt or message to an AI coding assistant running in the terminal. \
             The spoken text will be sent as input to the AI. \
             Preserve all technical terms, code references, file paths, variable names, and commands exactly. \
             Output clear, well-structured text. \
             Reply with ONLY the cleaned text, nothing else.".to_string(),

            "The user is writing a chat message.\n\
             Keep a casual, natural, and conversational tone.\n\
             Fix grammar and filler words but preserve the speaker's personality and intent.\n\
             Reply with ONLY the cleaned message text, nothing else.".to_string(),

            "Restructure the spoken content into proper email format (greeting, body, sign-off).\n\
             Use a professional, clear, and polite tone.\n\
             Reply with ONLY the email text, nothing else.".to_string(),

            "The user is writing in Notion (notes, docs, or wiki).\n\
             Produce clean, well-structured text suitable for documentation.\n\
             Preserve any lists, headings, or structure implied by the speaker.\n\
             Reply with ONLY the cleaned text, nothing else.".to_string(),

            "The user is writing a Slack message.\n\
             Keep a professional but approachable tone.\n\
             Fix grammar and filler words. Keep it concise.\n\
             Reply with ONLY the cleaned message text, nothing else.".to_string(),

            "The user is working on GitHub (e.g. PR description, issue, code review comment, commit message, README, or discussion).\n\
             IMPORTANT: Always output in English, regardless of the language spoken.\n\
             Use clear, professional, and concise language appropriate for software collaboration.\n\
             Preserve all technical terms, code references, file paths, and variable names exactly as spoken.\n\
             Use Markdown formatting when the content implies structure (lists, headings, code blocks).\n\
             Reply with ONLY the cleaned text, nothing else.".to_string(),

            "The user is composing a post or reply on X (Twitter).\n\
             Keep it concise and punchy. Aim for clarity within a short format.\n\
             Fix grammar but preserve the speaker's voice and tone.\n\
             Reply with ONLY the cleaned text, nothing else.".to_string(),
        )
    };

    vec![
        // ── Email ──
        PromptRule {
            name: "Gmail".to_string(),
            match_type: MatchType::Url,
            match_value: "mail.google.com".to_string(),
            prompt: email_prompt,
            enabled: true,
            icon: None,
        },
        // ── AI CLI tools (detected via terminal subprocess enrichment) ──
        PromptRule {
            name: "Claude Code".to_string(),
            match_type: MatchType::AppName,
            match_value: "Claude Code".to_string(),
            prompt: ai_cli_prompt.clone(),
            enabled: true,
            icon: None,
        },
        PromptRule {
            name: "Gemini CLI".to_string(),
            match_type: MatchType::AppName,
            match_value: "Gemini CLI".to_string(),
            prompt: ai_cli_prompt.clone(),
            enabled: true,
            icon: None,
        },
        PromptRule {
            name: "Codex CLI".to_string(),
            match_type: MatchType::AppName,
            match_value: "Codex CLI".to_string(),
            prompt: ai_cli_prompt.clone(),
            enabled: true,
            icon: None,
        },
        PromptRule {
            name: "Aider".to_string(),
            match_type: MatchType::AppName,
            match_value: "Aider".to_string(),
            prompt: ai_cli_prompt,
            enabled: true,
            icon: None,
        },
        // ── Code editors & terminals ──
        PromptRule {
            name: "Terminal".to_string(),
            match_type: MatchType::AppName,
            match_value: "Terminal".to_string(),
            prompt: code_editor_prompt.clone(),
            enabled: true,
            icon: None,
        },
        PromptRule {
            name: "VSCode".to_string(),
            match_type: MatchType::AppName,
            match_value: "Code".to_string(),
            prompt: code_editor_prompt.clone(),
            enabled: true,
            icon: None,
        },
        PromptRule {
            name: "Cursor".to_string(),
            match_type: MatchType::AppName,
            match_value: "Cursor".to_string(),
            prompt: code_editor_prompt.clone(),
            enabled: true,
            icon: None,
        },
        PromptRule {
            name: "Antigravity".to_string(),
            match_type: MatchType::AppName,
            match_value: "Antigravity".to_string(),
            prompt: code_editor_prompt.clone(),
            enabled: true,
            icon: None,
        },
        PromptRule {
            name: "iTerm2".to_string(),
            match_type: MatchType::AppName,
            match_value: "iTerm2".to_string(),
            prompt: code_editor_prompt,
            enabled: true,
            icon: None,
        },
        // ── Notes & docs ──
        PromptRule {
            name: "Notion".to_string(),
            match_type: MatchType::Url,
            match_value: "notion.so".to_string(),
            prompt: notion_prompt,
            enabled: true,
            icon: None,
        },
        // ── Chat & messaging ──
        PromptRule {
            name: "WhatsApp".to_string(),
            match_type: MatchType::AppName,
            match_value: "WhatsApp".to_string(),
            prompt: chat_prompt.clone(),
            enabled: true,
            icon: None,
        },
        PromptRule {
            name: "Telegram".to_string(),
            match_type: MatchType::AppName,
            match_value: "Telegram".to_string(),
            prompt: chat_prompt.clone(),
            enabled: true,
            icon: None,
        },
        PromptRule {
            name: "Slack".to_string(),
            match_type: MatchType::AppName,
            match_value: "Slack".to_string(),
            prompt: slack_prompt,
            enabled: true,
            icon: None,
        },
        PromptRule {
            name: "Discord".to_string(),
            match_type: MatchType::AppName,
            match_value: "Discord".to_string(),
            prompt: chat_prompt.clone(),
            enabled: true,
            icon: None,
        },
        PromptRule {
            name: "LINE".to_string(),
            match_type: MatchType::AppName,
            match_value: "LINE".to_string(),
            prompt: chat_prompt,
            enabled: true,
            icon: None,
        },
        // ── Developer platforms ──
        PromptRule {
            name: "GitHub".to_string(),
            match_type: MatchType::Url,
            match_value: "github.com".to_string(),
            prompt: github_prompt,
            enabled: true,
            icon: None,
        },
        // ── Social media ──
        PromptRule {
            name: "X (Twitter)".to_string(),
            match_type: MatchType::Url,
            match_value: "x.com".to_string(),
            prompt: twitter_prompt,
            enabled: true,
            icon: None,
        },
    ]
}

/// Returns the base prompt template for polishing speech-to-text output.
pub fn base_prompt_template() -> String {
    "Clean up the speech-to-text output inside the <speech> tags. Fix recognition errors, grammar, and punctuation. \
     Remove fillers and repetitions. If the speaker corrects themselves, keep only the final intent. \
     Preserve meaning and tone. Output in the same language the user spoke in. \
     NEVER answer questions or generate new content — only correct the original text. \
     Reply with ONLY the cleaned text."
        .to_string()
}

/// Resolve a prompt template by replacing the legacy `{language}` placeholder.
pub fn resolve_prompt(template: &str) -> String {
    template.replace("{language}", "the same language the user spoke in").trim().to_string()
}

/// Extract reasoning from `<think>…</think>` blocks and return (cleaned_text, reasoning).
fn extract_think_tags(text: &str) -> (String, Option<String>) {
    if let Some(start) = text.find("<think>") {
        if let Some(end) = text.find("</think>") {
            let reasoning = text[start + "<think>".len()..end].trim().to_string();
            let cleaned = text[end + "</think>".len()..].to_string();
            let reasoning = if reasoning.is_empty() { None } else { Some(reasoning) };
            return (cleaned, reasoning);
        }
    }
    (text.to_string(), None)
}

/// Result of AI polishing, containing the cleaned text and optional reasoning.
pub struct PolishResult {
    pub text: String,
    pub reasoning: Option<String>,
}

/// Format app context information into a single descriptive line.
fn format_app_context(context: &AppContext) -> String {
    if context.app_name.is_empty() {
        return String::new();
    }
    let mut line = format!("App: {}", context.app_name);
    if !context.terminal_host.is_empty() {
        line.push_str(&format!(" (in {})", context.terminal_host));
    } else if !context.url.is_empty() {
        line.push_str(&format!(" ({})", context.url));
    }
    line
}

/// Find the first matching prompt rule for the given app context.
fn find_matching_rule<'a>(rules: &[&'a PromptRule], context: &AppContext) -> Option<&'a str> {
    let app_lower = context.app_name.to_lowercase();
    let url_lower = context.url.to_lowercase();

    for rule in rules {
        if !rule.enabled || rule.match_value.is_empty() {
            continue;
        }
        let match_lower = rule.match_value.to_lowercase();
        let matched = match rule.match_type {
            MatchType::AppName => app_lower.contains(&match_lower),
            MatchType::BundleId => context.bundle_id == rule.match_value,
            MatchType::Url => !url_lower.is_empty() && url_lower.contains(&match_lower),
        };
        if matched {
            println!("[Sumi] Prompt rule matched: \"{}\"", rule.name);
            return Some(&rule.prompt);
        }
    }
    println!("[Sumi] No prompt rule matched (app: {:?}, url: {:?})", context.app_name, context.url);
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
    let header = "\n\nThe following are user-defined proper nouns. \
         When you encounter homophones or similar-sounding words, \
         automatically apply the correct form based on context:";
    let mut block = String::from(header);
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
    let mut prompt = resolve_prompt(base);

    // 2. Append matched rule's context prompt (search all language keys)
    let all_rules: Vec<&PromptRule> = config.prompt_rules.values()
        .flat_map(|rules| rules.iter())
        .collect();
    if let Some(rule_prompt) = find_matching_rule(&all_rules, context) {
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
    client: &reqwest::blocking::Client,
) -> PolishResult {
    if raw_text.trim().is_empty() {
        return PolishResult { text: raw_text.to_string(), reasoning: None };
    }

    match polish_text_inner(llm_cache, model_dir, config, context, raw_text, client) {
        Ok(raw_output) => {
            // Extract reasoning from <think> blocks
            let (polished, reasoning) = extract_think_tags(&raw_output);
            // Strip any <speech> tags the LLM may have echoed back
            let polished = polished
                .replace("<speech>", "")
                .replace("</speech>", "");
            let polished = polished.trim().to_string();

            // Safety: if output is empty or suspiciously long, use original
            if polished.is_empty() {
                println!("[Sumi] Polish returned empty, using original");
                return PolishResult { text: raw_text.to_string(), reasoning };
            }
            let raw_chars = raw_text.chars().count();
            let polished_chars = polished.chars().count();
            if polished_chars > raw_chars * 3 + 200 {
                println!(
                    "[Sumi] Polish output too long ({} vs {} chars), likely hallucination — using original",
                    polished_chars,
                    raw_chars
                );
                return PolishResult { text: raw_text.to_string(), reasoning };
            }
            PolishResult { text: polished, reasoning }
        }
        Err(e) => {
            eprintln!("[Sumi] Polish error: {} — using original text", e);
            PolishResult { text: raw_text.to_string(), reasoning: None }
        }
    }
}

fn polish_text_inner(
    llm_cache: &Mutex<Option<LlmModelCache>>,
    model_dir: &std::path::Path,
    config: &PolishConfig,
    context: &AppContext,
    raw_text: &str,
    client: &reqwest::blocking::Client,
) -> Result<String, String> {
    let system_prompt = build_system_prompt(config, context);

    // Wrap user speech in XML tags so the LLM can clearly distinguish
    // instructions from the actual speech content to be polished.
    let wrapped = format!("<speech>\n{}\n</speech>", raw_text);

    // Prepend /no_think to suppress model reasoning (Qwen3 convention)
    let user_text = if config.reasoning {
        wrapped
    } else {
        format!("/no_think\n{}", wrapped)
    };

    match config.mode {
        PolishMode::Cloud => run_cloud_inference(&config.cloud, &system_prompt, &user_text, client),
        PolishMode::Local => run_llm_inference(llm_cache, model_dir, config, &system_prompt, &user_text),
    }
}

/// Run cloud LLM inference via an OpenAI-compatible chat completions API.
fn run_cloud_inference(
    cloud: &CloudConfig,
    system_prompt: &str,
    raw_text: &str,
    client: &reqwest::blocking::Client,
) -> Result<String, String> {
    if cloud.api_key.is_empty() {
        return Err("Cloud API key is not set".to_string());
    }

    let endpoint = if cloud.endpoint.is_empty() {
        cloud.provider.default_endpoint().to_string()
    } else {
        validate_custom_endpoint(&cloud.endpoint)?;
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
        "temperature": 0.1,
        "max_tokens": 512
    });

    println!("[Sumi] Cloud polish: {} via {}", model_id, sanitize_url_for_log(&endpoint));
    let start = std::time::Instant::now();

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
        let preview = truncate_for_error(&resp_text, 200);
        return Err(format!("Cloud API returned HTTP {}: {}", status, preview));
    }

    let json: serde_json::Value =
        serde_json::from_str(&resp_text).map_err(|e| format!("Parse response JSON: {}", e))?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| {
            let preview = truncate_for_error(&resp_text, 200);
            format!("Unexpected response format: {}", preview)
        })?;

    println!(
        "[Sumi] Cloud polish done: {:.0?}, {} chars",
        start.elapsed(),
        content.len()
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

    // Validate the GGUF file before loading to prevent SIGSEGV on corrupted files
    validate_gguf_file(&model_path, &config.model)?;

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
                "[Sumi] Loading LLM: {}",
                config.model.display_name()
            );

            let mut backend = LlamaBackend::init().map_err(|e| format!("Backend init: {}", e))?;
            backend.void_logs();

            let model_params = LlamaModelParams::default().with_n_gpu_layers(99);
            let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)
                .map_err(|e| format!("Model load: {}", e))?;

            println!("[Sumi] LLM loaded with GPU offload (took {:.0?})", load_start.elapsed());
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
    println!("[Sumi] LLM tokenized: {} tokens ({:.0?})", tokens.len(), tokenize_start.elapsed());

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
    println!("[Sumi] LLM prompt eval: {:.0?} ({} tokens, {:.1} t/s)", prompt_start.elapsed(), tokens.len(), tokens.len() as f64 / prompt_start.elapsed().as_secs_f64());

    // Sample tokens
    let max_tokens: usize = 512;
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::greedy(),
    ]);

    let mut output_tokens: Vec<LlamaToken> = Vec::new();
    let mut n_decoded = tokens.len() as i32;
    let gen_start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(15);

    for _ in 0..max_tokens {
        // Timeout check
        if gen_start.elapsed() > timeout {
            println!("[Sumi] Polish inference timeout (15s)");
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
    println!("[Sumi] LLM generation: {} tokens in {:.0?} ({:.1} t/s)", output_tokens.len(), gen_elapsed, output_tokens.len() as f64 / gen_elapsed.as_secs_f64());

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
    client: &reqwest::blocking::Client,
) -> Result<String, String> {
    let raw_output = match config.mode {
        PolishMode::Cloud => run_cloud_inference(&config.cloud, system_prompt, raw_text, client)?,
        PolishMode::Local => run_llm_inference(llm_cache, model_dir, config, system_prompt, raw_text)?,
    };
    let (cleaned, _) = extract_think_tags(&raw_output);
    Ok(cleaned.trim().to_string())
}

/// Build the system prompt for edit-by-instruction mode.
fn build_edit_system_prompt() -> String {
    "You are a text editing assistant. The user provides selected text and an editing instruction.\n\
     Modify the selected text according to the instruction and output ONLY the modified result.\n\
     Do not add any explanation, prefix, or extra text. Output only the final result."
        .to_string()
}

/// Edit text by applying a voice instruction using the LLM.
///
/// Takes the selected text and a spoken instruction (e.g. "translate to English",
/// "rewrite in formal tone"), and returns the modified text.
pub fn edit_text_by_instruction(
    llm_cache: &Mutex<Option<LlmModelCache>>,
    model_dir: &std::path::Path,
    config: &PolishConfig,
    selected_text: &str,
    instruction: &str,
    client: &reqwest::blocking::Client,
) -> Result<String, String> {
    if selected_text.trim().is_empty() {
        return Err("Selected text is empty".to_string());
    }
    if instruction.trim().is_empty() {
        return Err("Instruction is empty".to_string());
    }

    let system_prompt = build_edit_system_prompt();

    let user_text = format!(
        "<selected_text>\n{}\n</selected_text>\n\n<instruction>\n{}\n</instruction>",
        selected_text, instruction
    );

    // Prepend /no_think to suppress model reasoning unless enabled
    let user_text = if config.reasoning {
        user_text
    } else {
        format!("/no_think\n{}", user_text)
    };

    let raw_output = match config.mode {
        PolishMode::Cloud => run_cloud_inference(&config.cloud, &system_prompt, &user_text, client)?,
        PolishMode::Local => run_llm_inference(llm_cache, model_dir, config, &system_prompt, &user_text)?,
    };

    let (cleaned, _reasoning) = extract_think_tags(&raw_output);

    // Strip any XML tags the LLM may have echoed back
    let cleaned = cleaned
        .replace("<selected_text>", "")
        .replace("</selected_text>", "")
        .replace("<instruction>", "")
        .replace("</instruction>", "");
    let cleaned = cleaned.trim().to_string();

    if cleaned.is_empty() {
        return Err("LLM returned empty result".to_string());
    }

    Ok(cleaned)
}

/// Validate a GGUF model file by checking magic bytes, version, and file size.
/// Returns `Ok(())` if the file appears valid, or an error describing the problem.
pub fn validate_gguf_file(path: &std::path::Path, expected_model: &PolishModel) -> Result<(), String> {
    use std::io::Read;

    let mut f = std::fs::File::open(path).map_err(|e| format!("Cannot open model file: {}", e))?;

    // Check GGUF magic bytes
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)
        .map_err(|e| format!("Cannot read GGUF header: {}", e))?;
    if &magic != b"GGUF" {
        return Err(format!(
            "Invalid GGUF magic: expected 'GGUF', got {:?}",
            magic
        ));
    }

    // Check GGUF version (2 or 3 are valid)
    let mut version_bytes = [0u8; 4];
    f.read_exact(&mut version_bytes)
        .map_err(|e| format!("Cannot read GGUF version: {}", e))?;
    let version = u32::from_le_bytes(version_bytes);
    if version < 2 || version > 3 {
        return Err(format!("Unsupported GGUF version: {}", version));
    }

    // Check file size is at least 90% of the expected size (catch truncated downloads)
    let file_size = std::fs::metadata(path)
        .map_err(|e| format!("Cannot stat model file: {}", e))?
        .len();
    let expected_size = expected_model.size_bytes();
    let min_size = expected_size * 9 / 10;
    if file_size < min_size {
        return Err(format!(
            "Model file too small: {} bytes (expected ~{} bytes, min {}). File may be corrupted or incomplete.",
            file_size, expected_size, min_size
        ));
    }

    Ok(())
}

/// Check if polishing is ready to run (either local model exists or cloud API key is set).
pub fn is_polish_ready(model_dir: &std::path::Path, config: &PolishConfig) -> bool {
    match config.mode {
        PolishMode::Cloud => !config.cloud.api_key.is_empty(),
        PolishMode::Local => model_dir.join(config.model.filename()).exists(),
    }
}

/// Check existence and size in a single metadata call.
pub fn model_file_status(model_dir: &std::path::Path, model: &PolishModel) -> (bool, u64) {
    let path = model_dir.join(model.filename());
    match std::fs::metadata(&path) {
        Ok(m) => (true, m.len()),
        Err(_) => (false, 0),
    }
}

/// Invalidate the cached LLM model so it gets reloaded on next use.
pub fn invalidate_cache(llm_cache: &Mutex<Option<LlmModelCache>>) {
    if let Ok(mut cache) = llm_cache.lock() {
        *cache = None;
        println!("[Sumi] LLM model cache invalidated");
    }
}

/// Extract only the host from a URL for safe logging (strips path, query params, credentials).
fn sanitize_url_for_log(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(parsed) => {
            let host = parsed.host_str().unwrap_or("unknown");
            let port = parsed.port().map(|p| format!(":{}", p)).unwrap_or_default();
            format!("{}://{}{}", parsed.scheme(), host, port)
        }
        Err(_) => "invalid-url".to_string(),
    }
}

/// Truncate a string for inclusion in error messages to avoid leaking large response bodies.
fn truncate_for_error(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        // Find a valid UTF-8 boundary
        let mut end = max_len;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }
}

/// Validate a custom cloud endpoint URL.
/// Allows localhost/private IPs (needed for local model servers like Ollama, LM Studio)
/// but blocks known dangerous targets (cloud metadata endpoints) and requires http(s).
pub fn validate_custom_endpoint(url_str: &str) -> Result<(), String> {
    if url_str.is_empty() {
        return Err("Endpoint URL is empty".to_string());
    }

    let parsed = url::Url::parse(url_str)
        .map_err(|e| format!("Invalid endpoint URL: {}", e))?;

    // Only allow HTTP and HTTPS schemes (block file://, ftp://, etc.)
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return Err(format!(
            "Endpoint must use HTTP or HTTPS (got \"{}://\")",
            parsed.scheme()
        ));
    }

    let host = parsed.host_str().unwrap_or("");
    if host.is_empty() {
        return Err("Endpoint URL has no host".to_string());
    }

    // Block cloud metadata endpoints (AWS/GCP/Azure instance metadata)
    if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
        // 169.254.169.254 — AWS/GCP/Azure instance metadata service
        if ip == std::net::Ipv4Addr::new(169, 254, 169, 254) {
            return Err("Endpoint must not target a cloud metadata address".to_string());
        }
    }
    // GCP metadata hostname
    if host == "metadata.google.internal" {
        return Err("Endpoint must not target a cloud metadata address".to_string());
    }

    // Reject credentials embedded in URL (e.g. https://user:pass@host/)
    if parsed.username() != "" || parsed.password().is_some() {
        return Err("Endpoint URL must not contain embedded credentials".to_string());
    }

    // Warn via log (not block) if using plain HTTP to a non-local host
    if parsed.scheme() == "http" {
        let is_local = host == "localhost"
            || host == "127.0.0.1"
            || host == "[::1]"
            || host == "0.0.0.0"
            || host.parse::<std::net::Ipv4Addr>().map_or(false, |ip| ip.is_private());
        if !is_local {
            eprintln!(
                "[Sumi] Warning: custom endpoint uses plain HTTP to remote host ({}). Data will be sent unencrypted.",
                host
            );
        }
    }

    Ok(())
}
