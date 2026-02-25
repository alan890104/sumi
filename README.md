# OpenTypeless

![GitHub Release](https://img.shields.io/github/v/release/alan890104/opentypeless)
![License](https://img.shields.io/github/license/alan890104/opentypeless)
![GitHub stars](https://img.shields.io/github/stars/alan890104/opentypeless?style=social)
![GitHub forks](https://img.shields.io/github/forks/alan890104/opentypeless?style=social)
![Rust](https://img.shields.io/badge/Rust-black?style=flat-square&logo=rust)
![Tauri](https://img.shields.io/badge/Tauri-FFC131?style=flat-square&logo=tauri&logoColor=white)
![macOS](https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white)

English | [繁體中文](README_TW.md)

OpenTypeless is a macOS desktop app (Tauri 2) that provides system-wide, fully-offline speech-to-text via a global hotkey. It uses Whisper (via `whisper-rs` with Metal acceleration) for on-device transcription and pastes the result at the cursor. Optionally uses a local LLM or cloud API to polish transcription output.

## Features

- **Local Speech-to-Text**: High-accuracy transcription powered by [Whisper](https://github.com/openai/whisper) (large-v3-turbo), running entirely on your Mac with Metal GPU acceleration.
- **AI Text Polishing**: Optionally refine transcription output using a local LLM (Llama 3 Taiwan 8B / Qwen 2.5 7B via `llama-cpp-2`) or cloud API (Groq / OpenRouter / OpenAI / Gemini / Custom endpoint).
- **Context-Aware**: Detects the frontmost app and browser URL to provide context-aware polishing.
- **Seamless Integration**: Automatically inserts transcribed text at your current cursor position (auto-paste via simulated Cmd+V).
- **Global Hotkey**: Toggle recording instantly with `Option+Z` (customizable). First press starts recording; second press stops, transcribes, and pastes.
- **Visual Indicator**: A floating capsule shows real-time waveform, elapsed timer, and status (Recording, Processing, Pasted, Error).
- **Transcription History**: Browse, search, and export past transcriptions with optional audio playback.
- **Multilingual UI**: English and Traditional Chinese (zh-TW) interface.
- **Privacy First**: All transcription runs locally on-device. Cloud polishing is optional and off by default.
- **Minimalist UI**: Resides in the System Tray (Menu Bar) to keep your workspace clean.

## Installation

### Prerequisites

- **macOS** (Apple Silicon or Intel)
- **Rust**: Install via [rustup.rs](https://rustup.rs/).
- **Tauri CLI**: `cargo install tauri-cli --version "^2"`
- **macOS Permissions**:
  - **Microphone** access for recording.
  - **Accessibility** permissions (System Settings > Privacy & Security > Accessibility) for the auto-paste feature.

### Build from Source

```bash
git clone https://github.com/alan890104/opentypeless.git
cd opentypeless

# Run in development mode
cargo tauri dev

# Build for production (outputs .dmg)
cargo tauri build
```

The Whisper model (~1.5 GB) is downloaded automatically from HuggingFace on first launch.

## Usage

1. Start the application. You will see an icon in your Menu Bar.
2. Focus any text field where you want to type.
3. Press `Option+Z` (⌥Z) to start recording. A floating indicator appears.
4. Speak naturally (max 30 seconds).
5. Press `Option+Z` again to stop.
6. The transcribed text is copied to your clipboard and automatically pasted at your cursor position.

## Tech Stack

- **Framework**: Tauri v2
- **Backend**: Rust
- **Audio Capture**: `cpal`
- **Speech Recognition**: `whisper-rs` (Metal-accelerated)
- **AI Polishing**: `llama-cpp-2` (local, Metal-accelerated) or cloud API (OpenAI-compatible)
- **Frontend**: HTML, CSS, JavaScript (no framework, no bundler)

## License

MIT
