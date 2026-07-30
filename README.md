# AAAi

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="AAAi" width="112" height="112" />
</p>

<h2 align="center">An AI chat and coding assistant, available anywhere on Windows</h2>

<p align="center">
  Double-tap <kbd>Alt</kbd> to bring a text selection, selected Explorer files, or manually attached images and files into a conversation.
</p>

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <img alt="platform" src="https://img.shields.io/badge/Windows-10%20%2F%2011-0078D4?style=flat-square" />
  <img alt="release" src="https://img.shields.io/badge/version-v0.1.3-4D6BFE?style=flat-square" />
  <img alt="license" src="https://img.shields.io/badge/license-Unlicense-3DA639?style=flat-square" />
</p>

## Context and attachments

When summoned, AAAi attempts to read the text selection in the foreground app or files selected in Explorer. You can also paste or drag images, text, and text files into the input.

<p align="center">
  <img src="./docs/image/select_text_recognition.webp" alt="AAAi recognizing a selected text context" width="720" />
</p>

<p align="center">
  <img src="./docs/image/select_image_recognition.webp" alt="AAAi attaching image context from a selected image" width="720" />
</p>

The workspace is not detected automatically: select it in Settings or with `/work`. Unselected clipboard text and the active-window title are not added to a message automatically.

### IDE context plugins

AAAi can receive richer coding context directly from VS Code and IntelliJ Platform IDEs. When you select code, the plugin shares the active file, workspace, language, selected text, and cursor/selection position with the locally running AAAi app. Delivery is best-effort, so the editor remains unaffected when AAAi is not running.

- [Install for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=AAAi.aaai-ide-context)
- [Install for IntelliJ Platform](https://plugins.jetbrains.com/plugin/33163-aaai-ide-context)

## How it works

### One gesture away

Double-tap <kbd>Alt</kbd> in any app to show or hide the overlay. The default fallback shortcut is <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Space</kbd>, configurable in Settings.

### Ask and Agent

- **Ask**: read-only tools such as file reading, search, LSP, and configured read-only tools; it cannot change files, run Shell commands, or use Git.
- **Agent**: the default mode; can read and edit files, run PowerShell, and use Git, Skills, MCP, and sub-agents. Tool approval behavior is configured in Settings.

<p align="center">
  <img src="./docs/image/set_code.png" alt="Agent editing files and showing the resulting changes" width="720" />
</p>

### Workflow helpers

- **Pinned-image assistance**: add an AAAi badge to PixPin or Snipaste pinned images, then open a conversation with that image already attached.
- **File diffs**: review Agent changes file by file, with additions and deletions kept together in a focused Diff view.
- **Sub-agents**: larger tasks can be divided among child agents, while their progress and tool activity remain visible from the main conversation.

### Model providers

- DeepSeek with an API key.
- Gemini through Google sign-in with Antigravity OAuth.
- Custom OpenAI-compatible providers with a Base URL, API key, and model list.

For image input with a model that does not support images, configure a vision model or enable multimodal split analysis in Settings.

### Optional capabilities

- Web search requires enabling it and configuring a Serper or Tavily API key.
- MCP requires adding MCP servers in Settings.
- LSP requires enabling it in Settings.
- Cross-conversation memory can use local memory; mem0 cloud sync contacts its service.

The input shows context usage. You can choose a model and thinking level, and inspect tool activity during a conversation.

## Install and get started

1. Download and install the MSI from [Releases](../../releases).
2. Open **Settings** from the AAAi system tray icon and configure a model provider.
3. Return to any app, double-tap <kbd>Alt</kbd>, then type and press <kbd>Enter</kbd>.

| Shortcut | Action |
| --- | --- |
| Double-tap <kbd>Alt</kbd> | Show or hide the overlay |
| <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Space</kbd> | Fallback summon shortcut |
| <kbd>Enter</kbd> | Send a message |
| <kbd>/</kbd> | Open slash commands |
| <kbd>Esc</kbd> | Clear input; closes the window in some contexts |

## Data and privacy

API keys, OAuth tokens, settings, and chat history are stored locally by default. Context and file capture also happen locally; only when you send a message are the message and its attached context sent to your configured model provider.

When you enable web search, MCP, or mem0 cloud sync, relevant content is also sent to the corresponding third-party service. Decide whether to enable them based on each service's privacy policy.

## Run from source

Requires Node.js 18+, pnpm, Rust stable, VS C++ Build Tools, and WebView2.

```bash
pnpm install
pnpm tauri dev
```

Build an MSI:

```bash
pnpm tauri build
```

The installer is written to `src-tauri/target/release/bundle/msi/`.

## License

This repository is dedicated to the public domain under the [Unlicense](./LICENSE).
