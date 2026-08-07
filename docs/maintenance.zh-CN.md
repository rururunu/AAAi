# AAAi 维护手册

面向 AAAi 开发者与维护者的操作手册。设计背景见
[技术架构总览](./architecture-overview.zh-CN.md)；发版流程见
[发布与远程更新](./release.zh-CN.md)。

<p>
  <a href="./maintenance.md">English</a> ·
  <a href="./maintenance.zh-CN.md">简体中文</a>
</p>

|              |                                |
| ------------ | ------------------------------ |
| **读者**     | 贡献者、发版负责人、日常维护者 |
| **产品版本** | v0.2.1                         |
| **主平台**   | Windows 10 / 11（x64）         |

---

## 1. 职责范围

| 领域          | 关注点                                          |
| ------------- | ----------------------------------------------- |
| 产品表面      | 悬浮窗、工作台、设置、更新体验                  |
| 领域循环      | `AgentRunner` / `agent_loop` 保持唯一主轴       |
| Provider      | DeepSeek、Gemini OAuth、自定义 OpenAI 兼容      |
| 工具与 Skills | 注册表、审批、Office COM、MCP、markdown skills  |
| 数据完整性    | SQLite schema 迁移、启动时 journal 结算         |
| 发版          | 版本号、签名 MSI、`latest.json`、GitHub Release |

**禁止：**另造第二套 Agent 循环；从 Vue store 绕过 IPC 门面；在 PR 中提交
`target/` / `dist/` 等生成物。

---

## 2. 开发环境

### 2.1 前置条件

- Node.js 18+ 与 **pnpm**
- Rust **stable**（`rustup`）
- Visual Studio C++ Build Tools（MSVC + Windows SDK）
- WebView2（Win10/11 通常已自带）
- 可选：本机 Office（测 COM）；VS Code / IntelliJ（测 IDE 上下文插件）

### 2.2 克隆与运行

```powershell
pnpm install
pnpm tauri:dev
```

仅前端 Vite（无原生宿主）：

```powershell
pnpm dev
```

### 2.3 本地质量门禁

```powershell
pnpm check                 # typecheck + eslint + vitest
cd src-tauri
cargo check
cargo test --lib
```

合并前建议：`pnpm check` **与** `cargo test --lib` 都通过。

### 2.4 关键配置文件

| 文件                            | 用途                                      |
| ------------------------------- | ----------------------------------------- |
| `package.json`                  | 前端版本、脚本、依赖                      |
| `src-tauri/tauri.conf.json`     | 产品版本、窗口、updater 公钥              |
| `src-tauri/Cargo.toml`          | Rust crate 版本（`peek` / 二进制 `AAAi`） |
| `src-tauri/permissions/*.toml`  | Tauri capability / ACL                    |
| `src-tauri/prompts/*.md`        | 稳定系统提示词                            |
| `src-tauri/prompts/skills/*.md` | Skill playbook（路由 + 行为）             |
| `.github/workflows/release.yml` | 推送 `v*` 标签触发签名发版                |

务必保持 **`package.json`**、**`tauri.conf.json`**、**`Cargo.toml`** 版本一致。

---

## 3. 仓库布局（维护者地图）

```text
AltAltAi/
├── src/                      # Vue 3 呈现层
│   ├── components/chat/      # 时间线、工具、输入栏
│   ├── layouts/              # Overlay / Workbench
│   ├── stores/               # Pinia
│   ├── services/ipc/         # 唯一通往 Tauri 的桥
│   └── pages/Settings/       # 内嵌设置
├── src-tauri/
│   ├── src/
│   │   ├── commands/         # 薄 IPC
│   │   ├── core/             # 领域
│   │   ├── runtime/          # 工具适配
│   │   ├── services/         # 窗口、热键、设置、OAuth
│   │   └── adapters/         # EventBus → Tauri
│   ├── prompts/
│   └── tauri.conf.json
├── docs/
└── scripts/
```

