# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Voxink is a macOS desktop app (Tauri 2) that provides system-wide speech-to-text via a global hotkey. It supports both local (Whisper via `whisper-rs` with Metal acceleration) and cloud STT APIs (Groq/OpenAI/Deepgram/Azure/Custom) for transcription, and pastes the result at the cursor. Optionally uses a local LLM (via `llama-cpp-2`) or cloud API (Groq/OpenRouter/OpenAI/Gemini/SambaNova/Custom) to polish transcription output.

## Commands

```bash
# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build

# Type-check without building
cargo check

# Lint
cargo clippy
```

No frontend build step — the frontend is plain HTML/CSS/JS served directly from `frontend/`.

## Architecture

### Backend

Rust source files:

#### `src/lib.rs` — Core application logic
- **`AppState`** — shared state managed by Tauri: `is_recording` (AtomicBool), `buffer` (Arc<Mutex<Vec<f32>>>), `sample_rate`, `settings`, `mic_available`, `whisper_ctx`, `llm_model`, `captured_context`, `test_mode` (AtomicBool).
- **`Settings`** — persisted to `~/.config/com.voxink.app/settings.json`. Fields: `hotkey`, `auto_paste`, `polish` (PolishConfig), `stt` (SttConfig), `history_retention_days` (u32, 0 = keep forever), `language` (Option<String>, UI language override).
- **`SttConfig`** — fields: `mode` (SttMode: Local or Cloud), `cloud` (SttCloudConfig). Cloud STT supports Groq, OpenAI, Deepgram, Azure, Custom providers.
- **`spawn_audio_thread`** — creates a persistent always-on cpal input stream at app startup. The callback checks `is_recording` atomically and discards samples when false, giving true zero-latency recording start.
- **`do_start_recording`** — clears the buffer and flips `is_recording` to true (instant, <5 ms).
- **`do_stop_recording`** — flips `is_recording` to false, extracts samples, resamples to 16 kHz. Dispatches to local Whisper or cloud STT based on `SttConfig.mode`.
- **Transcription** — `transcribe_with_cached_whisper` (local, loads model once, reuses WhisperContext) or `run_cloud_stt` (cloud, supports multipart/form-data for Groq/OpenAI/Custom, raw binary for Deepgram/Azure).
- **`keychain` module** — macOS Keychain integration via `security` CLI for storing cloud API keys per-provider (`save`, `load`, `delete`).
- **`macos_ffi` module** — unsafe Objective-C runtime calls for:
  - `setup_overlay`: sets window level to NSFloatingWindowLevel, disables `hidesOnDeactivate`, joins all Spaces.
  - `show_no_activate` / `hide_window`: `orderFrontRegardless` / `orderOut:` so the overlay never steals focus.
  - `simulate_cmd_v`: CGEvent-based Cmd+V simulation at the HID level.
- **Global shortcut handler** — single toggle: first press starts recording + shows overlay; second press stops recording, transcribes, optionally polishes with LLM, copies to clipboard, optionally pastes with Cmd+V, then hides the overlay. Max recording duration: 30 seconds (auto-stop).
- **Tauri commands** exposed to the frontend: `start_recording`, `stop_recording`, `cancel_recording`, `set_test_mode`, `get_settings`, `save_settings`, `update_hotkey`, `reset_settings`, `get_default_prompt`, `test_polish`, `get_mic_status`, `check_model_status`, `download_model`, `check_llm_model_status`, `download_llm_model`, `save_api_key`, `get_api_key`, `get_history`, `delete_history_entry`, `export_history_audio`, `get_history_storage_path`.

