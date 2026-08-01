# 贡献指南

## 环境准备

仓库使用 [mise](https://mise.jdx.dev/) 统一管理工具链（Node 与 Rust，版本声明见根 `mise.toml`）：

```shell
mise install     # 安装 node lts + rust stable（含 rustfmt/clippy）
pnpm install     # 安装 workspace 依赖
```

pnpm 由 nub（`nub pm`）或你自己的方式提供，版本以根 `package.json` 的 `packageManager` 字段为准（CI 用 `pnpm/action-setup` 读取同一字段）。

## 日常开发

```shell
pnpm build       # turbo 全量构建（含 Rust→napi 本机 target，core 先于 bump）
pnpm test        # 先 turbo build 再跑 vitest（根 pretest 保证干净 clone 可用）
cargo test --workspace   # Rust 侧测试（版本计算矩阵、文件更新、git 临时仓库等）
cargo fmt --all && cargo clippy --workspace --all-targets
```

## 仓库布局约定

- `crates/` — 纯 Rust 库 crate（不依赖 napi，可独立 `cargo test`）
- `napi/` — 不发版的 napi 绑定包
- `npm/` — 最终要发版的 npm 包（优先级高于 `napi/`：会发版的绑定包直接放这里）

配套规则（详见 [AGENTS.md](./AGENTS.md)「仓库布局」）：

- 根 `Cargo.toml` 是虚拟 workspace；新增 `crates/`、`napi/` 目录时同步加入 `members` glob
- `npm/` 下无 `Cargo.toml` 的包（纯 JS 包、平台二进制包）必须加入根 `Cargo.toml` 的 `exclude`
- `[profile.*]` 只写在根 workspace 清单
- Rust 代码统一两空格缩进（根 `rustfmt.toml`）

## 测试约定

双层测试：

- **cargo test**：纯 Rust 层的 parity spec（期望值多由真实 node-semver / 上游 bumpp 复刻脚本生成）
- **vitest**：经 napi 缝隙对编译产物 `.node` 做集成测试（临时 git 仓库、与上游逐字节/逐值对比）

## CI

GitHub Actions（`.github/workflows/ci.yml`）**仅在 `v*` 版本 tag 推送时触发**：5 个 target 的原生 runner 构建矩阵 + macOS / Ubuntu / Windows 的 test-bindings。日常直推 main 不触发；未来发版时在同一 workflow 挂 publish job。

## 已知事项

### Cargo.toml 版本线漂移点

`crates/bumpp-core/Cargo.toml` 与 `npm/bumpp-core/Cargo.toml` 都携带 `version` 字段，而 `vbumpp -r`（发布流程）只更新各 `package.json` 的版本号。未来实际发版时需**手动同步两处 Cargo.toml 的版本**，或在发布流程中引入同步机制（如 vbumpp 支持 Cargo.toml 版本字段更新）。

### 本阶段不发版

Rust 重写阶段已完成，但按 ADR-0001 决议**不随此发版**：CHANGELOG 迁移说明与 major 版本动作留待后续变更（oxc 加载 TS 配置、changelog Rust 化等）收敛后，随未来实际发版一并处理。