依赖规则见 [架构 §3](./architecture-overview.zh-CN.md#3-逻辑架构)。

---

## 4. 本机运行时数据

路径随 Tauri 应用数据目录约定略有差异；以下为状态**类型**：

| 类型        | 典型内容                                   | 说明                         |
| ----------- | ------------------------------------------ | ---------------------------- |
| 聊天 SQLite | 消息、journal、会话标题/工作区、token 用量 | init 时迁移（`ALTER TABLE`） |
| 设置存储    | Provider、密钥、Agent 偏好、MCP、skills    | 勿提交密钥                   |
| OAuth 凭据  | Gemini / Antigravity                       | 仅本机                       |
| 检查点      | Agent 改文件的撤销数据                     | 会话级                       |
| 更新缓存    | updater 元数据                             | 由 `latest.json` 驱动        |

**Schema 变更：**在 `core/chat/db.rs` 按现有 `PRAGMA table_info` 模式做**加法**迁移
（`tool_activities`、`estimated_tokens`、`work_timeline` 同模式）。字段用
`Option` / serde default。

**清空本地状态（仅开发）：**退出应用后删除 Windows 本地/漫游应用数据中的 AAAi
目录。不要提供无确认的历史清空能力给普通用户。

---

## 5. 日常改动流程

### 5.1 聊天 / 流式缺陷

1. 同时在悬浮窗与工作台复现（共享 `session_id`）。
2. 追踪：`commands/chat` → `ChatService::send` → `StreamManager` → `AgentRunner`。
3. 核对 `adapters/tauri_events` 与 `src/main.ts` 监听。
4. 顺序类问题（思考 vs 工具）查消息上的 `work_timeline`，不要只看
   `content` + `toolActivities`。
5. 根因属顺序 / 持久化 / 策略时，在 `conversation_manager` 或 `agent_loop` 补测试。

### 5.2 工具或审批回退

1. 检查 Ask / Agent 的 Schema 过滤（`read_only`、plan mode）。
2. 检查审批路径（`tool_approval`、`ask_user`、路径权限）。
3. 确认 `ToolActivity` upsert 与时间线 `Tool` 条目（仅首次插入）。
4. 优先在注册表附近写单元测试，而不是只靠 UI。

### 5.3 提示词 / Skill 变更

1. 编辑 `src-tauri/prompts/` 或 `prompts/skills/`。
2. **核心**提示词不得写入具体 skill 产品名（见 `core/chat/prompts` 测试）。
3. 「何时用 / 何时不用」写在 skill 文件自身。
4. 结构调整后跑 `cargo test --lib`（或 prompts 相关过滤）。
5. Rust 注释只解释 _为什么_，不要写厂商营销名。

### 5.4 Provider 变更

1. 在 `core/ai/` 实现 / 调整 `AIProvider`。
2. 接线设置与前端模型选择器。
3. 覆盖流式边界：重试（`stream_retry`）、空工具参数、reasoning。
4. 多模态：主模型纯文本时走既有分拆分析路径。

### 5.5 仅前端 UX

1. 停在 `components` / `composables`；经 Pinia action 改状态。
2. 组件内不要直接调 `@tauri-apps/api`，走 `services/ipc`。
3. Token 展示统一用 `services/chat/tokenEstimate.ts` 的 `formatTokenCount`
   并传入当前语言（输入栏与消息底栏必须一致）。

---

## 6. 调试手册

| 现象                   | 可能层                 | 先查                                                 |
| ---------------------- | ---------------------- | ---------------------------------------------------- |
| 浮窗能开但无回复       | Provider / 网络 / 设置 | 模型是否配置？Key？`chat-error`？                    |
| 崩溃后卡在「执行中」   | Journal 结算           | 重启；看 `status` 与 running tools                   |
| 工具与思考上下颠倒     | Timeline               | 实时：store `workTimeline`；历史：DB `work_timeline` |
| 追问冒出新气泡         | Soft-inject            | 会话是否已有活跃 assistant？                         |
| Diff / 撤销缺失        | Checkpoint             | 是否本会话由 AAAi 工具写入？                         |
| Office 工具缺失        | COM / 进程             | Word/Excel/PPT 是否在跑？                            |
| MCP OAuth「丢失」      | mcp-remote 钉版本      | 包版本钉死 + 配置目录稳定                            |
| `smithery.ts` 类型错误 | 前端 skills            | 与本次无关也可能阻塞 `pnpm check`                    |

可用 `RUST_LOG`；UI 中如有 Agent debug 面板可查看 run 快照。

---

## 7. 测试策略

| 层       | 命令                           | 守护内容                                    |
| -------- | ------------------------------ | ------------------------------------------- |
| 前端单元 | `pnpm test`                    | IPC 辅助、token 估算、展示辅助              |
| 前端静态 | `pnpm typecheck` / `pnpm lint` | 类型与风格                                  |
| Rust lib | `cargo test --lib`             | Agent 循环、DB、时间线、提示词、Provider    |
| 手工冒烟 | `pnpm tauri:dev`               | 热键、浮窗↔工作台、一次 Agent 改文件 + 审查 |

**发版前必过**

- [ ] Ask 模式无法写文件 / 跑 Shell
- [ ] Agent 改动能出现在审查且可撤销
- [ ] 思考与工具卡片重载后仍交错
- [ ] 中途取消能结算消息状态
- [ ] Updater 公钥在位；有密钥时可签 MSI

---

## 8. 版本提升清单

切 `vX.Y.Z` 时：

1. 同步三处版本：
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`（并更新 `Cargo.lock` 中 peek 包版本）
2. 更新 README 徽章 / MSI 示例名（若硬编码）。
3. 按需更新发布文档示例。
4. 用 UTF-8 安全方式改 `Cargo.toml`（避免 PowerShell 破坏描述中的破折号）。
5. 按 [release.zh-CN.md](./release.zh-CN.md) 签名并生成 `latest.json`。

---

## 9. 编码约定

- 小而可审的 diff；贴合现有命名与模块边界。
- 注释解释 **为什么**，不要写「借鉴某某厂商」。
- 新增 `ChatMessage` 字段：改结构、serde、SQLite 迁移、字面量/helper、前端类型、持久化测试。
- 新增 Bus 事件：`core/event` 定义 → adapters 投影 → `main.ts` / ipc 订阅 → 写入架构 §12。
- 勿提交 `__pycache__`、密钥、带真实签名的本地 `release/*.json`。

---

## 10. 提示词与 Skill 维护

| 资产                                                    | 规则                                    |
| ------------------------------------------------------- | --------------------------------------- |
| `system.md` / `tools.md` / `policies.md` / `context.md` | 稳定核心；示例丰富；无具体 skill 产品名 |
| `compact-summary.md`                                    | 仅压缩 LLM 格式                         |
| `skills/*.md`                                           | 自包含路由 + 流程                       |
| 体积 / 隔离测试                                         | 可有意抬升上限；保持与 skill 隔离断言   |

大改提示词后跑结构测试，并手工跑一回合相关 skill。

---

## 11. 安全与隐私卫生

- 勿在日志中打印 API Key、OAuth refresh、完整用户文档。
- 路径权限与审批模式是安全控制，回退按高优先级处理。
- Shell 在用户机器上执行；默认审批应偏保守，除非用户选择「始终允许」。
- 第三方 MCP / 搜索 / mem0：设置文案需标明数据会离开本机。

---

## 12. 事故响应（简版）

1. **复现**：新快速提问会话，步骤最小化。
2. **采集**：版本、Provider、Ask/Agent、是否绑定工作区。
3. **归类**：仅 UI / 领域循环 / Provider / 持久化。
4. **止血**：在 `AgentRunner` / store / 迁移上修；顺序类问题不要只改 UI。
5. **验证**：为根因补 Rust 单测。

---

## 13. 常用命令速查

```powershell
# 开发
pnpm install
pnpm tauri:dev
pnpm check
cd src-tauri; cargo test --lib

# 构建安装包
pnpm tauri:build

# 发版元数据
pnpm release:json -- --tag v0.2.1 --notes "…"

# 同步外部 skill（如使用）
pnpm sync-skills
```

---

## 14. 文档所有权

| 文档                      | 何时更新                         |
| ------------------------- | -------------------------------- |
| `architecture-overview.*` | 分层、循环、事件、持久化模型变化 |
| `maintenance.*`           | 开发流程、清单、数据位置变化     |
| `release.*`               | 签名、CI、更新 URL 变化          |
| 根目录 `README*`          | 用户可见能力或安装路径变化       |

结构章节保持中英文同步；截图可共用。
