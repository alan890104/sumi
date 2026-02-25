# OpenTypeless

OpenTypeless is a minimalist, local-first, speech-to-text desktop application designed for seamless transcription without the need for an internet connection. It stays in your menu bar and allows you to transcribe spoken words directly into any text field with a simple global hotkey.

## Features

- 🎙️ **Local Speech-to-Text**: High-accuracy transcription powered by the [SenseVoice](https://github.com/FunAudioLLM/SenseVoice) model, running entirely on your machine.
- ⚡ **Seamless Integration**: Automatically inserts transcribed text at your current cursor position (auto-paste).
- 🖱️ **Minimalist UI**: Resides in the System Tray (Menu Bar) to keep your workspace clean.
- ⌨️ **Global Hotkey**: Toggle recording instantly with `Option + Command + R`.
- 🟢 **Visual Indicator**: A floating capsule shows real-time status (Recording, Processing, Pasted).
- 🔒 **Privacy Focused**: No data leaves your computer. Period.

## Installation

### Prerequisites

- **Rust**: Install via [rustup.rs](https://rustup.rs/).
- **Tauri Dependencies**: Follow the [Tauri v2 setup guide](https://v2.tauri.app/start/prerequisites/).
- **macOS Permissions**: 
  - The app requires **Microphone** access for recording.
  - The app requires **Accessibility** permissions (System Settings > Privacy & Security > Accessibility) to enable the auto-paste feature.

### Build from Source

```bash
# Clone the repository
git clone https://github.com/alan890104/opentypeless.git
cd opentypeless

# Install dependencies and build
cargo build --release
```

## Usage

1. Start the application. You will see a microphone icon in your Menu Bar.
2. Focus any text field where you want to type.
3. Press `Option + Command + R` (⌥⌘R) to start recording.
4. Speak naturally.
5. Press `Option + Command + R` (⌥⌘R) again to stop.
6. The transcribed text will be automatically typed at your cursor position and copied to your clipboard as a fallback.

## Tech Stack

- **Backend**: Rust, Tauri v2
- **Audio captures**: `cpal`
- **Speech Recognition**: `sensevoice-rs` (C++ backend with Candle/Orthe)
- **Frontend**: HTML5, Vanilla CSS, JavaScript

## License

MIT