#### `src/polisher.rs` — AI text polishing
- **`PolishConfig`** — fields: `enabled` (default true), `model` (LlamaTaiwan or Qwen25), `output_language`, `custom_prompt` (Option<String>), `mode` (PolishMode: Local or Cloud), `cloud` (CloudConfig), `reasoning` (bool, default false).
- **`CloudConfig`** — fields: `provider` (CloudProvider: Groq/OpenRouter/OpenAi/Gemini/SambaNova/Custom), `api_key` (#[serde(skip)]), `endpoint`, `model_id` (default: "qwen/qwen3-32b").
- **`polish_text`** — dispatches to `run_cloud_inference` (OpenAI-compatible HTTP) or `run_llm_inference` (local llama-cpp-2) based on `PolishMode`.
- LLM model cached in `AppState` and reused across polishes. Lazy-loaded on first use.
- `is_polish_ready()` checks api_key for cloud, model file existence for local.
- **Reasoning toggle**: When `reasoning` is false, `/no_think` is prepended to suppress model reasoning (e.g. Qwen3 `<think>` blocks).

#### `src/context_detect.rs` — App context detection
- NSWorkspace FFI for frontmost app + osascript for browser URLs.
- Captured context fed to LLM prompt for context-aware polishing.

#### `src/history.rs` — Transcription history
- **`HistoryEntry`** — fields: `id`, `timestamp`, `text` (polished), `raw_text`, `stt_model`, `polish_model`, `duration_secs`, `has_audio`.
- Stores transcription history in app data directory. Audio files saved as WAV under `audio/` subdirectory.
- Retention cleanup: deletes entries older than `history_retention_days` setting.

### Frontend (`frontend/`)
Two standalone HTML pages — no framework, no bundler:

- **`index.html`** — settings window with sidebar navigation. Pages: settings (hotkey, language, mic, STT, AI polishing, model management), prompt rules, dictionary, history (transcription log with retention settings), about, test (hotkey test mode). Calls Tauri commands via `window.__TAURI__.core.invoke`.
- **`overlay.html`** — transparent, always-on-top recording indicator capsule. States: `preparing`, `recording`, `processing`, `pasted`, `copied`, `error`. Features real-time 20-bar waveform visualization and elapsed timer with color gradient (orange→red as it approaches 30 s limit). Listens for Tauri events: `recording-status`, `recording-max-duration`, `audio-levels`.
- **`i18n.js`** + **`i18n/`** — internationalization support with locale JSON files (`en.json`, `zh-TW.json`).

### Two Windows
- **`main`** (settings): 960×720 px, hidden by default, shown by tray click or "Settings…" menu item; close button hides rather than quits. `titleBarStyle: "Overlay"` with hidden title.
- **`overlay`**: frameless, transparent, always-on-top, 300×52 px, centered horizontally near the bottom of the screen during recording. Shown/hidden without activating the app via `macos_ffi`.

### Hotkey String Format
Hotkeys are stored as `"Modifier+…+KeyCode"`, e.g. `"Alt+KeyZ"`. Modifiers: `Alt`, `Control`, `Shift`, `Super`. Key codes follow the Web KeyboardEvent `code` property convention (`KeyA`–`KeyZ`, `Digit0`–`Digit9`, `F1`–`F12`, `Space`, `Enter`, etc.).

### Whisper Model
`whisper-rs` (with `metal` feature for GPU acceleration) downloads the Whisper model from HuggingFace on first use. The `WhisperContext` is cached in `AppState` and reused across transcriptions. Model download progress is reported to the frontend via Tauri events.

### LLM Polish Model
`llama-cpp-2` (with `metal` feature for GPU acceleration) downloads GGUF models from HuggingFace. Supported models: Llama 3 Taiwan 8B (Q4_K_M, ~4.6 GB) and Qwen 2.5 7B (Q4_K_M, ~4.6 GB). Model download progress reported via `llm-model-download-progress` Tauri events.

## macOS-Specific Requirements
- `macOSPrivateApi: true` in `tauri.conf.json` is required for `ns_window()` access and transparent windows.
- The app targets macOS; Windows/Linux builds will compile but the overlay and paste features are no-ops on non-macOS (`#[cfg(target_os = "macos")]` gates).
