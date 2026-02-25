# Voxink

![GitHub Release](https://img.shields.io/github/v/release/alan890104/voxink)
![License](https://img.shields.io/github/license/alan890104/voxink)
![GitHub stars](https://img.shields.io/github/stars/alan890104/voxink?style=social)
![GitHub forks](https://img.shields.io/github/forks/alan890104/voxink?style=social)
![Rust](https://img.shields.io/badge/Rust-black?style=flat-square&logo=rust)
![Tauri](https://img.shields.io/badge/Tauri-FFC131?style=flat-square&logo=tauri&logoColor=white)
![macOS](https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white)

English | [繁體中文](README_TW.md)

**Your voice, in writing.**

Voxink is a macOS desktop app that turns your voice into context-aware text. Press a hotkey, speak, and the transcribed text is automatically pasted at your cursor — polished by AI to match the app you're using.

## Features

- **Flexible Speech-to-Text**: Local transcription via [Whisper](https://github.com/openai/whisper) (large-v3-turbo, Metal GPU accelerated) or cloud STT APIs (Groq / OpenAI / Deepgram / Azure / Custom).
- **AI Text Polishing**: Refine transcription with a local LLM (Llama 3 Taiwan 8B / Qwen 2.5 7B via `llama-cpp-2`) or cloud API (Groq / OpenRouter / OpenAI / Gemini / SambaNova / Custom endpoint).
- **Context-Aware**: Detects the frontmost app and browser URL. Custom prompt rules adapt the output per app (e.g. casual in Slack, professional in Gmail, command-style in Terminal).
- **Custom Dictionary**: Add proper nouns, names, or domain terms so the AI always gets them right.
- **Seamless Integration**: Automatically pastes transcribed text at your cursor (simulated Cmd+V).
- **Global Hotkey**: Toggle recording with `Option+Z` (customizable). Press once to start, again to stop and paste.
- **Visual Indicator**: A floating capsule shows real-time waveform, elapsed timer, and status.
- **Transcription History**: Browse and export past transcriptions with optional audio playback.
- **Multilingual UI**: English and Traditional Chinese (zh-TW).
- **Minimalist UI**: Lives in the Menu Bar — stays out of your way.

## Installation

### Download (Recommended)

1. Download the latest DMG from [GitHub Releases](https://github.com/alan890104/voxink/releases/latest).
2. Open the DMG and drag **Voxink** into `/Applications`.
3. Since this app is not notarized by Apple, macOS will flag it. Run in Terminal:

   ```bash
   xattr -cr /Applications/Voxink.app
   ```

4. Launch the app. On first launch it will ask for:
   - **Microphone** access for recording.
   - **Accessibility** permissions (System Settings > Privacy & Security > Accessibility) for auto-paste.

### Build from Source

```bash
git clone https://github.com/alan890104/voxink.git
cd voxink

# Run in development mode
cargo tauri dev

# Build for production (outputs .dmg)
cargo tauri build
```

Requires [Rust](https://rustup.rs/) and [Tauri CLI](https://v2.tauri.app/) (`cargo install tauri-cli --version "^2"`).

## Usage

1. Start the application. You will see an icon in your Menu Bar.
2. Focus any text field where you want to type.
3. Press `Option+Z` (⌥Z) to start recording. A floating indicator appears.
4. Speak naturally (max 30 seconds).
5. Press `Option+Z` again to stop.
6. The transcribed text is pasted at your cursor position.

## Tech Stack

- **Framework**: Tauri v2
- **Backend**: Rust
- **Audio Capture**: `cpal`
- **Speech Recognition**: `whisper-rs` (local, Metal-accelerated) or cloud API (Groq / OpenAI / Deepgram / Azure)
- **AI Polishing**: `llama-cpp-2` (local, Metal-accelerated) or cloud API (OpenAI-compatible)
- **Frontend**: HTML, CSS, JavaScript (no framework, no bundler)

## License

MIT
