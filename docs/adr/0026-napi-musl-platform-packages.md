# napi 平台矩阵扩 musl ×2：修 Alpine npm 安装硬失败

Alpine 容器（node:*-alpine，musl libc）里 npm 安装 `@vill-v/bumpp` 今天硬失败——loader 报 cannot load 且矩阵内无 musl 可匹配。本 ADR 将 napi 平台矩阵扩 `x86_64/aarch64-unknown-linux-musl` 两个 target，与 ADR-0025 的 binstall 矩阵（7 target）对齐；npm 包 11→13。napi-rs 对 musl 为一等支持（swc/oxlint/biome 均发 musl 平台包），本仓手写 loader 从 optionalDependencies 派生 SUPPORTED_TARGETS 且 musl 探测逻辑预埋已久——加包即生效，loader 零逻辑改动。

## Decisions

- **动机定性**：修真实硬失败（Alpine 容器 npm 安装场景），非纯形式对齐；binstall CLI 不能替代编程式 API 用户。
- **新增平台包**：`@vill-v/bumpp-core-linux-x64-musl` 与 `-linux-arm64-musl`（napi 官方 `<platform>-<arch>-<abi>` 命名惯例，swc/oxlint/biome 同款），声明 `os/cpu/libc: ["musl"]` 字段；进主包 optionalDependencies（`workspace:*`）、根 `Cargo.toml` exclude、CI build matrix。
- **构建通路**：`napi build --cross-compile`（内部路由 cargo-zigbuild，CLI 自动为 musl 追加 `-C target-feature=-crt-static`——cdylib 在 musl 可加载的关键）；zig 经 mise 安装，与 ADR-0025 build-cli 同源复用。musl 两 target 进既有 napi build matrix（非独立 job，CLI 线/napi 线分立维持）。`--use-napi-cross` 事实不覆盖 musl（仅 glibc 工具链），不用。
- **libc 字段语义**：npm ≥10.3.0 / pnpm ≥7.1.0 原生过滤；本仓 engines 内 Node 自带 npm 均 ≥10.3；旧包管理器静默双装 gnu+musl，loader 探测兜底选对（仅浪费带宽，行为正确）。
- **测试**：Alpine 容器 smoke（docker node:alpine：装包 + 加载 + `--version` 级验证），与 test-bindings 同级新 job；x64 必测，arm64 经 qemu 尽力、失败则记为已知限制。
- **首发认证**：2 个新包受 ADR-0021 记录的约束（trusted publisher 须包已存在才能配置）——重演其混合路线：一次性长效 NPM_TOKEN 首发 → npmjs 配置 2 个 trusted publisher → 撤 token。回收尾巴列入实施票验收。
- **排序**：blocked by COL-67（ADR-0025 build-cli）——zig 工具链与 musl 交叉构建经验（含 vendored openssl 解法）由 CLI 线先行趟出。

## Considered Options

- **不做 musl napi（ADR 记不支持，Alpine 用户走 binstall CLI）**：loader 报错虽可读，但 npm 渠道在 Alpine 断裂是真实用户失败；编程式 API 用户无 CLI 可退——拒绝。
- **仅 musl x64**：arm64 Alpine 场景稀少，但 zigbuild 多一个 target 边际成本极低，且 binstall 已 ×2——拒绝，×2 对齐。
- **docker `cross` / alpine 构建镜像**：napi 官方已弃用 nodejs-rust alpine 镜像路线，`-x` + zig 是现行推荐——拒绝。
- **npm provenance 首发新包**：sigstore 通路对全新包的可行性未验证；ADR-0021 token 通路已被 v6 发版验证——拒绝，选一次性 token。
- **人工本地首发 2 包**：破坏「tag 推送即授权」全自动语义，操作窗口脆弱——拒绝。
- **Alpine 全套 vitest**：最彻底但镜像内装 pnpm + 全依赖太重；smoke 已覆盖 libc 过滤与 musl 加载全链路——拒绝。

## Consequences

- npm 包 11→13（CONTEXT.md「上架」计数同步）；publish 守卫/拓扑序由既有 `pnpm -r` + skip-if-published 自动覆盖，pack + publint 既有 glob 自动纳入新包。
- loader 零逻辑改动；行 42「musl 不在支持矩阵内」注释过时，实施时更新表述（该 skip 转为防旧包管理器双装的兜底）。
- **实现风险**：ADR-0024 的 vendored openssl 使 musl 交叉需 zig cc 编译 C 码，可能要 `TARGET_CC` 类环境变量——COL-67 的 CLI musl 构建先踩，本线复用其解法。
- ADR-0021 混合路线在新包维度重演一次（一次性 token），非安全态势回归；落地后 trusted publisher ×2 + 撤 token 是必须闭环的尾巴。
- CI build matrix 5→7；发版包数、LICENSE 同步（check:licenses）等既有 glob/脚本自动覆盖。
- website 平台支持表述随实施同步更新。
