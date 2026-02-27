# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Sumi is a macOS desktop app (Tauri 2) that provides system-wide speech-to-text via a global hotkey. It supports both local (Whisper via `whisper-rs` with Metal acceleration) and cloud STT APIs (Groq/OpenAI/Deepgram/Azure/Custom) for transcription, and pastes the result at the cursor. Optionally uses a local LLM (via `llama-cpp-2`) or cloud API (Groq/OpenRouter/OpenAI/Gemini/SambaNova/Custom) to polish transcription output. Also supports an "Edit by Voice" mode that applies spoken instructions to selected text via LLM.

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

### Frontend

```bash
# Frontend dev server (started automatically by cargo tauri dev)
# Note: Tauri runs beforeDevCommand from the frontend/ directory
cd frontend && npm run dev

# Frontend type-check
cd frontend && npx svelte-check --tsconfig ./tsconfig.json

# Frontend production build (started automatically by cargo tauri build)
cd frontend && npm run build
```

## Architecture

### Backend

Rust source files (13 modules + platform sub-module):

#### `src/lib.rs` — Core application logic & app setup
- **`AppState`** — shared state managed by Tauri: `is_recording` (AtomicBool), `is_processing` (AtomicBool), `buffer` (Arc<Mutex<Vec<f32>>>), `sample_rate`, `settings`, `mic_available`, `whisper_ctx`, `llm_model`, `captured_context`, `context_override`, `test_mode` (AtomicBool), `voice_rule_mode` (AtomicBool), `last_hotkey_time`, `http_client` (shared reqwest client), `api_key_cache`, `edit_mode` (AtomicBool), `edit_selected_text`, `edit_text_override`, `saved_clipboard`, `vad_ctx` (Silero VAD).
- **Global shortcut handler** — two hotkeys: the main recording toggle (default `Alt+KeyZ`) and edit-by-voice (default `Control+Alt+KeyZ`). Main toggle: first press starts recording + shows overlay; second press stops recording, transcribes, optionally polishes with LLM, copies to clipboard, optionally pastes with Cmd+V, then hides the overlay. Edit-by-voice: copies selected text via Cmd+C, records spoken instruction, applies edit via LLM, pastes result. Max recording duration: 30 seconds (auto-stop).
- Registers all Tauri commands from `commands.rs` and sets up the tray menu, windows, and global shortcuts.

#### `src/settings.rs` — Settings & data directories
- **`Settings`** — persisted to `~/.sumi/config/settings.json`. Fields: `hotkey`, `auto_paste`, `polish` (PolishConfig), `stt` (SttConfig), `history_retention_days` (u32, 0 = keep forever), `language` (Option<String>, UI language override), `edit_hotkey` (Option<String>, default `"Control+Alt+KeyZ"`), `onboarding_completed` (bool).
- **Data directory layout**: `~/.sumi/` with subdirectories: `config/` (settings.json), `models/` (Whisper & LLM GGUF files), `history/` (history.db), `audio/` (WAV files).

#### `src/commands.rs` — Tauri command handlers
All `#[tauri::command]` functions exposed to the frontend:
- **Recording**: `start_recording`, `stop_recording`, `cancel_recording`
- **Mode control**: `set_test_mode`, `set_voice_rule_mode`, `set_context_override`, `set_edit_text_override`
- **Settings**: `get_settings`, `save_settings`, `update_hotkey`, `update_edit_hotkey`, `reset_settings`
- **Polish**: `get_default_prompt`, `get_default_prompt_rules`, `test_polish`, `generate_rule_from_description`
- **Mic**: `get_mic_status`
- **Whisper models**: `check_model_status`, `download_model`, `list_whisper_models`, `get_system_info`, `get_whisper_model_recommendation`, `switch_whisper_model`, `download_whisper_model`
- **LLM models**: `check_llm_model_status`, `download_llm_model`, `list_polish_models`, `switch_polish_model`, `download_polish_model`
- **VAD**: `check_vad_model_status`, `download_vad_model`
- **Credentials**: `save_api_key`, `get_api_key`
- **History**: `get_history`, `get_history_page`, `get_history_stats`, `delete_history_entry`, `clear_all_history`, `export_history_audio`, `get_history_storage_path`
- **Permissions**: `check_permissions`, `open_permission_settings`
- **Utilities**: `get_app_icon`, `trigger_undo`, `copy_image_to_clipboard`

