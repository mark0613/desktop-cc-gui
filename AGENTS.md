<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

Use the `/trellis:start` command when starting a new session to:
- Initialize your developer identity
- Understand current project context
- Read relevant guidelines

Use `@/.trellis/` to learn:
- Development workflow (`workflow.md`)
- Project structure guidelines (`spec/`)
- Developer workspace (`workspace/`)

If you're using Codex, project-scoped helpers may also live in:
- `.agents/skills/` for reusable Trellis skills
- `.codex/agents/` for optional custom subagents

Keep this managed block so 'trellis update' can refresh the instructions.

<!-- TRELLIS:END -->

## 项目工作基线（mossx）

- 文档与规则统一使用“中文主体 + English technical terms”。
- 规则优先级：当前项目代码实现 > 项目内文档（`AGENTS.md` / `.trellis/spec` / `openspec/`）> 全局 `~/.codex/rules` / 全局 `~/.codex/AGENTS.md`。
- 当前项目同时包含：
  - frontend：`src/**`
  - backend：`src-tauri/src/**`
  - behavior spec：`openspec/**`
  - code spec：`.trellis/spec/**`
  - task workspace：`.trellis/tasks/**`

## 规范读取顺序（Session Start Order）

每次开始任务，按以下顺序建立上下文：

1. 先读项目内：`.claude/`、`.codex/`、`AGENTS.md`、`.trellis/spec/**`、`openspec/**`
2. 再读当前任务直接相关的实现文件与 config：
   - `package.json`
   - `tsconfig.json`
   - `.eslintrc.cjs`
   - `src/services/tauri.ts`
   - `src/services/clientStorage.ts`
   - `src-tauri/src/command_registry.rs`
   - `src-tauri/src/state.rs`
   - `src-tauri/src/storage.rs`
3. 只有项目内信息不足时，才补充参考全局 `~/.codex/rules/*` 与全局 `~/.codex/AGENTS.md`

## OpenSpec + Trellis 协作约定（团队共享）

- `openspec/` 是 behavior / proposal / change 的单一事实源（single source of truth）。
- `.trellis/spec/` 是 code-level implementation rule 与 executable contract 的沉淀位置。
- `.trellis/tasks/` 是任务执行容器，目录必须保留；即使暂时为空，也保留 `.gitkeep`，不要删除。
- 每个 Trellis task 都必须关联一个 OpenSpec change，保证任务与提案可追溯。
- 涉及行为变更、产品交互、跨层 contract 变更时：
  1. 先在 OpenSpec 创建或选择 change
  2. 再进入 Trellis / implementation
  3. 实现后同步更新 `.trellis/spec`
- 对未安装 OpenSpec/Trellis CLI 的协作者：提交这些文件不会影响代码运行；但 PR 仍应注明关联的 OpenSpec change 与任务映射。

## Trellis Session Record Gate

- 当 AI 在本仓库完成一次代码或规范提交后，必须立即执行 Trellis session record，将本次交付写入当前 active developer 对应的 `.trellis/workspace/<developer>/`。
- 这是 **AI commit workflow invariant**，不是“可选收尾动作”：只要 AI 在本仓库成功执行了 `git commit`，就不得直接结束回合或切回普通对话，必须继续尝试执行 Trellis session record，除非用户明确要求跳过。
- 记录时机：仅在 commit 已完成后执行；普通问答、方案讨论、未提交的中间态不写入 workspace，避免 journal 变成聊天流水账。
- 适用范围：所有开发者 workspace 都遵守同一规则，包括 `.trellis/workspace/chenxiangning/` 与 `.trellis/workspace/zhukunpenglinyutong/`。
- 前置检查：执行 record 前必须先运行 `python3 ./.trellis/scripts/get_context.py --mode record`。
- 如果提示 `Not initialized`，必须先根据当前使用者初始化 developer，例如：
  - `python3 ./.trellis/scripts/init_developer.py chenxiangning`
  - `python3 ./.trellis/scripts/init_developer.py zhukunpenglinyutong`
- 执行方式：
  1. 获取刚完成的 commit hash、标题、主要改动与验证结果。
  2. 运行 `python3 ./.trellis/scripts/add_session.py --stdin --title "..." --commit "<hash>"`。
  3. stdin 内容必须包含：任务目标、主要改动、涉及模块、验证结果、后续事项。
