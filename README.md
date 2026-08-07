# AAAi

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="AAAi" width="112" height="112" />
</p>

<h2 align="center">An AI chat and coding assistant, available anywhere on Windows</h2>

<p align="center">
  Double-tap <kbd>Alt</kbd> for a floating overlay anywhere on your desktop.
  When a chat needs more room, open it in the workbench with one click.
</p>

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <img alt="platform" src="https://img.shields.io/badge/Windows-10%20%2F%2011-0078D4?style=flat-square" />
  <img alt="release" src="https://img.shields.io/badge/version-v0.2.1-4D6BFE?style=flat-square" />
  <img alt="license" src="https://img.shields.io/badge/license-Unlicense-3DA639?style=flat-square" />
  <img alt="stack" src="https://img.shields.io/badge/Tauri%202%20%2B%20Vue%203%20%2B%20Rust-black?style=flat-square" />
</p>

## Documentation

| Document                                                 | Description                                             |
| -------------------------------------------------------- | ------------------------------------------------------- |
| [Architecture overview](./docs/architecture-overview.md) | Layers, control flow, agent loop, persistence, events   |
| [Maintenance guide](./docs/maintenance.md)               | Local setup, debug, tests, release hygiene, conventions |
| [Releases & updates](./docs/release.md)                  | MSI signing, `latest.json`, GitHub Releases / CI        |
| [Docs index](./docs/README.md)                           | Full documentation map (EN / 中文)                      |

中文文档入口：[README.zh-CN.md](./README.zh-CN.md) · [架构](./docs/architecture-overview.zh-CN.md) · [维护](./docs/maintenance.zh-CN.md) · [发布](./docs/release.zh-CN.md)

---

## Overlay — ask from anywhere

Double-tap <kbd>Alt</kbd> in any app to show or hide the floating window. Ask a
question, follow up in place, and keep Agent / model / approval controls under
the composer.

<p align="center">
  <img src="./docs/image/Alt%2BAlt.png" alt="AAAi floating overlay conversation" width="560" />
</p>

AAAi tries to pick up the current text selection or Explorer selection. You can
also paste or drag images and files into the input.

<p align="center">
  <img src="./docs/image/select_text_recognition.webp" alt="AAAi recognizing selected text context" width="800" />
</p>

<p align="center">
  <img src="./docs/image/select_image_recognition.webp" alt="AAAi attaching an image from selection" width="800" />
</p>

Summoning outside an IDE starts a **temporary Quick Ask** session — it is not
bound to a workspace, so history does not land in a stale project. Bind a
workspace only when you choose one in the overlay (or with `/work`), or when you
trigger from an IDE that is actually in the foreground.

While a conversation is running, use **Open conversation in workbench** (the
window icon on the overlay) to move that same session into the full desktop UI
in one click — progress, tools, and history continue there.

### IDE context plugins

Install the companion plugin so VS Code or IntelliJ can push active file,
workspace, language, and selection to the local AAAi app (best-effort; the
editor keeps working if AAAi is not running).

