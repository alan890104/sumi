# Sumi

![GitHub Release](https://img.shields.io/github/v/release/alan890104/sumi)
![License](https://img.shields.io/github/license/alan890104/sumi)
![GitHub stars](https://img.shields.io/github/stars/alan890104/sumi?style=social)
![GitHub forks](https://img.shields.io/github/forks/alan890104/sumi?style=social)
![Rust](https://img.shields.io/badge/Rust-black?style=flat-square&logo=rust)
![Tauri](https://img.shields.io/badge/Tauri_v2-FFC131?style=flat-square&logo=tauri&logoColor=white)
![Svelte](https://img.shields.io/badge/Svelte_5-FF3E00?style=flat-square&logo=svelte&logoColor=white)
![macOS](https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white)

[English](README.md) | 繁體中文

**懂情境的語音輸入。**

Sumi 是一款 macOS 語音轉文字工具，說完話 AI 會自動幫你潤稿，而且會根據你當下用的 App 調整語氣 — 在 LINE 口語輕鬆、在 Slack 專業簡潔、在 Gmail 自動排成正式信件。規則可以自己定，也可以用內建的。

免費、開源、本地優先。

<!-- TODO: 錄製 demo GIF 後替換
![Sumi Demo](demo.gif)
-->

## 為什麼選 Sumi？

### 依 App 自訂規則

大多數語音轉文字工具不管你在哪個 App，出來的文字都長一樣。Sumi 會偵測你正在用的 App 和網址，自動套用對應的規則來調整 AI 的輸出。內建 18 組預設規則（Gmail、Slack、Discord、GitHub、VSCode、Terminal 等），你也可以幫任何 App 建立自己的規則 — 甚至只要口述你想要什麼效果，AI 就會自動幫你產生規則。

**同一段話，不同輸出：**

> 你說：*「嗯就是我覺得這個專案的進度有點落後，我們需要開個會討論一下接下來要怎麼做」*
>
> **在 LINE**（聊天 — 輕鬆自然，可能加 emoji）：
> 我覺得專案進度有點落後，我們開個會討論一下接下來怎麼做吧
>
> **在 Slack**（專業但親切，簡潔）：
> 我覺得專案進度有些落後，我們需要開個會討論接下來的計畫。
>
> **在 Gmail**（正式信件格式，含問候語、正文、結尾）：
> 您好，
>
> 我注意到目前專案進度略有落後，想請大家安排一次會議，討論接下來的工作規劃。期待您的回覆。

### 隱私優先，資料不出你的電腦

Whisper + LLM 完全跑在你 Mac 的 GPU 上（Metal 加速）。用本地模式的話，語音和文字完全不會離開你的裝置 — 程式碼全部開源，可以自己驗證。

### 免費且開源

GPLv3 授權。不用訂閱、不限字數、不需要帳號。想要更快可以接雲端 API Key，也可以純用本地模型，完全免費。

## 功能特點

### 情境感知 AI

- **依 App 套用規則** — 內建 18 組預設規則，涵蓋信件（Gmail）、聊天（Slack、Discord、WhatsApp、Telegram、LINE）、程式編輯器（VSCode、Cursor、Antigravity）、終端機（Terminal、iTerm2）、AI CLI 工具（Claude Code、Gemini CLI、Codex CLI、Aider）、筆記（Notion）、開發平台（GitHub）、社群（X/Twitter）。可以用 App 名稱、Bundle ID 或 URL 來比對。
- **多重比對** — 一條規則可以同時符合多個條件（例如 Slack 桌面 App 和瀏覽器的 `app.slack.com` 共用同一條規則）。
- **用說的建立規則** — 口述你想要的效果，AI 會自動產生結構化的規則。
- **語音編輯** — 選取文字後按 `Ctrl+Option+Z`，說出指令（「翻譯成英文」、「語氣改正式一點」），AI 會直接改寫你選的文字。
- **自訂詞典** — 加入人名、地名或專業術語，語音辨識和 AI 潤飾都會用到這些詞彙。

### 語音轉文字

- **本地 Whisper** — 7 種模型可選（預設 large-v3-turbo、量化輕量版、中文特化版、medium、small、base），透過 `whisper-rs` 用 Metal GPU 加速。
- **雲端 STT** — 自帶 API Key 就能用 Groq、OpenAI、Deepgram、Azure 或任何自訂端點。
- **Silero VAD** — 選配的語音活動偵測，錄音中的靜音和雜音會在轉錄前自動過濾。
- **零延遲錄音** — 音訊串流隨時待命，按下錄音的瞬間就開始收音，沒有啟動延遲。

### AI 潤飾

- **本地 LLM** — 透過 `llama-cpp-2` 搭配 Metal 加速，提供 3 種模型：Llama 3 Taiwan 8B（~4.9 GB）、Qwen 2.5 7B（~4.7 GB）、Qwen 3 8B（~5.0 GB）。
- **雲端 LLM** — Groq、OpenRouter、OpenAI、Gemini、GitHub Models、SambaNova，或任何 OpenAI 相容端點。
- **思考模式開關** — 可以開啟或關閉模型的推理過程（例如 Qwen 3 的 `<think>` 區塊）。

### 使用體驗

- **全域快捷鍵** — 按 `Option+Z` 開始錄音，再按一次停止並貼上（快捷鍵可自訂）。
- **浮動指示器** — 透明的浮動膠囊，顯示即時波形、計時和狀態。
- **自動貼上** — 轉完的文字會自動貼在游標位置。
- **轉錄歷史** — 瀏覽過去的轉錄紀錄，支援音訊回放和匯出。
- **58 種語言** — 介面支援 58 種語言，包含中文、英文、日文、韓文、西班牙文、法文、德文等。
- **常駐選單列** — 安靜待在選單列，不佔畫面。

## 競品比較

> [!NOTE]
> 此表為撰寫當時的資訊，各產品功能可能隨時更新，歡迎透過 Issue 或 PR 更正。

| | **Sumi** | 系統內建聽寫 | Typeless | Wispr Flow | VoiceInk | SuperWhisper |
|---|---|---|---|---|---|---|
| **價格** | **免費** | 免費 | 每週 4K 字免費, $12-30/月 | 每週 2K 字免費, $12-15/月 | $25-49（買斷） | 免費試用, ~$8/月 |
| **開源** | ✅ GPLv3 | ❌ | ❌ | ❌ | ✅ GPLv3 | ❌ |
| **本地語音辨識** | ✅ Whisper+Metal | ✅ Apple Silicon | ❌ 僅雲端 | ❌ 僅雲端 | ✅ | ✅ |
| **雲端語音辨識** | ✅ 自帶 Key | ❌ | ✅ | ✅ | ✅ 可選 | ✅ |
| **AI 潤飾** | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| **本地 LLM 潤飾** | ✅ 3 種模型 | ❌ | ❌ | ❌ | ❌ | ✅ |
| **依 App 規則** | ✅ 18 預設 + 自訂 | ❌ | ❌ | ✅ Styles | ✅ Power Modes | ✅ 自訂模式 |
| **情境感知** | ✅ App + URL | ❌ | ✅ App | ✅ App | ✅ App | ✅ Super Mode |
| **語音編輯文字** | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ |
| **詞典** | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| **歷史紀錄** | ✅ 含音訊匯出 | ❌ | ✅ | ✅ | ✅ | ✅ |
| **平台** | macOS | macOS, iOS | macOS, Win, iOS, Android | macOS, Win, iOS, Android | macOS | macOS, Win, iOS |

## 安裝

### 下載安裝（推薦）

1. 從 [GitHub Releases](https://github.com/alan890104/sumi/releases/latest) 下載最新的 DMG。
2. 打開 DMG，把 **Sumi** 拖進 `/Applications`。
3. 這個 App 沒有經過 Apple 公證，macOS 會擋下來。在終端機執行：

   ```bash
   xattr -cr /Applications/Sumi.app
   ```

4. 打開 App，第一次啟動會要求：
   - **麥克風**權限（錄音用）。
   - **輔助功能**權限（系統設定 > 隱私權與安全性 > 輔助功能），用來自動貼上。

### 從原始碼編譯

```bash
git clone https://github.com/alan890104/sumi.git
cd sumi

# 開發模式
cargo tauri dev

# 正式編譯（輸出 .dmg）
cargo tauri build
```

需要 [Rust](https://rustup.rs/) 和 [Tauri CLI](https://v2.tauri.app/)（`cargo install tauri-cli --version "^2"`）。

## 使用方法

1. 打開 App，選單列會出現 Sumi 圖示。
2. 點一下你想打字的地方。
3. 按 `Option+Z`（⌥Z）開始錄音，畫面會出現浮動指示器。
4. 自然地說話（最長 30 秒）。
5. 再按一次 `Option+Z` 停止。
6. 文字會自動貼在游標位置。

**語音編輯：** 先選取文字，再按 `Ctrl+Option+Z`（⌃⌥Z），說出指令（例如「翻譯成日文」），AI 就會直接改寫。

## 技術堆疊

- **框架**: Tauri v2
- **後端**: Rust
- **前端**: Svelte 5 + TypeScript + Vite
- **音訊擷取**: `cpal`
- **語音辨識**: `whisper-rs`（本機, Metal 加速）或雲端 API（Groq / OpenAI / Deepgram / Azure）
- **AI 潤飾**: `llama-cpp-2`（本機, Metal 加速）或雲端 API（相容 OpenAI 格式）
- **語音活動偵測**: Silero VAD

## 授權條款

GPLv3
