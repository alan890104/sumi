# Sumi

![GitHub Release](https://img.shields.io/github/v/release/alan890104/sumi)
![License](https://img.shields.io/github/license/alan890104/sumi)
![GitHub stars](https://img.shields.io/github/stars/alan890104/sumi?style=social)
![GitHub forks](https://img.shields.io/github/forks/alan890104/sumi?style=social)
![Rust](https://img.shields.io/badge/Rust-black?style=flat-square&logo=rust)
![Tauri](https://img.shields.io/badge/Tauri-FFC131?style=flat-square&logo=tauri&logoColor=white)
![macOS](https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white)

English | [繁體中文](README_TW.md)

**Your voice, in writing.**

Sumi is a macOS desktop app that turns your voice into context-aware text. Press a hotkey, speak, and the transcribed text is automatically pasted at your cursor — polished by AI to match the app you're using.

Free and open source. Local-first, with built-in free cloud APIs.

<!-- TODO: Replace with actual demo GIF
![Sumi Demo](demo.gif)
-->

## Why Sumi?

Most voice dictation tools require a monthly subscription and only offer cloud processing. Sumi gives you the choice:

- **Local-First** — Run Whisper + LLM entirely on your Mac's GPU (Metal). In local mode, your audio never leaves your device — verifiable because the code is open source.
- **Cloud-Ready** — Prefer faster processing? Bring your own API keys for Groq, OpenAI, Deepgram, Azure, and more. Mix and match local and cloud freely.
- **Open Source** — GPLv3 licensed. Free to use, inspect, modify, and contribute.

### How It Looks

> **Raw dictation**: "um so like I was thinking that we should um probably update the uh the API endpoint"
>
> **After AI polish**: "I think we should update the API endpoint."

The AI adapts to context — casual in Slack, formal in email, technical in your code editor.

## Comparison

| | **Sumi** | Built-in Dictation | Typeless | Wispr Flow | VoiceInk | SuperWhisper |
|---|---|---|---|---|---|---|
| **Price** | **Free** | Free | 4K words/wk free, $12-30/mo | 2K words/wk free, $12-15/mo | $39.99 | Free trial, $10/mo |
| **Open Source** | ✅ GPLv3 | ❌ | ❌ | ❌ | ✅ | ❌ |
| **Local STT** | ✅ Whisper+Metal | ✅ Apple Silicon | ❌ Cloud only | ❌ Cloud only | ✅ | ✅ |
| **Cloud STT** | ✅ BYOK | ❌ | ✅ | ✅ | ✅ Optional | ✅ |
| **One-Step AI Polish** | ✅ | ❌ Separate Writing Tools | ✅ | ✅ | ❌ | ✅ |
| **Local LLM Polish** | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Custom Prompts** | ✅ Per-app rules | ❌ Fixed options only | ❌ | ✅ | ❌ | ✅ Custom modes |
| **Context-Aware** | ✅ App + URL | ❌ | ✅ App | ✅ App | ❌ | ❌ Manual modes |
| **Custom Dictionary** | ✅ | ❌ | ✅ | ✅ | ❌ | ✅ |
| **Transcription History** | ✅ + audio export | ❌ | ✅ | ❌ | ❌ | ❌ |
| **Voice Editing** | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ |
| **Platforms** | macOS | macOS, iOS | macOS, Win, iOS, Android | macOS, Win, iOS, Android | macOS | macOS, Win, iOS |

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

1. Download the latest DMG from [GitHub Releases](https://github.com/alan890104/sumi/releases/latest).
2. Open the DMG and drag **Sumi** into `/Applications`.
3. Since this app is not notarized by Apple, macOS will flag it. Run in Terminal:

   ```bash
   xattr -cr /Applications/Sumi.app
   ```

4. Launch the app. On first launch it will ask for:
   - **Microphone** access for recording.
   - **Accessibility** permissions (System Settings > Privacy & Security > Accessibility) for auto-paste.

### Build from Source

```bash
git clone https://github.com/alan890104/sumi.git
cd sumi

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

GPLv3