- [Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=AAAi.aaai-ide-context)
- [IntelliJ Platform](https://plugins.jetbrains.com/plugin/33163-aaai-ide-context)

---

## Workbench — manage every session

The workbench is the full desktop surface. Temporary Quick Ask chats from the
overlay appear here alongside pinned threads and project workspaces, so you can
talk, switch, and organize them in one place.

<p align="center">
  <img src="./docs/image/workspace.png" alt="AAAi workbench with pinned chats, workspaces, and quick ask" width="900" />
</p>

- **Pinned** — keep important threads at the top.
- **Workspaces** — bind chats to a project folder so Agent edits stay in context.
- **Quick Ask** — the same temporary sessions started from the overlay; continue
  them here, start new ones, or keep a long-running chat in the workbench while
  you still summon the overlay elsewhere.

### Review changes

When Agent edits files, AAAi shows a per-file summary and a focused Diff view so
you can inspect every addition and deletion before you move on.

<p align="center">
  <img src="./docs/image/workspace-diff.png" alt="AAAi reviewing Agent file changes in the Diff panel" width="900" />
</p>

- Task list and verification stay on the conversation timeline.
- Open **Review** to browse side-by-side or unified diffs.
- Undo is available for changes AAAi applied in the current session (checkpoints).

### Settings

Configure models, providers, agent behavior, and extensions from the embedded
settings page — theme, memory, search, MCP, Skills, and more.

<p align="center">
  <img src="./docs/image/workspace-settings.png" alt="AAAi model settings for DeepSeek and multimodal options" width="900" />
</p>

Highlights:

- Default chat model and optional vision / multimodal fallback.
- Split multimodal analysis when the primary model cannot see images.
- Reasoning effort, reasoning language, and whether to show the thinking process.
- Tool approval mode (for example Always allow) and Agent work display density.
- Large context window toggle (≈1M vs 64k budgets for compaction / turn limits).

---

## Capabilities

### Ask and Agent

| Mode      | Intent                              | Typical tools                                       |
| --------- | ----------------------------------- | --------------------------------------------------- |
| **Ask**   | Read-only investigation             | Read files, search, LSP, configured read-only tools |
| **Agent** | Default; change the world carefully | Files, PowerShell, Git, Skills, MCP, sub-agents     |

Ask withholds write / shell / git capabilities. Agent enables them subject to the
approval policy in Settings. Both modes share the same `AgentRunner` loop —
policy is enforced at tool schema exposure and approval gates, not via a second
orchestrator.

### Timeline and tool cards

Assistant turns interleave **reasoning**, **reply text**, and **tool activity**
in chronological order (live stream and persisted history). Long thinking no
longer hides the commands and edits that happened mid-thought.

### Integrations

- **Microsoft Office** — when Word, Excel, or PowerPoint is running, AAAi can collect document context and use `word_*` / `excel_*` / `ppt_*` tools (COM).
- **Skills** — built-in and vendor skills (docx, pandoc, research, review, bid tech, …) loaded as playbooks; can run as sub-agents.
- **MCP** — connect stdio / remote MCP servers (including Smithery-oriented helpers).
- **LSP** — language server diagnostics when configured.
- **Pinned-image badge** — optional PixPin / Snipaste badge to open a chat with that image attached.
- **Sub-agents** — split larger work across child agents while progress stays visible in the main thread.
- **Memory** — local memory tools; optional mem0 cloud sync.
- **Web search** — Serper or Tavily when an API key is set.

### Model providers

- **DeepSeek** — API key.
- **Gemini** — Google sign-in (Antigravity OAuth).
- **Custom** — OpenAI-compatible Base URL, API key, and model list.

For image input with a text-only primary model, set a vision model or enable
multimodal split analysis in Settings.

The composer shows session token estimates and context usage; you can pick model
and thinking level while tools run.

---

## Install and get started

1. Download and install the MSI from [Releases](../../releases).
2. Open **Settings** from the AAAi tray icon and configure a model provider.
3. Double-tap <kbd>Alt</kbd>, type a question, press <kbd>Enter</kbd> — or open the session in the workbench when you need the full UI.

| Shortcut                                            | Action                                          |
| --------------------------------------------------- | ----------------------------------------------- |
| Double-tap <kbd>Alt</kbd>                           | Show or hide the overlay                        |
| <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Space</kbd> | Fallback summon shortcut                        |
| <kbd>Enter</kbd>                                    | Send a message                                  |
| <kbd>/</kbd>                                        | Slash commands                                  |
| <kbd>Esc</kbd>                                      | Clear input; closes the window in some contexts |

---

## Data and privacy

API keys, OAuth tokens, settings, and chat history stay on your machine by
default. Context capture is local; the message and attached context leave the
device only when you send them to your configured provider.

Enabling web search, MCP, or mem0 cloud sync also sends data to those services —
enable them only if you accept their policies.

Crash recovery uses a local SQLite journal so interrupted streaming turns can be
settled on next launch instead of leaving the UI stuck “executing”.

---

## Technical architecture (summary)

AAAi is a single-process **Tauri 2** application: WebView2 surfaces (Vue 3 + Pinia)
for presentation, and a Rust host for OS integration, chat domain logic, model I/O,
and tool execution.

```mermaid
flowchart TB
  subgraph Surfaces["Window surfaces"]
    WB[Workbench]
    OV[Overlay]
    ST[Settings]
    PV[Image preview]
  end

  subgraph Host["AAAi.exe — Rust host"]
    CMD[commands / EventBus]
    CHAT[ChatService · StreamManager · AgentRunner]
    TOOLS[ToolRegistry · Skills · MCP · Office]
    STORE[(SQLite + journal)]
  end

  subgraph External["External"]
    LLM[Model providers]
    IDE[IDE plugins]
    OFFICE[Word / Excel / PPT]
  end

  WB & OV & ST & PV <-->|IPC invoke + events| CMD
  CMD --> CHAT
  CHAT --> TOOLS
  CHAT --> STORE
  CHAT -->|HTTPS stream| LLM
  IDE -->|context push| Host
  TOOLS -->|COM| OFFICE
```

Primary chat path:

```text
invoke("chat")
  → ChatService::send
  → StreamManager / AgentRuntime
  → AgentRunner::run  (agent_loop policies)
  → AIProvider::stream + ToolRegistry
  → EventBus → chat-* / tool-* events → Pinia
  → ConversationManager persists messages (+ work_timeline)
```

`AgentRunner` owns the model↔tools loop. `AgentRuntime` owns run lifecycle
(cancel, soft-inject, debug). Do not add a second chat loop beside `AgentRunner`.

Full diagrams (layers, sequencing, persistence, frontend batching, updates):
[Architecture overview](./docs/architecture-overview.md) ·
[简体中文](./docs/architecture-overview.zh-CN.md).

Maintainer workflows (debug, tests, version bump, data paths):
[Maintenance guide](./docs/maintenance.md).

---

## Run from source

Requires Node.js 18+, pnpm, Rust stable, VS C++ Build Tools, and WebView2.

```bash
pnpm install
pnpm tauri:dev
```

```bash
pnpm check          # typecheck + lint + frontend tests
cd src-tauri && cargo test --lib
pnpm tauri:build
```

The installer is written to `src-tauri/target/release/bundle/msi/` as
`AAAi_0.2.1_x64.msi` (no `_en-US` suffix).

For releases and in-app updates (`latest.json`, signing, GitHub Releases), see
[Releases and remote updates](./docs/release.md) · [简体中文](./docs/release.zh-CN.md).

---

## License

This repository is dedicated to the public domain under the [Unlicense](./LICENSE).
