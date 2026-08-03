## 仓库布局

Rust 代码按三个顶层目录分层：

- `crates/` — 纯 Rust 库 crate（不依赖 napi，可独立 `cargo test`）
- `napi/` — **内部机制包**：Rust↔Node 的 napi 绑定包及其平台二进制分发包。**判别标准是受众而非是否发布**——"用户会直接 npm install 它吗？"不会，就放这里（ADR-0005）
- `npm/` — **面向用户的 npm 包**（用户直接安装使用的包）

**受众规则：`npm/` 与 `napi/` 按受众分流，与是否发布无关。** 内部机制包即使发布到 npm（例：`napi/bumpp-core` 即 `@vill-v/bumpp-core`，以及 5 个平台二进制包）也放 `napi/`——它们发布只是因为 npm 不支持 workspace 协议与 optionalDependencies 分发机制，用户没有直接安装它们的场景。本仓库**没有 `packages/` 目录**。

配套约定：

- 根 `Cargo.toml` 是虚拟 workspace；`members` 只声明当前非空的目录 glob（cargo 对零匹配的 glob 报错），新增 `crates/`、`napi/` 目录时同步加入
- cargo 对 glob 匹配到但没有 `Cargo.toml` 的目录也报错：`npm/`、`napi/` 下所有无 `Cargo.toml` 的包（纯 JS 包、平台二进制包等）必须同步加进根 `Cargo.toml` 的 `exclude`
- `[profile.*]` 只在根 workspace 清单生效，成员 crate 内不写
- 成员 crate 之间的引用一律走根 `[workspace.dependencies]` 声明 + 成员内 `xxx.workspace = true` 继承，成员清单里不写 `path`
- 版本号唯一维护点是根 `[workspace.package].version`，成员 crate 一律 `version.workspace = true` 继承（成员清单不写版本字面量）；发版清单（`.vbumpprc.toml` 的 files）只列根 `Cargo.toml`

## Agent skills

### Issue tracker

Issues are tracked in Linear under the `villv-bump` project. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the five canonical triage labels without overrides. See `docs/agents/triage-labels.md`.

### Domain docs

Use the single-context layout with `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
