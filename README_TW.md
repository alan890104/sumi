# OpenTypeless

![GitHub Release](https://img.shields.io/github/v/release/alan890104/opentypeless)
![License](https://img.shields.io/github/license/alan890104/opentypeless)
![GitHub stars](https://img.shields.io/github/stars/alan890104/opentypeless?style=social)
![GitHub forks](https://img.shields.io/github/forks/alan890104/opentypeless?style=social)
![Rust](https://img.shields.io/badge/Rust-black?style=flat-square&logo=rust)
![Tauri](https://img.shields.io/badge/Tauri-FFC131?style=flat-square&logo=tauri&logoColor=white)
![macOS](https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white)

[English](README.md) | 繁體中文

OpenTypeless 是一款 macOS 桌面應用程式 (基於 Tauri 2)，透過全域快速鍵提供全系統、完全離線的語音轉文字功能。它使用 Whisper (透過具備 Metal 加速的 `whisper-rs`) 在裝置上進行逐字稿轉錄，並將結果貼上至游標所在處。可選擇使用本機 LLM 或雲端 API 來優化轉錄輸出的文字。

## 功能特點

- **本機語音轉文字**：由 [Whisper](https://github.com/openai/whisper) (large-v3-turbo) 驅動的高準確度轉錄，完全在您的 Mac 上執行，並具備 Metal GPU 加速。
- **AI 內容優化**：可選擇使用本機 LLM (透過 `llama-cpp-2` 執行 Llama 3 Taiwan 8B / Qwen 2.5 7B) 或雲端 API (Groq / OpenRouter / OpenAI / Gemini / 自定義端點) 來精煉轉錄結果。
- **情境感知**：偵測當前最上層的應用程式和瀏覽器 URL，以提供更符合情境的優化結果。
- **無縫整合**：自動在您目前的游標位置插入轉錄文字 (透過模擬 Cmd+V 自動貼上)。
- **全域快速鍵**：使用 `Option+Z` (可自定義) 立即切換錄音。第一次按：開始錄音；第二次按：停止錄音、轉錄並貼上。
- **視覺化指示器**：浮動膠囊視窗顯示即時波形、計時器和狀態 (錄音中、處理中、已貼上、發生錯誤)。
- **轉錄歷史紀錄**：瀏覽、搜尋和匯出過往的轉錄內容，並支援音訊回放。
- **多語系介面**：提供英文和繁體中文 (zh-TW) 介面。
- **隱私優先**：所有轉錄過程皆在裝置本機執行。雲端優化功能為選用，且預設關閉。
- **極簡 UI**：常駐於系統工作列 (Menu Bar)，保持工作空間整潔。

## 安裝說明

### 下載安裝（推薦）

1. 從 [GitHub Releases](https://github.com/alan890104/opentypeless/releases/latest) 下載最新的 DMG 檔案。
2. 打開 DMG，將 **OpenTypeless** 拖曳至 `/Applications`。
3. 由於此應用程式未經 Apple 公證，macOS 會顯示「檔案已損毀」。請在終端機執行以下指令：

   ```bash
   xattr -cr /Applications/OpenTypeless.app
   ```

4. 啟動應用程式。首次啟動時會要求：
   - **麥克風**存取權限（用於錄音）。
   - **輔助功能**權限（系統設定 > 隱私權與安全性 > 輔助功能），用於自動貼上功能。

### 從原始碼編譯

```bash
git clone https://github.com/alan890104/opentypeless.git
cd opentypeless

# 執行開發模式
cargo tauri dev

# 編譯正式版本（輸出 .dmg）
cargo tauri build
```

需要安裝 [Rust](https://rustup.rs/) 和 [Tauri CLI](https://v2.tauri.app/)（`cargo install tauri-cli --version "^2"`）。

Whisper 模型（約 1.5 GB）會在第一次啟動時自動從 HuggingFace 下載。

## 使用方法

1. 啟動應用程式。您會在選單列 (Menu Bar) 看到圖示。
2. 切換到您想要輸入文字的任何文字欄位。
3. 按下 `Option+Z` (⌥Z) 開始錄音。畫面上會出現浮動指示器。
4. 自然地說話 (最長 30 秒)。
5. 再次按下 `Option+Z` 停止錄音。
6. 轉錄後的文字會複製到剪貼簿，並自動貼在您的游標位置。

## 技術堆疊

- **框架**: Tauri v2
- **後端**: Rust
- **音訊擷取**: `cpal`
- **語音辨識**: `whisper-rs` (Metal 加速)
- **AI 優化**: `llama-cpp-2` (本機, Metal 加速) 或 雲端 API (相容於 OpenAI 格式)
- **前端**: HTML, CSS, JavaScript (不使用框架或打包工具)

## 授權條款

MIT