#### `src/stt.rs` — STT configuration
- **`SttConfig`** — fields: `mode` (SttMode: Local or Cloud), `cloud` (SttCloudConfig), `whisper_model` (WhisperModel), `language` (BCP-47 string, "auto" or specific like "zh-TW"), `vad_enabled` (bool, Silero VAD toggle).
- **`SttCloudConfig`** — fields: `provider` (SttProvider: Deepgram/Groq/OpenAi/Azure/Custom), `api_key` (#[serde(skip)]), `endpoint`, `model_id`, `language`.

#### `src/polisher.rs` — AI text polishing
- **`PolishConfig`** — fields: `enabled` (default true), `model` (PolishModel), `custom_prompt` (Option<String>), `mode` (PolishMode: Local or Cloud), `cloud` (CloudConfig), `prompt_rules` (HashMap<String, Vec<PromptRule>>, per-language map), `dictionary` (DictionaryConfig), `reasoning` (bool, default false).
- **`CloudConfig`** — fields: `provider` (CloudProvider: Groq/OpenRouter/OpenAi/Gemini/SambaNova/Custom), `api_key` (#[serde(skip)]), `endpoint`, `model_id` (default: "qwen/qwen3-32b").
- **`PolishModel`** variants: `LlamaTaiwan` (Llama 3 Taiwan 8B, ~4.9 GB), `Qwen25` (Qwen 2.5 7B, ~4.7 GB), `Qwen3` (Qwen 3 8B, ~5.0 GB).
- **`polish_text`** — dispatches to `run_cloud_inference` (OpenAI-compatible HTTP) or `run_llm_inference` (local llama-cpp-2) based on `PolishMode`. Returns `PolishResult { text, reasoning }`.
- **`edit_text_by_instruction`** — "Edit by Voice": takes selected text + spoken instruction, returns edited text via LLM.
- **Prompt rules**: `PromptRule { name, match_type (AppName/BundleId/Url), match_value, prompt, enabled, icon (Option<String>) }`. The `icon` field is an optional key for the frontend (e.g. "terminal", "slack"); auto-detected if None. Built-in preset rules for Gmail, Terminal, VSCode, Cursor, Antigravity, iTerm2, Notion, WhatsApp, Telegram, Slack, Discord, LINE, GitHub, X (Twitter).
- **Dictionary**: `DictionaryConfig { enabled, entries: Vec<DictionaryEntry> }` for proper noun correction, injected into both Whisper initial prompt and LLM system prompt.
- **Reasoning toggle**: When `reasoning` is false, `/no_think` is prepended to suppress model reasoning (e.g. Qwen3 `<think>` blocks).

#### `src/whisper_models.rs` — Multi-model Whisper selection
- **`WhisperModel`** variants: `LargeV3Turbo` (default, 1.62 GB), `LargeV3TurboQ5` (547 MB), `BelleZh` (1.6 GB), `Medium` (1.53 GB), `Small` (488 MB), `Base` (148 MB), `LargeV3TurboZhTw` (1.6 GB).
- **`WhisperModelInfo`** — serializable model metadata for frontend: `id`, `display_name`, `description`, `size_bytes`, `languages`, `downloaded`, `file_size_on_disk`, `is_active`.
- **`SystemInfo`** — `total_ram_bytes`, `available_disk_bytes`, `is_apple_silicon`, `gpu_vram_bytes`, `has_cuda`, `os`, `arch`.
- **`recommend_model`** — smart model recommendation based on system RAM/VRAM/disk/language preference.

#### `src/transcribe.rs` — Whisper transcription & VAD
- **`WhisperContextCache`** — cached `WhisperContext` with loaded model path, reused across transcriptions.
- **`VadContextCache`** — cached Silero VAD context (`ggml-silero-v6.2.0.bin`).
- **`filter_with_vad`** — Silero VAD speech filtering before Whisper transcription.
- **`transcribe_with_cached_whisper`** — accepts `dictionary_terms` for Whisper initial prompt biasing and `app_name` for context-aware prompting.

#### `src/audio.rs` — Audio recording
- **`spawn_audio_thread`** — creates a persistent always-on cpal input stream at app startup. The callback checks `is_recording` atomically and discards samples when false, giving true zero-latency recording start.
- **`try_reconnect_audio`** — auto-reconnect on mic disconnection.
- **`do_start_recording`** — clears the buffer and flips `is_recording` to true (instant, <5 ms).
- **`do_stop_recording`** — flips `is_recording` to false, extracts samples, resamples to 16 kHz. Applies VAD filtering (or RMS trimming fallback). Dispatches to local Whisper or cloud STT based on `SttConfig.mode`.

#### `src/context_detect.rs` — App context detection
- **`AppContext`** — `app_name`, `bundle_id`, `url`, `terminal_host` (original terminal app name when `app_name` was enriched with a CLI tool name; empty when no enrichment occurred).
- NSWorkspace FFI for frontmost app + osascript for browser URLs. Supports Safari, Chrome, Arc, Brave, Microsoft Edge. Terminal subprocess detection enriches `app_name` with CLI tool names.
- Cross-platform: Windows uses `GetForegroundWindow`/`QueryFullProcessImageNameW` FFI.
- Captured context fed to LLM prompt for context-aware polishing.

#### `src/history.rs` — Transcription history (SQLite)
- **`HistoryEntry`** — fields: `id`, `timestamp`, `text` (polished), `raw_text`, `reasoning` (Option), `stt_model`, `polish_model`, `duration_secs`, `has_audio`, `stt_elapsed_ms`, `polish_elapsed_ms` (Option), `total_elapsed_ms`, `app_name`, `bundle_id`, `chars_per_sec`, `word_count` (u64, multilingual via UAX#29 word boundaries).
- **`HistoryStats`** — `total_entries`, `total_duration_secs`, `total_chars`, `local_entries`, `local_duration_secs`, `total_words`.
- SQLite database (`history.db`) with WAL mode. Audio files saved as WAV under `~/.sumi/audio/`.
- Functions: `load_history`, `load_history_page` (paginated), `get_stats`, `add_entry`, `delete_entry`, `clear_all`, `migrate_from_json` (legacy migration).
- Retention cleanup: deletes entries older than `history_retention_days` setting.

#### `src/credentials.rs` — API key storage
- Cross-platform credential storage. macOS: `security` CLI (Keychain). Non-macOS: `keyring` crate (Windows Credential Manager).
- Service name format: `sumi-api-key-{provider}`. Functions: `save`, `load`, `delete`.

#### `src/hotkey.rs` — Hotkey parsing
- `parse_key_code`, `parse_hotkey_string`, `hotkey_display_label` — parsing and display of hotkey strings.

#### `src/permissions.rs` — System permissions
- `check_permissions() -> PermissionStatus { microphone, accessibility }` — checks AVFoundation/AXIsProcessTrusted.
- `open_permission_settings(permission_type)` — opens System Settings pane or triggers microphone access prompt.

#### `src/platform/` — Cross-platform abstraction
Replaces the previous `macos_ffi` module. Sub-modules: `macos.rs`, `windows.rs`, `fallback.rs`.
- `set_app_accessory_mode` — LSUIElement equivalent.
- `setup_overlay_window` — sets window level to NSFloatingWindowLevel, disables `hidesOnDeactivate`, joins all Spaces.
- `show_overlay` / `hide_overlay` — `orderFrontRegardless` / `orderOut:` so the overlay never steals focus.
- `simulate_paste` / `simulate_copy` / `simulate_undo` — CGEvent-based HID simulation.

### Frontend (`frontend/`)
Svelte 5 + TypeScript + Vite. Two Vite entry points (`main.html` + `overlay.html`), each mounting a separate Svelte app. Uses `@tauri-apps/api` ESM imports (`withGlobalTauri: false`). Path alias: `$lib → src/lib`.

- **`src/main/`** — Settings window. Pages: StatsPage (landing/default), SettingsPage, PromptRulesPage, DictionaryPage, HistoryPage, TestWizard, AboutPage. Components: Sidebar, SetupOverlay, ConfirmModal, RuleCard, RuleGridCard, RuleEditorModal, DictEditorModal, HistoryDetailModal, and settings sub-sections (BehaviorSection, LanguageSection, HotkeySection, MicSection, SttSection, PolishSection, DangerZone).
- **`src/overlay/`** — Transparent, always-on-top recording indicator capsule. States: `preparing`, `recording`, `transcribing`, `polishing`, `pasted`, `copied`, `error`, `edited`, `edit_requires_polish`, `processing`, `undo`. Features 20-bar canvas waveform and elapsed timer with color gradient.
- **`src/lib/`** — Shared code: `types.ts` (TypeScript interfaces), `api.ts` (typed Tauri command wrappers), `constants.ts` (provider metadata, key labels, SVG icons), `utils.ts`, `stores/` (Svelte 5 `$state` rune stores for settings, i18n, UI state), `components/` (SettingRow, Toggle, SegmentedControl, Select, Keycaps, Modal, ProgressBar, CloudConfigPanel, InstructionCard, SectionHeader).
- **`src/i18n/`** — 58 locale JSON files (af, ar, az, be, bg, bs, ca, cs, cy, da, de, el, en, es, et, fa, fi, fr, gl, he, hi, hr, hu, hy, id, is, it, ja, kk, kn, ko, lt, lv, mi, mk, mr, ms, ne, nl, no, pl, pt, ro, ru, sk, sl, sr, sv, sw, ta, th, tl, tr, uk, ur, vi, zh-CN, zh-TW), statically imported by the i18n store.

### Two Windows
- **`main`** (settings): 1120×800 px, hidden by default, shown by tray click or "Settings…" menu item; close button hides rather than quits. `titleBarStyle: "Overlay"` with hidden title. Default page is StatsPage.
- **`overlay`**: frameless, transparent, always-on-top, 300×52 px, centered horizontally near the bottom of the screen during recording. Shown/hidden without activating the app via `platform` module.

### Hotkey String Format
Hotkeys are stored as `"Modifier+…+KeyCode"`, e.g. `"Alt+KeyZ"`. Modifiers: `Alt`, `Control`, `Shift`, `Super`. Key codes follow the Web KeyboardEvent `code` property convention (`KeyA`–`KeyZ`, `Digit0`–`Digit9`, `F1`–`F12`, `Space`, `Enter`, etc.).

### Whisper Model
`whisper-rs` (with `metal` feature for GPU acceleration) downloads Whisper models from HuggingFace on first use. 7 model variants available with smart system-based recommendation. The `WhisperContext` is cached in `AppState` and reused across transcriptions. Model download progress is reported to the frontend via Tauri events.

### Silero VAD
Optional Silero VAD model (`ggml-silero-v6.2.0.bin`) downloaded separately. Filters out non-speech segments before Whisper transcription. Falls back to RMS-based silence trimming if not downloaded. Controlled by `stt.vad_enabled`.

### LLM Polish Model
`llama-cpp-2` (with `metal` feature for GPU acceleration) downloads GGUF models from HuggingFace. Supported models: Llama 3 Taiwan 8B (Q4_K_M, ~4.9 GB), Qwen 2.5 7B (Q4_K_M, ~4.7 GB), and Qwen 3 8B (Q4_K_M, ~5.0 GB). Model download progress reported via `llm-model-download-progress` Tauri events. Multi-model management with per-model download/switch.

## macOS-Specific Requirements
- `macOSPrivateApi: true` in `tauri.conf.json` is required for `ns_window()` access and transparent windows.
- The app targets macOS primarily; Windows support is implemented via the `platform/` module and `#[cfg(target_os)]` gates. Linux builds compile but overlay and paste features are no-ops.
