# AltAltAi

Windows overlay coding agent — double-tap **Alt** to bring your current selection, files, and workspace to AI (DeepSeek).

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="AltAltAi" width="96" height="96" />
</p>

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <img alt="platform" src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4?style=flat-square" />
  <img alt="tauri" src="https://img.shields.io/badge/Tauri-2-FFC131?style=flat-square" />
  <img alt="vue" src="https://img.shields.io/badge/Vue-3-42B883?style=flat-square" />
  <img alt="rust" src="https://img.shields.io/badge/Rust-backend-DEA584?style=flat-square" />
  <img alt="ai" src="https://img.shields.io/badge/AI-DeepSeek-4D6BFE?style=flat-square" />
</p>

AltAltAi is a local Windows desktop overlay. It captures context from the foreground app, then runs a DeepSeek agent that can read and edit files, run Shell, use MCP tools, and search the web — without leaving the window you were in.

---

## Quickstart

### Installer

1. Download the MSI from [Releases](../../releases)
2. Open **Settings** from the tray and add your DeepSeek API key
3. In any app, **double-tap Alt** → type → Enter

### From source

Requires Node 18+, pnpm, Rust stable, VS C++ Build Tools, and WebView2.

```bash
pnpm install
pnpm tauri dev
```

Build an MSI:

```bash
pnpm tauri build
# → src-tauri/target/release/bundle/msi/
```

### Everyday use

| Shortcut | Action |
|----------|--------|
| Double-tap `Alt` | Show / hide overlay |
| Enter | Send (with captured context) |
| `/` | Slash commands |

---

## What you get

- **Overlay summon** — transparent panel near the cursor
- **Automatic context** — selection, Explorer files, active window, optional workspace
- **Coding agent** — files, Shell, Git, Skills, MCP, sub-agents, optional LSP / web search
- **Controls** — tool approval, Plan Mode, checkpoints / rewind, path sandbox
- **Local-first** — settings and chats stay on your machine by default

---

## Project layout

| Path | Role |
|------|------|
| `src/` | Overlay, chat UI, settings |
| `src-tauri/src/commands/` | Tauri IPC |
| `src-tauri/src/core/` | AI, chat, context, tools, MCP, checkpoint |
| `src-tauri/src/runtime/` | Search, browser, Git helpers |
| `src-tauri/prompts/` | System / tool prompts |

---

## Docs

- [使用指南](./docs/user-guide.md) — how to install, summon, chat, and configure
- [维护文档](./docs/maintenance.md) — develop, test, and release

---

## Privacy

API keys and settings live in the local app data directory. Chats are local by default. Workspace files leave your machine only when you send a turn to your configured DeepSeek endpoint.

---

## Acknowledgments

Inspired in part by [DeepSeek-Reasonix](https://github.com/DeepSeek-Reasonix).

Built with Tauri, Vue, Rust, and DeepSeek.

## License

This repository is dedicated to the public domain under the [Unlicense](./LICENSE).
