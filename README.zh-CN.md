# AltAltAi

Windows 上的 Overlay 编码助手——**双击 Alt**，把当前选区、文件和工作区交给 AI（DeepSeek）。

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

AltAltAi 是本机运行的 Windows 桌面 Overlay：从前台应用捕获上下文，再交给 DeepSeek Agent 读写文件、跑 Shell、调 MCP、搜网页——不用离开你正在用的窗口。

---

## 快速开始

### 安装包

1. 从 [Releases](../../releases) 下载 MSI  
2. 托盘打开 **设置**，填入 DeepSeek API Key  
3. 任意应用里 **双击 Alt** → 输入 → 回车  

### 从源码运行

需要 Node 18+、pnpm、Rust stable、VS C++ Build Tools、WebView2。

```bash
pnpm install
pnpm tauri dev
```

打包 MSI：

```bash
pnpm tauri build
# → src-tauri/target/release/bundle/msi/
```

### 日常操作

| 快捷键 | 作用 |
|--------|------|
| 双击 `Alt` | 显示 / 隐藏 Overlay |
| 回车 | 发送（附带捕获的上下文） |
| `/` | 斜杠命令 |

---

## 能力概览

- **Overlay 唤出** — 鼠标附近的透明面板  
- **自动上下文** — 选区、资源管理器文件、活动窗口、可选工作区  
- **编码 Agent** — 文件、Shell、Git、Skills、MCP、子 Agent，以及可选 LSP / 网页搜索  
- **可控** — 工具审批、Plan Mode、Checkpoint / 撤回、路径沙箱  
- **本机优先** — 设置与对话默认落在本地  

---

## 仓库结构

| 路径 | 职责 |
|------|------|
| `src/` | Overlay、聊天 UI、设置 |
| `src-tauri/src/commands/` | Tauri IPC |
| `src-tauri/src/core/` | AI、聊天、上下文、工具、MCP、checkpoint |
| `src-tauri/src/runtime/` | 搜索、浏览器、Git 辅助 |
| `src-tauri/prompts/` | System / 工具提示词 |

---

## 文档

- [使用指南](./docs/user-guide.md) — 安装、唤出、聊天与设置  
- [维护文档](./docs/maintenance.md) — 开发、测试与发版  

---

## 隐私

API Key 与设置保存在本机应用数据目录。聊天默认本地存储。工作区文件仅在你发起对话时发往已配置的 DeepSeek 端点。

---

## 致谢

部分思路参考了 [DeepSeek-Reasonix](https://github.com/DeepSeek-Reasonix)。

基于 Tauri、Vue、Rust 与 DeepSeek 构建。

## 许可协议

本仓库采用 [Unlicense](./LICENSE)，相当于公共领域，几乎无限制使用、修改与分发。
