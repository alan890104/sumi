<div align="center">

# Sumi

<p>
  <a href="https://github.com/alan890104/sumi/releases/latest"><img src="https://img.shields.io/github/v/release/alan890104/sumi?style=flat-square&color=blue" alt="Latest Release"/></a>
  <a href="https://github.com/alan890104/sumi/blob/main/LICENSE"><img src="https://img.shields.io/github/license/alan890104/sumi?style=flat-square" alt="License"/></a>
  <a href="https://github.com/alan890104/sumi/stargazers"><img src="https://img.shields.io/github/stars/alan890104/sumi?style=flat-square" alt="Stars"/></a>
  <img src="https://img.shields.io/badge/Rust-black?style=flat-square&logo=rust" alt="Rust"/>
  <img src="https://img.shields.io/badge/Tauri_v2-FFC131?style=flat-square&logo=tauri&logoColor=white" alt="Tauri"/>
  <img src="https://img.shields.io/badge/Svelte_5-FF3E00?style=flat-square&logo=svelte&logoColor=white" alt="Svelte"/>
  <img src="https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white" alt="macOS"/>
</p>

**System-wide speech-to-text for macOS. Free and open source.**

Press a hotkey anywhere. Speak. The text pastes at your cursor — rewritten by an AI that knows which app you're in.

English | [繁體中文](README_TW.md) | [简体中文](README_CN.md)

<br/>

<table>
<tr>
<td align="center"><img src="assets/demo-gmail.gif" width="280"/><br/><sub><b>Gmail</b> — formats as a proper email</sub></td>
<td align="center"><img src="assets/demo-notion.gif" width="280"/><br/><sub><b>Notion</b> — clean structured prose</sub></td>
<td align="center"><img src="assets/demo-telegram.gif" width="280"/><br/><sub><b>Telegram</b> — casual, conversational</sub></td>
</tr>
</table>

<br/>

```bash
brew tap alan890104/sumi && brew install --cask sumi
```

