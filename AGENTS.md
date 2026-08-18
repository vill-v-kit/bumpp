## 仓库布局

Rust 代码按三个顶层目录分层：

- `crates/` — 纯 Rust 库 crate（不依赖 napi，可独立 `cargo test`）
- `napi/` — **内部机制包**：Rust↔Node 的 napi 绑定包及其平台二进制分发包。**判别标准是受众而非是否发布**——"用户会直接 npm install 它吗？"不会，就放这里
- `npm/` — **面向用户的 npm 包**（用户直接安装使用的包）

**受众规则：`npm/` 与 `napi/` 按受众分流，与是否发布无关。** 内部机制包即使发布到 npm（例：`napi/bumpp-core` 即 `@vill-v/bumpp-core`，以及 7 个平台二进制包）也放 `napi/`——它们发布只是因为 npm 不支持 workspace 协议与 optionalDependencies 分发机制，用户没有直接安装它们的场景。本仓库**没有 `packages/` 目录**。

平台二进制包目录（`napi/<triple>`，如 `napi/linux-x64-musl`）**由 `pnpm create:npm-dirs` 从 `napi.targets` 生成、gitignore 不提交**；fresh clone 无这些目录，`optionalDependencies` 的 `workspace:*` 被 pnpm 静默跳过，loader 走本地 `.node` fallback。

非 Rust 的顶层目录：

- `website/` — 面向用户的文档网站（fumadocs / Next.js 静态导出）；与 `docs/` 的工程内部文档（ADR、agent 约定）分离，不进 cargo/pnpm 的既有 glob，pnpm-workspace 以精确名 `website` 加入
- `docs/` — 工程内部文档（`adr/`、`agents/`、迁移指南源稿）

配套约定：

- 根 `Cargo.toml` 是虚拟 workspace；`members` 只声明当前非空的目录 glob（cargo 对零匹配的 glob 报错），新增 `crates/`、`napi/` 目录时同步加入
- cargo 对 glob 匹配到但没有 `Cargo.toml` 的目录也报错：`npm/`、`napi/` 下所有无 `Cargo.toml` 的包（纯 JS 包、平台二进制包等）必须同步加进根 `Cargo.toml` 的 `exclude`
- `[profile.*]` 只在根 workspace 清单生效，成员 crate 内不写
- 成员 crate 之间的引用一律走根 `[workspace.dependencies]` 声明 + 成员内 `xxx.workspace = true` 继承，成员清单里不写 `path`
- 版本号唯一维护点是根 `[workspace.package].version`，成员 crate 一律 `version.workspace = true` 继承（成员清单不写版本字面量）；发版**不配置 files**——根 `Cargo.toml` 已在默认清单（链上 manifest basenames 根级并集），显式 files 会顶替默认清单反而漏掉根 `package.json`（浅替换语义）；嵌套 npm/napi 包版本由 `-r` 整树收集覆盖；private 包（website、根 workspace）随整树一并锁步是既定行为，不按 `"private": true` 排除
- 仓库脚本（`scripts/`、`website/scripts/`、napi 冒烟等维护用命令行脚本）一律 TypeScript 由 node 原生直跑、不经编译步骤，语法与 import 约束见 `docs/agents/scripts.md`

## Git 提交信息

Git 提交的标题和正文均不写 ADR 编号或 ADR 文件路径，也不以“新增/更新 ADR”作为变更主题；也不写需求管理器编号（如 Linear 工单号）——唯一例外是 GitHub 自身 issue 号（如 `#123`，可被 GitHub 自动关联）；应直接描述本次确定或改变的实际行为、约束或架构。

提交标题即 changelog 条目：本仓库 CHANGELOG 由提交标题自动生成、面向用户阅读，标题必须是用户看得懂的文案——按用户可感知的口径写「什么变了、带来什么效果」（如「文件层键名与类型双重校验——类型不符报错指出键路径与期望类型」），不写只有翻代码才能懂的实现口径（模块/函数改名、内部机制代号、重构手法）。类型前缀决定 changelog 分组（映射见 `.vbumpprc.toml` 的 `changelog.types`），按用户意义选型：用户可感知的行为变化选 feat/fix。例外：AI 相关配置与文案的变更（AGENTS.md、`docs/agents/` 等只面向 agent 的内容）用户在 changelog 里看到无意义——提交信息只用 `chore(agent):`（配置类）或 `docs(agent):`（文案类）两个 scope（如 `chore(agent): update AGENTS.md`）；这两个 scope 已由 `.vbumpprc.toml` 的 `changelog.types.X.excludeScopes` 统一排除、不进 CHANGELOG（breaking 提交除外）。

## Agent skills

### Issue tracker

Issues are tracked in Linear under the `villv-bump` project. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the five canonical triage labels without overrides. See `docs/agents/triage-labels.md`.

### Rust crate structure

Rust crate 的模块入口风格（无 `mod.rs`）、单文件行数软上限（src 500 / tests 1000）、tests 镜像 src 等结构约定见 `docs/agents/rust-structure.md`。

### Domain docs

Use the single-context layout with `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
