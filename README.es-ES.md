

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

**Reconocimiento de voz a texto a nivel de sistema para macOS. Gratis y de código abierto.**

Presiona una tecla de acceso rápido en cualquier lugar. Habla. El texto se pegará en el cursor, reescrito por una IA que sabe en qué aplicación te encuentras.

Inglés | [繁體中文](README_TW.md) | [简体中文](README_CN.md)

<br/>

<table>
<tr>
<td align="center" valign="middle" width="33%"><img src="assets/demo-gmail.gif" width="280"/></td>
<td align="center" valign="middle" width="33%"><img src="assets/demo-notion.gif" width="280"/></td>
<td align="center" valign="middle" width="33%"><img src="assets/demo-telegram.gif" width="280"/></td>
</tr>
<tr>
<td align="center"><sub><b>Gmail</b> — formatea como un correo electrónico adecuado</sub></td>
<td align="center"><sub><b>Notion</b> — prosa estructurada y limpia</sub></td>
<td align="center"><sub><b>Telegram</b> — casual y conversacional</sub></td>
</tr>
</table>

<br/>

```bash
brew tap alan890104/sumi && brew install --cask sumi
```

[Descargar DMG](https://github.com/alan890104/sumi/releases/latest) · [Lanzamientos](https://github.com/alan890104/sumi/releases) · [Problemas](https://github.com/alan890104/sumi/issues)

</div>

---

## ¿Por qué Sumi?

<table>
<tr>
<td width="50%" valign="top">

### 🎯 Pulido con IA por aplicación
Cada aplicación recibe un prompt diferente para LLM. Slack suena como Slack. Gmail suena como un correo. La Terminal obtiene comandos limpios. 18 reglas integradas: escribe las tuyas, o describe lo que quieres y la IA genera la regla.

</td>
<td width="50%" valign="top">

### 🔒 Totalmente local
Ejecuta todo en el dispositivo: Whisper o Qwen3-ASR para reconocimiento de voz, LLM local para reescritura. El audio nunca sale de tu Mac. Puedes verificarlo: el código está aquí.

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🗣 Diarización de hablantes
El modo reunión transcribe continuamente en segundo plano. Las transcripciones están etiquetadas por hablante con marcas de tiempo. Importa archivos de audio existentes para transcripción retroactiva.

</td>
<td width="50%" valign="top">

### ✏️ Edición por voz
Selecciona cualquier texto, presiona `Option+E` y di lo que quieres hacer. "Haz esto más formal." "Traduce al japonés." "Acórtalo." La IA lo reescribe y lo pega de nuevo.

</td>
</tr>
<tr>
<td width="50%" valign="top">

### ☁️ Nube o Local — Tu elección
BYOK para todo: Groq, OpenAI, Deepgram, Azure, Gemini, OpenRouter o cualquier extremo compatible con OpenAI. Sin cuenta de Sumi. Sin suscripción.

</td>
<td width="50%" valign="top">

### 🌏 58 idiomas de interfaz
La interfaz se entrega en 58 locales. Los usuarios de chino tradicional obtienen una normalización automática de zh-CN → zh-TW en la salida de la transcripción.

</td>
</tr>
</table>

---

## La misma frase, tres aplicaciones

> Dices: *"um creo que el proyecto está un poco retrasado y probablemente deberíamos tener una reunión para averiguar qué hacer a continuación"*

<table>
<tr>
<td><b>LINE</b> (casual)</td>
<td>Creo que el proyecto está retrasado, deberíamos tener una reunión para decidir qué hacer a continuación</td>
</tr>
<tr>
<td><b>Slack</b> (profesional)</td>
<td>Creo que el proyecto está retrasado. Deberíamos tener una reunión para discutir los siguientes pasos.</td>
</tr>
<tr>
<td><b>Gmail</b> (correo)</td>
<td>Hola,<br/><br/>Creo que el proyecto está actualmente retrasado. ¿Podríamos programar una reunión para discutir los siguientes pasos?<br/><br/>Saludos cordiales</td>
</tr>
</table>

---

## Cómo funciona

1. La aplicación reside en la barra de menús — nada más en la pantalla.
2. Haz clic en cualquier campo de texto, en cualquier parte de tu Mac.
3. Presiona `Option+V`. Aparece una cápsula flotante con la forma de onda.
4. Habla.
5. Presiona `Option+V` nuevamente. El texto se pega.

**Edición por voz:** selecciona texto → `Option+E` → di qué hacer con él.
**Modo Reunión:** presiona `Option+M` para alternar la transcripción continua en segundo plano a un archivo de notas.

---

## En qué se ejecuta

**Reconocimiento de voz** — Local: Whisper (GPU Metal, 7 tamaños de modelo de 148 MB a 1.6 GB) o Qwen3-ASR. Nube: Groq, OpenAI, Deepgram, Azure, cualquier extremo personalizado.

**Reescritura con LLM** — Local: Qwen3-8B, Qwen2.5-7B, Llama 3 Taiwan 8B mediante candle (Metal/CUDA). Nube: OpenAI, Groq, Gemini, GitHub Models, OpenRouter, SambaNova, cualquier extremo compatible con OpenAI.

**Uso de recursos** — En reposo: ~130 MB, 0% CPU. Transcripción local: ~730 MB RSS, <20% CPU (Metal). Modo nube: ~7 MB durante la grabación, vuelve al 0% al finalizar.

**Otros detalles** — Silero VAD para filtrado de silencio · diccionario de pronunciación personalizado · historial de transcripciones con exportación de audio · teclas de acceso rápido personalizables

---

## Comparación

> [!NOTE]
> Esto refleja nuestra mejor comprensión al momento de escribir. Los competidores se actualizan con frecuencia: se aceptan correcciones mediante issues o PRs.

| | **Sumi** | Dictado integrado | Typeless | Wispr Flow | VoiceInk | SuperWhisper |
|---|---|---|---|---|---|---|
| **Precio** | **Gratis** | Gratis | 4K palabras/semana gratis, $12-30/mes | 2K palabras/semana gratis, $12-15/mes | $25-49 (pago único) | Prueba gratuita, ~$8/mes |
| **Código abierto** | ✅ GPLv3 | ❌ | ❌ | ❌ | ✅ GPLv3 | ❌ |
| **STT local** | ✅ | ✅ Apple Silicon | ❌ Solo nube | ❌ Solo nube | ✅ | ✅ |
| **STT en la nube** | ✅ BYOK | ❌ | ✅ | ✅ | ✅ Opcional | ✅ |
| **Pulido con IA** | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| **Pulido con LLM local** | ✅ 3 modelos | ❌ | ❌ | ❌ | ❌ | ✅ |
| **Reglas por aplicación** | ✅ 18 preajustes + personalizados | ❌ | ❌ | ✅ Estilos | ✅ Modos potentes | ✅ Modos personalizados |
| **Consciente del contexto** | ✅ App + URL | ❌ | ✅ App | ✅ App | ✅ App | ✅ Modo súper |
| **Edición por voz** | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ |
| **Diccionario** | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| **Historial** | ✅ + exportación de audio | ❌ | ✅ | ✅ | ✅ | ✅ |
| **Notas de reunión** | ✅ + etiquetas de hablante | ❌ | — | ❌ | — | ✅ |
| **Plataformas** | macOS | macOS, iOS | macOS, Win, iOS, Android | macOS, Win, iOS, Android | macOS | macOS, Win, iOS |

---

## Instalación

### Homebrew (recomendado)

```bash
brew tap alan890104/sumi
brew install --cask sumi
```

### Descargar DMG

1. Descarga el último DMG desde [GitHub Releases](https://github.com/alan890104/sumi/releases/latest).
2. Abre el DMG y arrastra Sumi a `/Applications`.
3. La aplicación aún no está notariada, por lo que macOS la bloqueará. Ejecuta esto primero:
   ```bash
   xattr -cr /Applications/Sumi.app
   ```
4. Al iniciar por primera vez: otorga acceso al micrófono y habilita Accesibilidad en Configuración del sistema → Privacidad y seguridad → Accesibilidad (necesario para el pegado automático).

### Compilar desde el código fuente

Requiere [Rust](https://rustup.rs/) y `cargo install tauri-cli --version "^2"`.

```bash
git clone https://github.com/alan890104/sumi.git
cd sumi
cargo tauri dev      # modo desarrollo
cargo tauri build    # .dmg de producción
```

<details>
<summary>Windows (CUDA)</summary>

Metal es exclusivo de macOS. En Windows:

```bash
# Solo CPU
cargo tauri dev --no-default-features

# NVIDIA CUDA (requiere CUDA Toolkit, LLVM, Ninja, CMake)
bash dev-cuda.sh
bash dev-cuda.sh --release
```
</details>

---

## Licencia

[GPLv3](LICENSE)