[Download DMG](https://github.com/alan890104/sumi/releases/latest) · [Releases](https://github.com/alan890104/sumi/releases) · [Issues](https://github.com/alan890104/sumi/issues)

</div>

---

## Why Sumi?

<table>
<tr>
<td width="50%" valign="top">

### 🎯 Per-App AI Polish
Every app gets a different LLM prompt. Slack sounds like Slack. Gmail sounds like email. Terminal gets clean commands. 18 built-in rules — write your own, or describe what you want and the AI generates the rule.

</td>
<td width="50%" valign="top">

### 🔒 Fully Local
Run everything on-device: Whisper or Qwen3-ASR for speech recognition, local LLM for rewriting. Audio never leaves your Mac. You can verify it — the code is here.

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🗣 Speaker Diarization
Meeting mode transcribes continuously in the background. Transcripts are labelled by speaker with timestamps. Import existing audio files for retroactive transcription.

</td>
<td width="50%" valign="top">

### ✏️ Edit by Voice
Select any text, press `Ctrl+Option+Z`, say what you want done. "Make this more formal." "Translate to Japanese." "Shorten it." The AI rewrites and pastes back.

</td>
</tr>
<tr>
<td width="50%" valign="top">

### ☁️ Cloud or Local — Your Choice
BYOK for everything: Groq, OpenAI, Deepgram, Azure, Gemini, OpenRouter, or any OpenAI-compatible endpoint. No Sumi account. No subscription.

</td>
<td width="50%" valign="top">

### 🌏 58 UI Languages
The interface ships in 58 locales. Traditional Chinese users get automatic zh-CN → zh-TW normalization on transcription output.

</td>
</tr>
</table>

---

## Same sentence, three apps

> You say: *"um I think the project is kind of behind schedule and we should probably have a meeting to figure out what to do next"*

<table>
<tr>
<td><b>LINE</b> (casual)</td>
<td>I think the project is behind schedule, we should have a meeting to figure out what to do next</td>
</tr>
<tr>
<td><b>Slack</b> (professional)</td>
<td>I think the project is behind schedule. We should have a meeting to discuss next steps.</td>
</tr>
<tr>
<td><b>Gmail</b> (email)</td>
<td>Hi,<br/><br/>I believe the project is currently behind schedule. Could we schedule a meeting to discuss the next steps?<br/><br/>Best regards</td>
</tr>
</table>

---

## How it works

1. App lives in the menu bar — nothing else on screen.
2. Click into any text field, anywhere on your Mac.
3. Press `Option+Z`. A floating capsule appears with the waveform.
4. Speak.
5. Press `Option+Z` again. Text pastes.

**Edit by Voice:** select text → `Ctrl+Option+Z` → say what to do with it.
**Meeting Mode:** separate hotkey toggles continuous background transcription into a note file.

---

## What it runs on

**Speech recognition** — Local: Whisper (Metal GPU, 7 model sizes from 148 MB to 1.6 GB) or Qwen3-ASR. Cloud: Groq, OpenAI, Deepgram, Azure, any custom endpoint.

**LLM rewriting** — Local: Qwen3-8B, Qwen2.5-7B, Llama 3 Taiwan 8B via candle (Metal/CUDA). Cloud: OpenAI, Groq, Gemini, GitHub Models, OpenRouter, SambaNova, any OpenAI-compatible endpoint.

**Resource usage** — Idle: ~130 MB, 0% CPU. Local transcription: ~730 MB RSS, <20% CPU (Metal). Cloud mode: ~7 MB during recording, back to 0% when done.

**Other details** — Silero VAD for silence filtering · custom pronunciation dictionary · transcription history with audio export · customizable hotkeys

---

## Comparison

> [!NOTE]
> This reflects our best understanding at the time of writing. Competitors update frequently — corrections welcome via issues or PRs.

| | **Sumi** | Built-in Dictation | Typeless | Wispr Flow | VoiceInk | SuperWhisper |
|---|---|---|---|---|---|---|
| **Price** | **Free** | Free | 4K words/wk free, $12-30/mo | 2K words/wk free, $12-15/mo | $25-49 (one-time) | Free trial, ~$8/mo |
| **Open Source** | ✅ GPLv3 | ❌ | ❌ | ❌ | ✅ GPLv3 | ❌ |
| **Local STT** | ✅ | ✅ Apple Silicon | ❌ Cloud only | ❌ Cloud only | ✅ | ✅ |
| **Cloud STT** | ✅ BYOK | ❌ | ✅ | ✅ | ✅ Optional | ✅ |
| **AI Polish** | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| **Local LLM Polish** | ✅ 3 models | ❌ | ❌ | ❌ | ❌ | ✅ |
| **Per-App Rules** | ✅ 18 presets + custom | ❌ | ❌ | ✅ Styles | ✅ Power Modes | ✅ Custom modes |
| **Context-Aware** | ✅ App + URL | ❌ | ✅ App | ✅ App | ✅ App | ✅ Super Mode |
| **Edit by Voice** | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ |
| **Dictionary** | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| **History** | ✅ + audio export | ❌ | ✅ | ✅ | ✅ | ✅ |
| **Meeting notes** | ✅ + speaker labels | ❌ | — | ❌ | — | ✅ |
| **Platforms** | macOS | macOS, iOS | macOS, Win, iOS, Android | macOS, Win, iOS, Android | macOS | macOS, Win, iOS |

---

## Installation

### Homebrew (recommended)

```bash
brew tap alan890104/sumi
brew install --cask sumi
```

### Download DMG

1. Download the latest DMG from [GitHub Releases](https://github.com/alan890104/sumi/releases/latest).
2. Open the DMG, drag Sumi into `/Applications`.
3. The app isn't notarized yet, so macOS will block it. Run this first:
   ```bash
   xattr -cr /Applications/Sumi.app
   ```
4. On first launch: grant Microphone access and enable Accessibility under System Settings → Privacy & Security → Accessibility (required for auto-paste).

### Build from source

Requires [Rust](https://rustup.rs/) and `cargo install tauri-cli --version "^2"`.

```bash
git clone https://github.com/alan890104/sumi.git
cd sumi
cargo tauri dev      # dev mode
cargo tauri build    # production .dmg
```

<details>
<summary>Windows (CUDA)</summary>

Metal is macOS-only. On Windows:

```bash
# CPU only
cargo tauri dev --no-default-features

# NVIDIA CUDA (requires CUDA Toolkit, LLVM, Ninja, CMake)
bash dev-cuda.sh
bash dev-cuda.sh --release
```
</details>

---

## License

[GPLv3](LICENSE)
