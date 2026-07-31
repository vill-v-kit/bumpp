## 仓库布局

Rust 代码按三个顶层目录分层：

- `crates/` — 纯 Rust 库 crate（不依赖 napi，可独立 `cargo test`）
- `napi/` — Rust↔Node 的 napi 绑定包，**不发布到 npm**
- `npm/` — 最终要发版的 npm 包

**优先级规则：`npm/` 高于 `napi/`。** 一个包即使当前只完成了 Rust↔Node 绑定，只要它最终要发版，就放到 `npm/` 下（例：`npm/bumpp-core` 即 `@vill-v/bumpp-core`）。

配套约定：

- 根 `Cargo.toml` 是虚拟 workspace；`members` 只声明当前非空的目录 glob（cargo 对零匹配的 glob 报错），新增 `crates/`、`napi/` 目录时同步加入
- cargo 对 glob 匹配到但没有 `Cargo.toml` 的目录也报错：往 `npm/` 添加纯 npm 包（如平台二进制包）时，必须同步加进根 `Cargo.toml` 的 `exclude`
- `[profile.*]` 只在根 workspace 清单生效，成员 crate 内不写

## Agent skills

### Issue tracker

Issues are tracked in Linear under the `villv-bump` project. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the five canonical triage labels without overrides. See `docs/agents/triage-labels.md`.

### Domain docs

Use the single-context layout with `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
