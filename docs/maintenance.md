# AAAi Maintenance Guide

Operational handbook for developers and maintainers of AAAi. Pair with the
[Architecture overview](./architecture-overview.md) for design context and
[Releases](./release.md) for shipping.

<p>
  <a href="./maintenance.md">English</a> ·
  <a href="./maintenance.zh-CN.md">简体中文</a>
</p>

|                       |                                                     |
| --------------------- | --------------------------------------------------- |
| **Audience**          | Contributors, release managers, on-call maintainers |
| **Product version**   | v0.2.1                                              |
| **Primary platforms** | Windows 10 / 11 (x64)                               |

---

## 1. Responsibilities

| Area             | Owner focus                                                 |
| ---------------- | ----------------------------------------------------------- |
| Product surfaces | Overlay, workbench, settings, updater UX                    |
| Domain loop      | `AgentRunner` / `agent_loop` policies stay the single spine |
| Providers        | DeepSeek, Gemini OAuth, custom OpenAI-compatible            |
| Tools & skills   | Registry, approval, Office COM, MCP, markdown skills        |
| Data integrity   | SQLite schema migrations, journal settle on boot            |
| Release          | Version bump, signed MSI, `latest.json`, GitHub Release     |

Do **not** invent a second agent loop, skip the IPC façade from Vue stores, or
edit generated `target/` / `dist/` artifacts in PRs.

---

## 2. Development environment

### 2.1 Prerequisites

- Node.js 18+ and **pnpm**
- Rust **stable** toolchain (`rustup`)
- Visual Studio C++ Build Tools (MSVC + Windows SDK)
- WebView2 runtime (usually present on Windows 10/11)
- Optional: Office desktop apps for COM tool testing; VS Code / IntelliJ for IDE context plugins

### 2.2 Clone and run

```powershell
pnpm install
pnpm tauri:dev
```

Frontend-only Vite (no native host):

```powershell
pnpm dev
```

### 2.3 Quality gates (local)

```powershell
pnpm check                 # typecheck + eslint + vitest
cd src-tauri
cargo check
cargo test --lib
cargo fmt --check          # if formatting is enforced in CI
```

Recommended before merge: `pnpm check` **and** `cargo test --lib`.

### 2.4 Important config files

| File                            | Purpose                                  |
| ------------------------------- | ---------------------------------------- |
| `package.json`                  | Frontend version, scripts, deps          |
| `src-tauri/tauri.conf.json`     | Product version, windows, updater pubkey |
| `src-tauri/Cargo.toml`          | Rust crate version (`peek` / bin `AAAi`) |
| `src-tauri/permissions/*.toml`  | Tauri capability / ACL fragments         |
| `src-tauri/prompts/*.md`        | Stable system prompts                    |
| `src-tauri/prompts/skills/*.md` | Skill playbooks (routing + behavior)     |
| `.github/workflows/release.yml` | Tag-triggered signed release             |

Keep **`package.json`**, **`tauri.conf.json`**, and **`Cargo.toml`** versions in sync.

---

## 3. Repository layout (maintainer map)

```text
AltAltAi/
├── src/                      # Vue 3 presentation
│   ├── components/chat/      # Timeline, tools, composer
│   ├── layouts/              # Overlay / Workbench shells
│   ├── stores/               # Pinia (chat, settings, models)
│   ├── services/ipc/         # Only bridge to Tauri
│   └── pages/Settings/       # Embedded settings
├── src-tauri/
│   ├── src/
│   │   ├── commands/         # Thin IPC handlers
│   │   ├── core/             # Domain (chat, ai, tools, …)
│   │   ├── runtime/          # Tool adapters (git, search, …)
│   │   ├── services/         # Window, hotkey, settings, OAuth
│   │   └── adapters/         # EventBus → Tauri emits
│   ├── prompts/              # System + skill markdown
│   └── tauri.conf.json
├── docs/                     # Architecture, maintenance, release
└── scripts/                  # MSI rename, latest.json, skill sync
```

