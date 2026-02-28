use serde::{Deserialize, Serialize};

use crate::polisher::truncate_for_error;
use crate::whisper_models::WhisperModel;

fn default_true() -> bool {
    true
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SttMode {
    #[default]
    Local,
    Cloud,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SttProvider {
    #[default]
    Deepgram,
    Groq,
    OpenAi,
    Azure,
    Custom,
}

impl SttProvider {
    pub fn as_key(&self) -> &'static str {
        match self {
            Self::Deepgram => "stt_deepgram",
            Self::Groq => "stt_groq",
            Self::OpenAi => "stt_open_ai",
            Self::Azure => "stt_azure",
            Self::Custom => "stt_custom",
        }
    }

    pub fn default_endpoint(&self) -> &'static str {
        match self {
            Self::Deepgram => "https://api.deepgram.com/v1/listen",
            Self::Groq => "https://api.groq.com/openai/v1/audio/transcriptions",
            Self::OpenAi => "https://api.openai.com/v1/audio/transcriptions",
            Self::Azure => "",
            Self::Custom => "",
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            Self::Deepgram => "whisper",
            Self::Groq => "whisper-large-v3-turbo",
            Self::OpenAi => "whisper-1",
            Self::Azure => "",
            Self::Custom => "",
        }
    }

    /// Whether this provider uses the OpenAI-compatible multipart API.
    pub fn is_openai_compatible(&self) -> bool {
        matches!(self, Self::Groq | Self::OpenAi | Self::Custom)
    }

    /// Whether the provider requires an endpoint URL from the user.
    pub fn requires_endpoint(&self) -> bool {
        matches!(self, Self::Azure | Self::Custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttCloudConfig {
    #[serde(default)]
    pub provider: SttProvider,
    #[serde(skip)]
    pub api_key: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default = "default_stt_model_id")]
    pub model_id: String,
    /// BCP-47 language code for STT (e.g. "zh-TW", "en", "ja").
    /// Empty string means auto-detect (provider-dependent).
    #[serde(default = "default_stt_language")]
    pub language: String,
}

fn default_stt_model_id() -> String {
    "whisper".to_string()
}

fn default_stt_language() -> String {
    "auto".to_string()
}

impl Default for SttCloudConfig {
    fn default() -> Self {
        Self {
            provider: SttProvider::default(),
            api_key: String::new(),
            endpoint: String::new(),
            model_id: default_stt_model_id(),
            language: default_stt_language(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    #[serde(default)]
    pub mode: SttMode,
    #[serde(default)]
    pub cloud: SttCloudConfig,
    #[serde(default)]
    pub whisper_model: WhisperModel,
    /// BCP-47 language code shared by both local and cloud STT.
    /// Migrated from `cloud.language` for older settings files.
    #[serde(default = "default_stt_language")]
    pub language: String,
    /// Whether to use Silero VAD to filter out non-speech audio before transcription.
    #[serde(default = "default_true")]
    pub vad_enabled: bool,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            mode: SttMode::default(),
            cloud: SttCloudConfig::default(),
            whisper_model: WhisperModel::default(),
            language: default_stt_language(),
            vad_enabled: true,
        }
    }
}

impl SttConfig {
    /// Migrate: if top-level `language` is the default but `cloud.language`
    /// was customised, pull it up.  Called once on settings load.
    pub fn migrate_language(&mut self) {
        // Treat empty string as "auto"
        if self.language.is_empty() {
            self.language = default_stt_language();
        }
        if self.cloud.language.is_empty() {
            self.cloud.language = default_stt_language();
        }
        if self.language == default_stt_language() && self.cloud.language != default_stt_language() {
            self.language = self.cloud.language.clone();
        }
        // Keep cloud.language in sync so cloud providers still work.
        self.cloud.language = self.language.clone();
    }
}

/// Map a raw system locale identifier (e.g. `"zh_tw"`, `"en_us"`) to a
/// BCP-47 language code recognised by Whisper.  Returns `"auto"` when the
/// locale cannot be mapped.
pub fn locale_to_stt_language(locale: &str) -> String {
    let lower = locale.to_lowercase();
    // Strip encoding suffix (e.g. "en_us.utf-8" → "en_us")
    let base = lower.split('.').next().unwrap_or(&lower);

    // Chinese: region/script matters
    if base.starts_with("zh") {
        if base.contains("tw") || base.contains("hant") {
            return "zh-TW".to_string();
        }
        return "zh-CN".to_string();
    }

    // Extract the language part (before _ or -)
    let lang = base.split(|c: char| c == '_' || c == '-').next().unwrap_or(base);

    const VALID: &[&str] = &[
        "af", "am", "ar", "as", "az", "ba", "be", "bg", "bn", "bo", "br", "bs",
        "ca", "cs", "cy", "da", "de", "el", "en", "es", "et", "eu", "fa", "fi",
        "fo", "fr", "gl", "gu", "ha", "haw", "he", "hi", "hr", "ht", "hu", "hy",
        "id", "is", "it", "ja", "jw", "ka", "kk", "km", "kn", "ko", "lb", "ln",
        "lo", "lt", "lv", "mg", "mi", "mk", "ml", "mn", "mr", "ms", "mt", "my",
        "ne", "nl", "nn", "no", "oc", "pa", "pl", "ps", "pt", "ro", "ru", "sa",
        "sd", "si", "sk", "sl", "sn", "so", "sq", "sr", "su", "sv", "sw", "ta",
        "te", "tg", "th", "tk", "tl", "tr", "tt", "uk", "ur", "uz", "vi", "yi",
        "yo", "yue",
    ];

    if VALID.contains(&lang) {
        lang.to_string()
    } else {
        "auto".to_string()
    }
}

/// Transcribe audio via a cloud STT API.
pub fn run_cloud_stt(stt_cloud: &SttCloudConfig, samples_16k: &[f32], client: &reqwest::blocking::Client) -> Result<String, String> {
    if stt_cloud.api_key.is_empty() {
        return Err("Cloud STT API key is not set. Please configure it in Settings.".to_string());
    }

    let endpoint = if stt_cloud.provider == SttProvider::Azure {
        let region = stt_cloud.endpoint.trim();
        if region.is_empty() {
            return Err("Azure region is not configured. Please set it in Settings.".to_string());
        }
        format!(
            "https://{}.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1",
            region
        )
    } else if stt_cloud.provider == SttProvider::Custom {
        if stt_cloud.endpoint.is_empty() {
            return Err("Cloud STT endpoint is not configured.".to_string());
        }
        crate::polisher::validate_custom_endpoint(&stt_cloud.endpoint)?;
        stt_cloud.endpoint.clone()
    } else {
        let default_ep = stt_cloud.provider.default_endpoint();
        if default_ep.is_empty() {
            if !stt_cloud.endpoint.is_empty() {
                crate::polisher::validate_custom_endpoint(&stt_cloud.endpoint)?;
            }
            stt_cloud.endpoint.clone()
        } else {
            default_ep.to_string()
        }
    };
    if endpoint.is_empty() {
        return Err("Cloud STT endpoint is not configured.".to_string());
    }

    let model_id = {
        let default = stt_cloud.provider.default_model();
        if default.is_empty() {
            stt_cloud.model_id.clone()
        } else {
            default.to_string()
        }
    };

    // Encode f32 samples → 16-bit PCM WAV in-memory
    let wav_bytes = {
        let num_samples = samples_16k.len();
        let data_size = (num_samples * 2) as u32;
        let file_size = 36 + data_size;
        let mut buf = Vec::with_capacity(44 + data_size as usize);

        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&file_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&16000u32.to_le_bytes());
        buf.extend_from_slice(&32000u32.to_le_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.extend_from_slice(&16u16.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        for &s in samples_16k {
            let clamped = s.clamp(-1.0, 1.0);
            let val = (clamped * 32767.0) as i16;
            buf.extend_from_slice(&val.to_le_bytes());
        }
        buf
    };

    let language = if stt_cloud.language == "auto" { "" } else { &stt_cloud.language };

    let resp = match stt_cloud.provider {
        SttProvider::Deepgram => {
            let lang_param = if language.is_empty() { "multi".to_string() } else { language.to_string() };
            let url = format!("{}?model={}&language={}&punctuate=true&smart_format=true",
                endpoint, model_id, lang_param);
            client
                .post(&url)
                .header("Authorization", format!("Token {}", stt_cloud.api_key))
                .header("Content-Type", "audio/wav")
                .body(wav_bytes)
                .send()
                .map_err(|e| format!("Cloud STT request failed: {}", e))?
        }
        SttProvider::Azure => {
            let lang_param = if language.is_empty() { "en-US".to_string() } else { language.to_string() };
            let url = format!("{}?language={}&format=simple", endpoint, lang_param);
            client
                .post(&url)
                .header("Ocp-Apim-Subscription-Key", &stt_cloud.api_key)
                .header("Content-Type", "audio/wav; codecs=audio/pcm; samplerate=16000")
                .header("Accept", "application/json")
                .body(wav_bytes)
                .send()
                .map_err(|e| format!("Cloud STT request failed: {}", e))?
        }
        _ => {
            let file_part = reqwest::blocking::multipart::Part::bytes(wav_bytes)
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .map_err(|e| format!("Failed to create multipart part: {}", e))?;

            let mut form = reqwest::blocking::multipart::Form::new()
                .part("file", file_part)
                .text("model", model_id)
                .text("response_format", "json");

            if !language.is_empty() {
                let iso_lang = language.split('-').next().unwrap_or("").to_string();
                if !iso_lang.is_empty() {
                    form = form.text("language", iso_lang);
                }
            }

            client
                .post(&endpoint)
                .header("Authorization", format!("Bearer {}", stt_cloud.api_key))
                .multipart(form)
                .send()
                .map_err(|e| format!("Cloud STT request failed: {}", e))?
        }
    };

    let status = resp.status();
    let body = resp
        .text()
        .map_err(|e| format!("Failed to read Cloud STT response: {}", e))?;

    if !status.is_success() {
        let preview = truncate_for_error(&body, 200);
        return Err(format!("Cloud STT returned HTTP {}: {}", status, preview));
    }

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| {
            let preview = truncate_for_error(&body, 200);
            format!("Failed to parse Cloud STT response: {} — body: {}", e, preview)
        })?;

    let text = match stt_cloud.provider {
        SttProvider::Deepgram => {
            json["results"]["channels"]
                .as_array()
                .and_then(|ch| ch.first())
                .and_then(|c| c["alternatives"].as_array())
                .and_then(|alts| alts.first())
                .and_then(|a| a["transcript"].as_str())
                .unwrap_or("")
                .trim()
                .to_string()
        }
        SttProvider::Azure => {
            json["DisplayText"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string()
        }
        _ => {
            json["text"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string()
        }
    };

    if text.is_empty() {
        Err("no_speech".to_string())
    } else {
        Ok(text)
    }
}

