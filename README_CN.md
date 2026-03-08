<div align="center">

# Sumi

<p>
  <a href="https://github.com/alan890104/sumi/releases/latest"><img src="https://img.shields.io/github/v/release/alan890104/sumi?style=flat-square&color=blue" alt="最新版本"/></a>
  <a href="https://github.com/alan890104/sumi/blob/main/LICENSE"><img src="https://img.shields.io/github/license/alan890104/sumi?style=flat-square" alt="许可证"/></a>
  <a href="https://github.com/alan890104/sumi/stargazers"><img src="https://img.shields.io/github/stars/alan890104/sumi?style=flat-square" alt="Stars"/></a>
  <img src="https://img.shields.io/badge/Rust-black?style=flat-square&logo=rust" alt="Rust"/>
  <img src="https://img.shields.io/badge/Tauri_v2-FFC131?style=flat-square&logo=tauri&logoColor=white" alt="Tauri"/>
  <img src="https://img.shields.io/badge/Svelte_5-FF3E00?style=flat-square&logo=svelte&logoColor=white" alt="Svelte"/>
  <img src="https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white" alt="macOS"/>
</p>

**macOS 全局语音输入工具。免费开源。**

在任何地方按下快捷键、说话，文字就粘贴到光标位置 — 而且 AI 会根据你当下打开的 App 自动调整改写方式。

[English](README.md) | [繁體中文](README_TW.md) | 简体中文

<br/>

<table>
<tr>
<td align="center" valign="middle" width="33%"><img src="assets/demo-gmail.gif" width="280"/></td>
<td align="center" valign="middle" width="33%"><img src="assets/demo-notion.gif" width="280"/></td>
<td align="center" valign="middle" width="33%"><img src="assets/demo-telegram.gif" width="280"/></td>
</tr>
<tr>
<td align="center"><sub><b>Gmail</b> — 自动排版成邮件格式</sub></td>
<td align="center"><sub><b>Notion</b> — 整洁的结构化文章</sub></td>
<td align="center"><sub><b>Telegram</b> — 轻松自然的口语</sub></td>
</tr>
</table>

<br/>

```bash
brew tap alan890104/sumi && brew install --cask sumi
```