Dependency rules (non-negotiable): see [Architecture §3](./architecture-overview.md#3-logical-architecture).

---

## 4. Runtime data on disk

Exact paths can vary with Tauri app data conventions; treat these as the
canonical _kinds_ of state:

| Kind              | Typical contents                                          | Notes                                            |
| ----------------- | --------------------------------------------------------- | ------------------------------------------------ |
| Chat SQLite       | Messages, journal, session titles/workspaces, token usage | Migrated on init (`ALTER TABLE` for new columns) |
| Settings store    | Providers, keys, agent prefs, MCP, skills toggles         | Never commit secrets                             |
| OAuth credentials | Gemini / Antigravity tokens                               | Local only                                       |
| Checkpoints       | Undo data for Agent file edits                            | Session-scoped                                   |
| Updater           | Cached update metadata                                    | Driven by `latest.json`                          |

**Schema changes:** add additive migrations next to existing `PRAGMA table_info`
checks in `core/chat/db.rs` (pattern used for `tool_activities`, `estimated_tokens`,
`work_timeline`). Prefer additive columns with `Option` / default serde.

**Resetting local state (dev only):** quit the app, then remove the app data
directory for AAAi under the Windows local/roaming app data tree. Do not ship
reset tools that wipe user history without confirmation.

---

## 5. Day-to-day change workflows

### 5.1 Chat / streaming bug

1. Reproduce on Overlay **and** Workbench (shared `session_id`).
2. Trace: `commands/chat` → `ChatService::send` → `StreamManager` → `AgentRunner`.
3. Confirm events in `adapters/tauri_events` and listeners in `src/main.ts`.
4. For ordering bugs (thinking vs tools), inspect `work_timeline` on the message
   in SQLite / store — not only `content` + `toolActivities`.
5. Add a focused Rust test under `conversation_manager` or `agent_loop` when the
   bug is ordering / persistence / policy.

### 5.2 Tool or approval regression

1. Check Ask vs Agent schema filtering (`read_only`, plan mode).
2. Check approval path (`tool_approval`, `ask_user`, path permissions).
3. Verify `ToolActivity` upsert and timeline `Tool` entry (first insert only).
4. Prefer golden / unit tests near the registry over UI-only checks.

### 5.3 Prompt / skill change

1. Edit markdown under `src-tauri/prompts/` or `prompts/skills/`.
2. Keep **core** prompts free of skill-specific routing (enforced by tests in
   `core/chat/prompts/mod.rs`).
3. Put “When to use / When NOT to use” examples in the skill file itself.
4. Run `cargo test --lib prompts` (or full `--lib`) after structural edits.
5. Comments in Rust must explain _why_, without vendor marketing names.

### 5.4 Provider change

1. Implement / adjust `AIProvider` in `core/ai/`.
2. Wire settings + frontend model picker.
3. Cover streaming edge cases: retry (`stream_retry`), empty tool args, reasoning.
4. Multimodal: follow existing split-analysis path when primary model is text-only.

### 5.5 Frontend-only UX

1. Stay inside `components` / `composables`; mutate via Pinia actions.
2. Do not call `@tauri-apps/api` from components — go through `services/ipc`.
3. Token display: use `formatTokenCount` from `services/chat/tokenEstimate.ts`
   with the active language (composer and message footer must match).

---

## 6. Debugging playbook

| Symptom                                         | Likely layer                  | First checks                                                               |
| ----------------------------------------------- | ----------------------------- | -------------------------------------------------------------------------- |
| Overlay opens but no reply                      | Provider / network / settings | Model configured? API key? `chat-error` payload?                           |
| Stuck “executing” after crash                   | Journal settle                | Restart; inspect message `status` and running tools                        |
| Tools appear above / below thinking incorrectly | Timeline                      | Live: store `workTimeline`; History: DB `work_timeline` JSON               |
| Soft follow-up creates new bubble               | Soft-inject                   | Active assistant for session? `AgentRuntime` inject path?                  |
| Diff / undo missing                             | Checkpoint                    | Was edit applied via AAAi tools this session?                              |
| Office tools missing                            | COM / process                 | Is Word/Excel/PPT running? Tool hint in context?                           |
| MCP OAuth “lost”                                | mcp-remote pin                | Pinned package version + `MCP_REMOTE_CONFIG_DIR` stability                 |
| Typecheck fail in `smithery.ts`                 | Frontend skills               | Unrelated skill ID typing — fix before release if `pnpm check` is required |

Enable Rust logging via `RUST_LOG` where applicable; use Agent debug panel in
the UI for run snapshots when available.

---

## 7. Testing strategy

| Layer           | Command                        | What it guards                                             |
| --------------- | ------------------------------ | ---------------------------------------------------------- |
| Frontend unit   | `pnpm test`                    | IPC helpers, token estimate, display helpers               |
| Frontend static | `pnpm typecheck` / `pnpm lint` | Types and style                                            |
| Rust lib        | `cargo test --lib`             | Agent loop, DB round-trip, timeline, prompts, providers    |
| Manual smoke    | `pnpm tauri:dev`               | Hotkey, overlay↔workbench handoff, one Agent edit + Review |

**Must-pass themes before release**

- [ ] Ask mode cannot write files / run shell
- [ ] Agent edit appears in Review and can undo
- [ ] Thinking + tool cards stay interleaved after reload
- [ ] Cancel mid-stream settles message status
- [ ] Updater pubkey present; signed MSI builds when keys are set

---

## 8. Version bump checklist

When cutting `vX.Y.Z`:

1. Update version in **all three**:
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml` (then refresh `Cargo.lock` peek package version)
2. Update README badges / MSI filename examples if they hardcode the version.
3. Update release doc examples if desired.
4. Prefer UTF-8-safe editors for `Cargo.toml` (avoid PowerShell rewrites that
   corrupt the em dash in the description).
5. Follow [release.md](./release.md) for signing and `latest.json`.

---

## 9. Coding conventions

- Prefer small, reviewable diffs; match existing naming and module boundaries.
- Comments: explain **why**, never “inspired by &lt;vendor&gt;”.
- New `ChatMessage` fields: update struct, serde, SQLite migration, all literals
  (or helpers), frontend types, and persistence tests.
- New Bus events: define in `core/event`, project in adapters, subscribe in
  `main.ts` / ipc services, document in architecture §12.
- Do not commit `__pycache__`, secrets, or local `release/*.json` with live signatures.

---

## 10. Prompt & skill maintenance

| Asset                                                   | Rule                                                |
| ------------------------------------------------------- | --------------------------------------------------- |
| `system.md` / `tools.md` / `policies.md` / `context.md` | Stable core; example-rich; no skill product names   |
| `compact-summary.md`                                    | Compaction LLM format only                          |
| `skills/*.md`                                           | Self-contained routing + procedure                  |
| Prompt size tests                                       | May raise limits deliberately; keep isolation tests |

After large prompt edits, run the prompt structure tests and a manual Agent turn
that exercises the changed skill.

---

## 11. Security & privacy hygiene

- Never log API keys, OAuth refresh tokens, or full user documents in telemetry.
- Path permissions and approval modes are security controls — treat regressions
  as sev-high.
- Shell tools run on the user’s machine; default approval should remain
  conservative unless the user opts into “always allow”.
- Third-party MCP / search / mem0: document in Settings that data leaves the device.

Threat-model notes for Tauri itself may live under `src-tauri/.agents/skills/tauri/`;
product threat decisions belong in this guide and Settings copy.

---

## 12. Incident response (short)

1. **Reproduce** with a minimal session (new Quick Ask).
2. **Capture**: app version, provider, Ask/Agent mode, whether workspace-bound.
3. **Classify**: UI-only vs domain loop vs provider vs persistence.
4. **Mitigate**: ship hotfix on `AgentRunner` / store / migration; avoid UI-only
   patches for ordering bugs that need `work_timeline`.
5. **Verify** with Rust unit coverage for the root cause.

---

## 13. Useful commands cheat sheet

```powershell
# Dev
pnpm install
pnpm tauri:dev
pnpm check
cd src-tauri; cargo test --lib

# Build installer
pnpm tauri:build

# Release metadata
pnpm release:json -- --tag v0.2.1 --notes "…"

# Sync external skill vendors (when used)
pnpm sync-skills
```

---

## 14. Document ownership

| Doc                       | Update when…                                    |
| ------------------------- | ----------------------------------------------- |
| `architecture-overview.*` | Layers, loops, events, persistence model change |
| `maintenance.*`           | Dev workflow, checklists, data locations change |
| `release.*`               | Signing, CI, updater URL change                 |
| Root `README*`            | User-facing capabilities or install path change |

Keep EN and ZH docs in lockstep for structural sections; screenshots may be shared.