- `add_session.py` 会自动更新 `.trellis/workspace` 并提交 Trellis 元数据，因此一次业务提交后通常会跟随一个独立的 Trellis session 记录提交。
- 如果用户明确要求“不记录 Trellis session”，则跳过记录，并在最终回复中说明。
- 禁止把该流程放入 Git `post-commit` hook；该脚本会产生提交，hook 方案容易导致递归提交和低质量记录。
- 对任何处理 Git commit 的 skill / workflow（包括但不限于 `git-flow`、自定义 commit helper、直接 shell commit）都必须遵守这一规则；如果 skill 自身未显式提及，仍以本项目规则为准自动补执行。

## PlanFirst 执行约束

- 任何代码、配置、规范落盘前，先给出 `PLAN` 或等价的执行步骤。
- 如果当前任务已经进入 OpenSpec workflow，则使用 OpenSpec artifact 作为 plan 载体。
- 未确认前不做超出范围的额外落盘；需求范围一旦扩大，重新输出 `PLAN`。

## Frontend 执行入口

开始改 frontend 前，至少按需阅读：

- `.trellis/spec/frontend/index.md`
- `.trellis/spec/frontend/directory-structure.md`
- `.trellis/spec/frontend/component-guidelines.md`
- `.trellis/spec/frontend/hook-guidelines.md`
- `.trellis/spec/frontend/state-management.md`
- `.trellis/spec/frontend/quality-guidelines.md`
- `.trellis/spec/frontend/type-safety.md`

Hard rules：

- frontend -> runtime command 统一走 `src/services/tauri.ts`，不要在 feature hook / component 内直接 `invoke()`
- persistent UI state 统一走 `src/services/clientStorage.ts`
- user-visible copy 必须走 i18n
- 修改大 CSS 文件或接近阈值的大文件时，必须跑 `npm run check:large-files`

## Backend 执行入口

开始改 backend 前，至少按需阅读：

- `.trellis/spec/backend/index.md`
- `.trellis/spec/backend/directory-structure.md`
- `.trellis/spec/backend/error-handling.md`
- `.trellis/spec/backend/logging-guidelines.md`
- `.trellis/spec/backend/database-guidelines.md`
- `.trellis/spec/backend/quality-guidelines.md`

Hard rules：

- `#[tauri::command]` 注册统一收口在 `src-tauri/src/command_registry.rs`
- 共享状态统一遵循 `src-tauri/src/state.rs` 中的 `AppState` 锁模型
- 文件持久化复用 `storage.rs` / `client_storage.rs` 的 `lock + atomic write`
- runtime path 禁止 `unwrap()` / `expect()`
- command payload / response 改动后，必须同步检查 frontend `src/services/tauri.ts`

## Cross-Layer 触发器

出现以下任一情况，必须额外阅读 `.trellis/spec/guides/cross-layer-thinking-guide.md`：

- 修改 `src/services/tauri.ts`
- 修改 Tauri command 名、参数、字段名
- 修改 persistent storage 字段结构
- 修改 polling、listener、session、workspace、spec-hub、git-history 等主链路行为

若发现相似逻辑已出现 2 次以上，额外阅读 `.trellis/spec/guides/code-reuse-thinking-guide.md`。

## 标准验证命令（Quality Gates）

基础门禁：

```bash
npm run lint
npm run typecheck
npm run test
```

涉及 runtime contract / app doctor：

```bash
npm run check:runtime-contracts
npm run doctor:strict
```

涉及大文件 / 样式重构：

```bash
npm run check:large-files
```

涉及 Rust backend：

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Merge Guardrails

- 高风险文件冲突时，禁止整文件 `--ours` / `--theirs` 覆盖。
- 先列 capability matrix，再做语义融合（semantic merge）。
- 至少验证相关 symbol、测试、contract command，确认双方能力都还在。

## Shell 命令基线

- 遇到 `command not found`，不要直接判断“未安装”。
- 先用：

```bash
zsh -lc 'source ~/.zshrc && <command>'
```

- 仍失败再排查：

```bash
zsh -lc 'source ~/.zshrc && which <command> && echo $PATH'
```
