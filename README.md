# Sumi

![GitHub Release](https://img.shields.io/github/v/release/alan890104/sumi)
![License](https://img.shields.io/github/license/alan890104/sumi)
![GitHub stars](https://img.shields.io/github/stars/alan890104/sumi?style=social)
![GitHub forks](https://img.shields.io/github/forks/alan890104/sumi?style=social)
![Rust](https://img.shields.io/badge/Rust-black?style=flat-square&logo=rust)
![Tauri](https://img.shields.io/badge/Tauri_v2-FFC131?style=flat-square&logo=tauri&logoColor=white)
![Svelte](https://img.shields.io/badge/Svelte_5-FF3E00?style=flat-square&logo=svelte&logoColor=white)
![macOS](https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white)

English | [繁體中文](README_TW.md)

**Voice-to-text that adapts to what you're doing.**

Sumi is a macOS app that transcribes your speech and polishes it with AI — automatically adjusting tone and style based on the app you're in. Casual in LINE, professional in Slack, formal email format in Gmail. You define the rules, or let Sumi's built-in presets handle it.

Free, open source, and local-first.

<!-- TODO: Replace with actual demo GIF
![Sumi Demo](demo.gif)
-->

## Why Sumi?

### Per-app rules you can customize

Most voice-to-text tools produce the same output everywhere. Sumi detects the frontmost app and URL, then applies prompt rules that shape the AI's output. It ships with 18 built-in presets (Gmail, Slack, Discord, GitHub, VSCode, Terminal, and more), and you can create your own for any app — even by just describing what you want in natural language and letting the LLM generate the rule for you.

**Same speech, different output:**

> You say: *"um I think the project is kind of behind schedule and we should probably have a meeting to figure out what to do next"*
>
> **In LINE** (chat — casual, natural, may add emoji):
> I think the project is behind schedule, we should have a meeting to figure out what to do next
>
> **In Slack** (professional but approachable, concise):
> I think the project is behind schedule. We should have a meeting to discuss next steps.
>
> **In Gmail** (formal email format with greeting/body/sign-off):
> Hi,
>
> I believe the project is currently behind schedule. Could we schedule a meeting to discuss the next steps?
>
> Best regards

### Local-first privacy

Run Whisper + LLM entirely on your Mac's GPU (Metal accelerated). In local mode, your audio and text never leave your device — verifiable because the code is open source.

### Free & open source

GPLv3 licensed. No subscription, no word limits, no account required. Bring your own API keys for cloud providers if you want faster processing, or use local models for free.

## Features

### Context-Aware AI

- **Per-app prompt rules** — 18 built-in presets covering email (Gmail), chat (Slack, Discord, WhatsApp, Telegram, LINE), code editors (VSCode, Cursor, Antigravity), terminals (Terminal, iTerm2), AI CLI tools (Claude Code, Gemini CLI, Codex CLI, Aider), docs (Notion), developer platforms (GitHub), and social media (X/Twitter). Rules match by app name, bundle ID, or URL.
- **Multi-match rules** — A single rule can match multiple conditions (e.g. Slack desktop app OR `app.slack.com` in browser).
- **Create rules with your voice** — Describe what you want in natural language; the LLM generates the structured rule for you.
- **Edit by Voice** — Select text, press `Ctrl+Option+Z`, speak an instruction ("translate to English", "make it more formal"), and the AI rewrites the selection in place.
- **Custom dictionary** — Add proper nouns, names, or domain terms so the AI always gets them right. Dictionary terms are injected into both Whisper and LLM prompts.

### Speech-to-Text

- **Local Whisper** — 7 model variants (large-v3-turbo default, quantized lite, Chinese-tuned, medium, small, base) with Metal GPU acceleration via `whisper-rs`.
- **Cloud STT** — Bring your own API keys for Groq, OpenAI, Deepgram, Azure, or any custom endpoint.
- **Silero VAD** — Optional voice activity detection filters out silence and non-speech before transcription.
- **Zero-latency start** — Audio stream runs continuously; recording starts by flipping an atomic flag with no stream initialization delay.

### AI Polish

- **Local LLM** — 3 models via `candle` with Metal/CUDA acceleration: Llama 3 Taiwan 8B (~4.9 GB), Qwen 2.5 7B (~4.7 GB), Qwen 3 8B (~5.0 GB).
- **Cloud LLM** — Groq, OpenRouter, OpenAI, Gemini, GitHub Models, SambaNova, or any OpenAI-compatible endpoint.
- **Reasoning toggle** — Enable/disable model thinking (e.g. Qwen 3 `<think>` blocks) per your preference.

### UX

- **Global hotkey** — `Option+Z` to toggle recording (customizable). Press once to start, again to stop and paste.
- **Floating overlay** — Transparent always-on-top capsule with real-time waveform, elapsed timer, and status.
- **Auto-paste** — Transcribed text is pasted at your cursor via simulated `Cmd+V`.
- **Transcription history** — Browse past transcriptions with audio playback and export.
- **58 languages** — UI available in 58 languages including English, Chinese, Japanese, Korean, Spanish, French, German, and many more.
- **Menu bar app** — Lives in the menu bar, stays out of your way.

## Comparison

> [!NOTE]
> This table reflects our best understanding as of the time of writing. Competitors update their features frequently — corrections are welcome via issues or PRs.

| | **Sumi** | Built-in Dictation | Typeless | Wispr Flow | VoiceInk | SuperWhisper |
|---|---|---|---|---|---|---|
| **Price** | **Free** | Free | 4K words/wk free, $12-30/mo | 2K words/wk free, $12-15/mo | $25-49 (one-time) | Free trial, ~$8/mo |
| **Open Source** | ✅ GPLv3 | ❌ | ❌ | ❌ | ✅ GPLv3 | ❌ |
| **Local STT** | ✅ Whisper+Metal | ✅ Apple Silicon | ❌ Cloud only | ❌ Cloud only | ✅ | ✅ |
| **Cloud STT** | ✅ BYOK | ❌ | ✅ | ✅ | ✅ Optional | ✅ |
| **AI Polish** | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| **Local LLM Polish** | ✅ 3 models | ❌ | ❌ | ❌ | ❌ | ✅ |
| **Per-App Rules** | ✅ 18 presets + custom | ❌ | ❌ | ✅ Styles | ✅ Power Modes | ✅ Custom modes |
| **Context-Aware** | ✅ App + URL | ❌ | ✅ App | ✅ App | ✅ App | ✅ Super Mode |
| **Edit by Voice** | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ |
| **Dictionary** | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| **History** | ✅ + audio export | ❌ | ✅ | ✅ | ✅ | ✅ |
| **Platforms** | macOS | macOS, iOS | macOS, Win, iOS, Android | macOS, Win, iOS, Android | macOS | macOS, Win, iOS |

## Installation

### Homebrew

```bash
brew tap alan890104/sumi
brew install --cask sumi
```

### Download

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

**Edit by Voice:** Select text, then press `Ctrl+Option+Z` (⌃⌥Z). Speak your instruction (e.g. "translate to Japanese"), and the AI will rewrite the selected text accordingly.

## Tech Stack

- **Framework**: Tauri v2
- **Backend**: Rust
- **Frontend**: Svelte 5 + TypeScript + Vite
- **Audio Capture**: `cpal`
- **Speech Recognition**: `whisper-rs` (local, Metal-accelerated) or cloud API (Groq / OpenAI / Deepgram / Azure)
- **AI Polishing**: `candle` (local, Metal/CUDA-accelerated) or cloud API (OpenAI-compatible)
- **Voice Activity Detection**: Silero VAD

## License

GPLv3
