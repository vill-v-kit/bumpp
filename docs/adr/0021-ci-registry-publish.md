# CI 上架通路：tag 触发、双 registry 并行、token 首发后迁 OIDC

website 安装指南已双渠道承诺（npm + crates.io），`ci.yml` 亦预留「未来发版时在此挂 publish job」坑位——本 ADR 接通该通路：11 个 npm 包（5 用户包 + 6 内部机制包）与 2 个 crate（`vbumpp-core` 连带上架、`vbumpp`，落实 ADR-0019 的既定方向）随同一版本 tag 全自动上架，首发用长效 token、落地后迁 OIDC trusted publishing。

## Decisions

- **挂点与拓扑**：`publish-npm` 与 `publish-crates` 均挂进现有 `ci.yml`，`needs` 门控在 test + build + test-bindings 全绿之后，两 job **并行**（registry、凭证、产物来源皆独立）；**无人工批准门**——tag 只能由本地 `pnpm release` 刻意产生，tag 推送即授权。
- **publish-npm**：下载 build job 的 5 个 `.node` artifact 注入各平台包目录 → 断言 tag 版本 == 根 workspace 版本 → `pnpm -r pack` + publint 前置验证 → **skip-if-published 守卫**（`npm view <pkg>@<version>` 已存在即跳过）→ `pnpm publish -r --no-git-checks`。`workspace:*` 协议转换与拓扑序（平台包 → core → 用户包）由 pnpm 原生处理；`website` 为 private 自动跳过。
- **publish-crates**：`cargo publish --dry-run` 前置 → skip-if-published 守卫（查 crates.io）→ 先 `vbumpp-core` 后 `vbumpp`。根 `[workspace.dependencies]` 的 vbumpp-core 条目补**宽松 `version = ">=5.1, <7"`**（cargo publish 要求 path 依赖带版本；cargo 会校验 path 依赖版本与 spec 匹配，^6 在 5.x 现世连本地编译都不可解析，故下界放到 5.1、上界卡 major——crates.io 上 vbumpp-core 只会有锁步发版的 ≥6.0.0，解析必落同代；v7 才需动一次——绕开 vbumpp 尚不维护 workspace.dependencies 版本字段的缺口）。
- **认证（混合路线）**：首发用长效 token——secrets 挂 `NPM_TOKEN`（granular）与 `CARGO_REGISTRY_TOKEN`；npm/crates.io 的 trusted publishing 均要求包已存在才能配置，全新包无法 OIDC 首发。v6 落地后逐包（11 npm 包 + 2 crate）在 registry 侧配置 trusted publisher，CI 撤 token 转 OIDC + `--provenance`；workflow 自始预留 `id-token: write` 权限。
- **幂等与恢复**：两 job 均可任意次重跑，已上架版本自动跳过——GitHub「Re-run failed jobs」是部分失败后的**唯一**恢复手段，不做手动补发、不做回滚（crates.io 上架不可撤）。

## Considered Options

- **独立 `publish.yml`**：publish 可单独重跑，但 `.node` 产物需跨 workflow 传递，且违背与 docs.yml「tag 触发同构」的既有风格（ADR-0020）——拒绝；重跑需求由幂等守卫解决。
- **environment 人工批准门**：防误触，但 tag 不可能误触产生，批准门防不住真实的部分失败风险——拒绝。
- **纯 OIDC trusted publishing**：无长效凭证最干净，但 trusted publisher 配置挂在 registry 的包设置页，11 个全新 npm 包 + 2 个新 crate 首发时无从配置——物理不通，降为迁移终点而非起点。
- **napi-rs 官方 flow（`napi artifacts` + 逐包 `npm publish`）**：只覆盖 core + 平台包，5 个用户包需另写一套，且模板已从本仓 package.json 删净——拒绝，`pnpm publish -r` 原生能力恰好全覆盖。
- **串行 crates → npm**：「核心库先上架」语义好看，但两 registry 失败域独立，串行买不到一致性、只拉长发版时间——拒绝。
- **workspace dep 精确版本（`6.0.0`）**：每次发版需同步该字段，而 vbumpp 只维护 `[package].version` 与 `[workspace.package].version`（ADR-0003）——拒绝，宽松 spec 绕开。
- **CLI crate 沿用 ADR-0019 原名 `bumpp-cli`、lib crate 沿用 `bumpp-core`**：家族感一致且零改动，但 `cargo install bumpp-cli` 装出的命令是 `vbumpp`——crate 名 ≠ bin 名的生态先例（ripgrep→rg、fd-find→fd）皆源于「好名被占」的无奈，而 `vbumpp` 在 crates.io 空闲；工具类 crate 的主流是 crate name == bin name（bat、eza、hyperfine）。更关键的是查证发现 **crates.io 的 `bumpp` 已被第三方占用且是同领域废弃工具**（"Bump version number in Cargo.toml"，v0.0.0）——`bumpp-*` 命名空间挂着一个会让用户混淆归属的他主 crate。——拒绝，2026-08 修订：crates.io 家族整体对齐 `vbumpp-*`——CLI crate 与目录更名 `vbumpp`（`crates/vbumpp`，安装名 == 命令名 == npm bin 名，三名合一），lib crate 与目录更名 `vbumpp-core`（`crates/vbumpp-core`）；顺带消除与 npm 侧 `@vill-v/bumpp-core`（napi 绑定包，与纯引擎是不同物）的跨 registry 同名歧义。napi 内部 crate `bumpp-core-napi` 与目录 `napi/bumpp-core` 不动（内部机制、永不发布、npm 包名不受影响）。

## Consequences

- `crates/vbumpp-core` 与 `crates/vbumpp` 移除 `publish = false`；`vbumpp-core` 以独立库 crate 身份公开占位，crates.io 名字占用不可撤销。
- 仓库 secrets 新增两枚长效 token；granular token 最长一年期，OIDC 迁移完成前需轮换——迁移尾巴由本 ADR 记死，防 token 变永久遗留。
- ADR-0014 注记的「发布前需复测交叉编译链」由 tag CI 的 5 平台 build + test-bindings 承担，publish 只在全绿后发生。
- 平台包 `.node` 注入的 `cp` 逻辑与 `test-bindings` job 同款；ADR-0019 的「接通发布通路」注记由本 ADR 兑现，GitHub Release 预编译维持不做。
- 守卫与上架编排沉淀为 `scripts/` 可测脚本族（publish-guard / npm-publish / crates-publish，vitest CLI 契约测试）；面向贡献者的发布流程描述见 `CONTRIBUTING.md`「CI」节（操作视图，与本 ADR 双向互查）。
