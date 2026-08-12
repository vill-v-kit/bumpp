# 贡献指南

## 环境准备

仓库使用 [mise](https://mise.jdx.dev/) 统一管理工具链（Node、Rust 与 zig 交叉链，版本声明见根 `mise.toml`）：

```shell
mise install     # node lts + rust stable（含 rustfmt/clippy）；linux 主机另装 zig / cargo-zigbuild（arm64 交叉链，ADR-0025）
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
- `napi/` — 内部机制包：napi 绑定包及其平台二进制分发包（判别标准是受众而非是否发布，ADR-0005）
- `npm/` — 面向用户的 npm 包（用户直接安装使用的包）

配套规则（详见 [AGENTS.md](./AGENTS.md)「仓库布局」）：

- 根 `Cargo.toml` 是虚拟 workspace；新增 `crates/`、`napi/` 目录时同步加入 `members` glob
- `npm/`、`napi/` 下无 `Cargo.toml` 的包（纯 JS 包、平台二进制包）必须加入根 `Cargo.toml` 的 `exclude`
- `[profile.*]` 只写在根 workspace 清单
- Rust 代码统一两空格缩进（根 `rustfmt.toml`）

## 测试约定

双层测试：

- **cargo test**：纯 Rust 层的 parity spec（期望值多由真实 node-semver / 上游 bumpp 复刻脚本生成）
- **vitest**：经 napi 缝隙对编译产物 `.node` 做集成测试（临时 git 仓库、与上游逐字节/逐值对比）

## CI

GitHub Actions（`.github/workflows/ci.yml`）**仅在 `v*` 版本 tag 推送时触发**；日常直推 main 不触发。tag 由本地 `pnpm release`（vbumpp `-r` 全树版本 bump）刻意产生，tag 推送即授权（ADR-0021）：

1. `test`（cargo fmt/clippy/test + vitest + `check:licenses`，先经 `pnpm create:npm-dirs` 生成平台包目录）→ `build`（7 平台 runner 矩阵，含 musl ×2 交叉腿）→ `test-bindings`（macOS / Ubuntu / Windows）全绿；
2. `publish-npm` 与 `publish-crates` 并行自动**上架**，无人工批准门——npm 侧 `pnpm create:npm-dirs` 生成 7 平台包目录、`napi artifacts` 注入 `.node` 与 `index.d.ts`（ADR-0029）、断言 tag 版本 == 工作区版本、`pnpm -r pack` + publint 前置验证后 `pnpm publish -r`（13 包）；crates 侧对称断言后按 `vbumpp-core` → `vbumpp` 硬序 `cargo publish`（cli 依赖 core，各自 dry-run 先行）；
3. 任一 publish job 部分失败时，「Re-run failed jobs」即唯一恢复手段——`scripts/publish-guard.mjs` 的 skip-if-published 守卫让已上架版本自动跳过、未上架版本补发，两 job 可任意次重跑收敛。

认证走 repo secrets（`NPM_TOKEN` / `CARGO_REGISTRY_TOKEN` 长效 token）首发，上架后迁 OIDC trusted publishing（ADR-0021 决策④）。设计与决策依据见 [ADR-0021](./docs/adr/0021-ci-registry-publish.md)；「上架」为术语基准（CONTEXT.md，与 Bump、平台 Release 三者分立）。

`pnpm check:licenses`（test job 同步执行）校验各发版包的 `LICENSE` 与根逐字节一致——MIT 要求软件副本携带版权与许可文本，发版包即"副本"载体。新增发版包时必须从根复制 `LICENSE`；根 `LICENSE` 变更后须同步全部副本。

### tag 推送后 CI 未触发（COL-62 实例）

v6.0.0 首发时 GitHub 收到了 tag push（ref 创建成功），但其下游事件被丢失——两个 workflow 零 run、零 check-suite，上架静默不发生；删 tag 重推（同一 object）即恢复。此故障形态的唯一现场特征是 **tag 推送后 Actions 长时间无 CI run**。`pnpm release` 已内建 `scripts/verify-tag-ci.mjs` 自检（vbumpp 推送后轮询 Actions runs，无则告警并中断后续 build）；若告警或人工发现未触发，恢复手段：

```shell
git push origin :refs/tags/<tag>   # 删远端 tag
git push origin <tag>              # 原样重推，强制生成新 push 事件
```

重推后 CI 即重新触发；自检告警已中断 `pnpm release` 的后续 build，确认 Actions 有 run 后**人工补跑 `pnpm build`** 完成发版收尾（上架由 CI 执行，本地 build 仅为发版前置校验）。

自检脚本本身依赖 GitHub API 可达性——API 故障时以 exit 2 与「事件丢失」（exit 1）区分，此时按脚本 ERROR 输出中的 Actions 链接（`https://github.com/vill-v-kit/bumpp/actions`）人工核对后继续。

## 已知事项

### 本阶段不发版

Rust 重写阶段已完成，但按 ADR-0001 决议**不随此发版**：CHANGELOG 迁移说明与 major 版本动作留待后续变更（oxc 加载 TS 配置、changelog Rust 化等）收敛后，随未来实际发版一并处理。