[下载 DMG](https://github.com/alan890104/sumi/releases/latest) · [所有版本](https://github.com/alan890104/sumi/releases) · [反馈问题](https://github.com/alan890104/sumi/issues)

</div>

---

## 为什么选 Sumi？

<table>
<tr>
<td width="50%" valign="top">

### 🎯 按 App 套用 AI 规则
每个 App 各自有不同的 LLM 提示词。Slack 有 Slack 的语气，Gmail 写成邮件格式，终端机输出干净的命令。内置 18 条规则，可以自己写，或直接用白话描述，AI 帮你生成。

</td>
<td width="50%" valign="top">

### 🔒 完全本地运行
语音识别（Whisper 或 Qwen3-ASR）和 LLM 改写都可以在设备上跑，音频不会离开你的 Mac。代码都在这里，可以自己验证。

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🗣 说话者分离
会议模式在后台持续转录，文字记录标注说话者与时间戳。也可以导入现有音频文件进行事后转录。

</td>
<td width="50%" valign="top">

### ✏️ 语音编辑文字
选中任何文字，按 `Ctrl+Option+Z`，说你想怎么改。"改得更正式一点""翻成英文""缩短它"。AI 改写后自动粘贴回去。

</td>
</tr>
<tr>
<td width="50%" valign="top">

### ☁️ 云端或本地，自由选择
所有服务都带自己的 Key：Groq、OpenAI、Deepgram、Azure、Gemini、OpenRouter，或任何兼容 OpenAI 格式的端点。没有 Sumi 账号，没有订阅费。

</td>
<td width="50%" valign="top">

### 🌏 58 种界面语言
界面支持 58 种语系。繁体中文用户转录输出时，会自动将 zh-CN 规范化为 zh-TW。

</td>
</tr>
</table>

---

## 同一段话，三个 App

> 你说：*"嗯就是我觉得这个项目的进度有点落后，我们需要开个会讨论一下接下来要怎么做"*

<table>
<tr>
<td><b>LINE</b>（轻松随意）</td>
<td>我觉得项目进度有点落后，我们开个会讨论一下接下来怎么做吧</td>
</tr>
<tr>
<td><b>Slack</b>（专业简洁）</td>
<td>我觉得项目进度有些落后，我们需要开个会讨论接下来的计划。</td>
</tr>
<tr>
<td><b>Gmail</b>（邮件格式）</td>
<td>您好，<br/><br/>我注意到目前项目进度略有落后，想请大家安排一次会议，讨论接下来的工作规划。期待您的回复。</td>
</tr>
</table>

---

## 怎么用

1. 打开 App，它住在菜单栏，没有别的东西。
2. 在 Mac 上任何文字输入框点一下。
3. 按 `Option+Z`，画面出现带波形的浮动胶囊。
4. 说话。
5. 再按一次 `Option+Z`，文字粘贴上去。

**语音编辑：** 选中文字 → `Ctrl+Option+Z` → 说你想怎么改。
**会议模式：** 另一组快捷键切换后台持续转录，存成笔记文件。

---

## 运行什么东西

**语音识别** — 本地：Whisper（Metal GPU，7 种模型大小，148 MB～1.6 GB）或 Qwen3-ASR。云端：Groq、OpenAI、Deepgram、Azure，或任何自定义端点。

**LLM 改写** — 本地：Qwen3-8B、Qwen2.5-7B、Llama 3 Taiwan 8B，通过 candle 跑 Metal/CUDA。云端：OpenAI、Groq、Gemini、GitHub Models、OpenRouter、SambaNova，或任何兼容 OpenAI 格式的端点。

**资源使用** — 待机：约 130 MB、0% CPU。本地转录：RSS 升至约 730 MB、CPU <20%（Metal）。云端模式：录音期间多约 7 MB，传完立刻归零。

**其他细节** — Silero VAD 静音过滤 · 自定义发音词典 · 转录历史含音频导出 · 快捷键可自定义

---

## 竞品对比

> [!NOTE]
> 此表为撰写时的信息，各产品功能可能随时更新，欢迎通过 Issue 或 PR 更正。

| | **Sumi** | 系统内置听写 | Typeless | Wispr Flow | VoiceInk | SuperWhisper |
|---|---|---|---|---|---|---|
| **价格** | **免费** | 免费 | 每周 4K 字免费, $12-30/月 | 每周 2K 字免费, $12-15/月 | $25-49（买断） | 免费试用, ~$8/月 |
| **开源** | ✅ GPLv3 | ❌ | ❌ | ❌ | ✅ GPLv3 | ❌ |
| **本地语音识别** | ✅ | ✅ Apple Silicon | ❌ 仅云端 | ❌ 仅云端 | ✅ | ✅ |
| **云端语音识别** | ✅ 自带 Key | ❌ | ✅ | ✅ | ✅ 可选 | ✅ |
| **AI 润色** | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| **本地 LLM 润色** | ✅ 3 种模型 | ❌ | ❌ | ❌ | ❌ | ✅ |
| **按 App 规则** | ✅ 18 预设 + 自定义 | ❌ | ❌ | ✅ Styles | ✅ Power Modes | ✅ 自定义模式 |
| **情境感知** | ✅ App + URL | ❌ | ✅ App | ✅ App | ✅ App | ✅ Super Mode |
| **语音编辑文字** | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ |
| **词典** | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| **历史记录** | ✅ 含音频导出 | ❌ | ✅ | ✅ | ✅ | ✅ |
| **会议记录** | ✅ + 说话者标注 | ❌ | — | ❌ | — | ✅ |
| **平台** | macOS | macOS, iOS | macOS, Win, iOS, Android | macOS, Win, iOS, Android | macOS | macOS, Win, iOS |

---

## 安装

### Homebrew（推荐）

```bash
brew tap alan890104/sumi
brew install --cask sumi
```

### 下载 DMG

1. 从 [GitHub Releases](https://github.com/alan890104/sumi/releases/latest) 下载最新的 DMG。
2. 打开 DMG，把 Sumi 拖进 `/Applications`。
3. 这个 App 还没有 Apple 公证，macOS 第一次会拦住。先运行这个：
   ```bash
   xattr -cr /Applications/Sumi.app
   ```
4. 第一次打开：给麦克风权限，然后到系统设置 → 隐私与安全 → 辅助功能 开启 Sumi（自动粘贴功能需要此权限）。

### 从源码编译

需要 [Rust](https://rustup.rs/) 和 `cargo install tauri-cli --version "^2"`。

```bash
git clone https://github.com/alan890104/sumi.git
cd sumi
cargo tauri dev      # 开发模式
cargo tauri build    # 正式编译（输出 .dmg）
```

<details>
<summary>Windows（CUDA）</summary>

Metal 是 macOS 专属的，在 Windows 上需要关掉：

```bash
# 纯 CPU
cargo tauri dev --no-default-features

# 搭配 NVIDIA CUDA（需要 CUDA Toolkit、LLVM、Ninja、CMake）
bash dev-cuda.sh
bash dev-cuda.sh --release
```
</details>

---

## 许可证

[GPLv3](LICENSE)
