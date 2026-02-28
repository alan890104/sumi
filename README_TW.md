# Sumi

![GitHub Release](https://img.shields.io/github/v/release/alan890104/sumi)
![License](https://img.shields.io/github/license/alan890104/sumi)
![GitHub stars](https://img.shields.io/github/stars/alan890104/sumi?style=social)
![GitHub forks](https://img.shields.io/github/forks/alan890104/sumi?style=social)
![Rust](https://img.shields.io/badge/Rust-black?style=flat-square&logo=rust)
![Tauri](https://img.shields.io/badge/Tauri-FFC131?style=flat-square&logo=tauri&logoColor=white)
![macOS](https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white)

[English](README.md) | 繁體中文

**聲音成字。**

Sumi 是一款 macOS 桌面應用程式，將您的語音轉換為符合情境的文字。按下快捷鍵、說話，轉錄後的文字會自動貼上至游標位置 — 並由 AI 根據您正在使用的應用程式進行潤飾。

免費開源。本地優先，內建免費雲端 API。

<!-- TODO: 錄製 demo GIF 後替換
![Sumi Demo](demo.gif)
-->

## 為什麼選 Sumi？

大多數語音聽寫工具需要月費訂閱，且只提供雲端處理。Sumi 讓你自己選擇：

- **本地優先** — Whisper + LLM 完全跑在你 Mac 的 GPU 上（Metal 加速）。使用本地模式時，語音不會離開你的設備 — 程式碼開源，可自行驗證。
- **雲端就緒** — 想要更快的處理速度？自帶 API Key 即可使用 Groq、OpenAI、Deepgram、Azure 等服務。本地和雲端可自由搭配。
- **開源** — GPLv3 授權。免費使用、檢視、修改和貢獻。

### 效果展示

> **原始聽寫**：「嗯那個就是我覺得那個 API 的 endpoint 就是嗯應該要更新一下」
>
> **AI 潤稿後**：「我認為應該更新 API 的 endpoint。」

AI 會根據情境調整語氣 — 在 Slack 用輕鬆語氣，寫 email 用正式語氣，在程式編輯器用技術語氣。

## 競品比較

| | **Sumi** | 系統內建聽寫 | Typeless | Wispr Flow | VoiceInk | SuperWhisper |
|---|---|---|---|---|---|---|
| **價格** | **免費** | 免費 | 每週 4K 字免費, $12-30/月 | 每週 2K 字免費, $12-15/月 | $39.99 | 免費試用, $10/月 |
| **開源** | ✅ GPLv3 | ❌ | ❌ | ❌ | ✅ | ❌ |
| **本地語音辨識** | ✅ Whisper+Metal | ✅ Apple Silicon | ❌ 僅雲端 | ❌ 僅雲端 | ✅ | ✅ |
| **雲端語音辨識** | ✅ 自帶 Key | ❌ | ✅ | ✅ | ✅ 可選 | ✅ |
| **一步完成 AI 潤稿** | ✅ | ❌ 需另開 Writing Tools | ✅ | ✅ | ❌ | ✅ |
| **本地 LLM 潤稿** | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **自訂 Prompt** | ✅ 每個 App 獨立規則 | ❌ 僅固定選項 | ❌ | ✅ | ❌ | ✅ 自訂模式 |
| **情境感知** | ✅ App + URL | ❌ | ✅ App | ✅ App | ❌ | ❌ 手動模式 |
| **自訂詞典** | ✅ | ❌ | ✅ | ✅ | ❌ | ✅ |
| **歷史紀錄** | ✅ 含音訊匯出 | ❌ | ✅ | ❌ | ❌ | ❌ |
| **語音編輯文字** | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ |
| **平台** | macOS | macOS, iOS | macOS, Win, iOS, Android | macOS, Win, iOS, Android | macOS | macOS, Win, iOS |

## 功能特點

- **彈性語音轉文字**：支援本機 [Whisper](https://github.com/openai/whisper) (large-v3-turbo, Metal GPU 加速) 或雲端 STT API (Groq / OpenAI / Deepgram / Azure / 自訂)。
- **AI 內容潤飾**：使用本機 LLM (透過 `llama-cpp-2` 執行 Llama 3 Taiwan 8B / Qwen 2.5 7B) 或雲端 API (Groq / OpenRouter / OpenAI / Gemini / SambaNova / 自訂端點) 精煉轉錄結果。
- **情境感知**：偵測當前最上層的應用程式和瀏覽器 URL。自訂提示規則可針對不同應用程式調整輸出（例如：Slack 用輕鬆語氣、Gmail 用專業語氣、終端機用指令格式）。
- **自訂詞典**：新增人名、地名或專有名詞，AI 會自動辨識並使用正確形式。
- **無縫整合**：自動在游標位置插入轉錄文字（模擬 Cmd+V 貼上）。
- **全域快捷鍵**：使用 `Option+Z`（可自訂）切換錄音。按一次開始，再按一次停止並貼上。
- **視覺化指示器**：浮動膠囊視窗顯示即時波形、計時器和狀態。
- **轉錄歷史紀錄**：瀏覽和匯出過往的轉錄內容，並支援音訊回放。
- **多語系介面**：提供英文和繁體中文 (zh-TW) 介面。
- **極簡 UI**：常駐於選單列 (Menu Bar)，不干擾工作。

## 安裝說明

### 下載安裝（推薦）

1. 從 [GitHub Releases](https://github.com/alan890104/sumi/releases/latest) 下載最新的 DMG 檔案。
2. 打開 DMG，將 **Sumi** 拖曳至 `/Applications`。
3. 由於此應用程式未經 Apple 公證，macOS 會顯示警告。請在終端機執行：

   ```bash
   xattr -cr /Applications/Sumi.app
   ```

4. 啟動應用程式。首次啟動時會要求：
   - **麥克風**存取權限（用於錄音）。
   - **輔助功能**權限（系統設定 > 隱私權與安全性 > 輔助功能），用於自動貼上功能。

### 從原始碼編譯

```bash
git clone https://github.com/alan890104/sumi.git
cd sumi

# 執行開發模式
cargo tauri dev

# 編譯正式版本（輸出 .dmg）
cargo tauri build
```

需要安裝 [Rust](https://rustup.rs/) 和 [Tauri CLI](https://v2.tauri.app/)（`cargo install tauri-cli --version "^2"`）。

## 使用方法

1. 啟動應用程式。您會在選單列 (Menu Bar) 看到圖示。
2. 切換到您想要輸入文字的任何文字欄位。
3. 按下 `Option+Z` (⌥Z) 開始錄音。畫面上會出現浮動指示器。
4. 自然地說話（最長 30 秒）。
5. 再次按下 `Option+Z` 停止錄音。
6. 轉錄後的文字會自動貼在您的游標位置。

## 技術堆疊

- **框架**: Tauri v2
- **後端**: Rust
- **音訊擷取**: `cpal`
- **語音辨識**: `whisper-rs` (本機, Metal 加速) 或雲端 API (Groq / OpenAI / Deepgram / Azure)
- **AI 潤飾**: `llama-cpp-2` (本機, Metal 加速) 或雲端 API (相容 OpenAI 格式)
- **前端**: HTML, CSS, JavaScript（不使用框架或打包工具）

## 授權條款

GPLv3
