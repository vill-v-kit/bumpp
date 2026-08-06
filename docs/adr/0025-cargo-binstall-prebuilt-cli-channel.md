# cargo-binstall 免编译安装渠道：GitHub Release 挂预编译 CLI 产物

`crates/vbumpp`（纯 Rust CLI，ADR-0019）已随 v6 上架 crates.io（ADR-0021），但 `cargo install vbumpp` 需全量编译。本 ADR 接通 cargo-binstall 免编译通路并立为一等安装渠道：binstall 从 crates.io 元数据的 `repository` 字段探测 GitHub Releases，按默认模板匹配预编译产物——只要 tag CI 把符合约定的产物挂上 Release，`cargo binstall vbumpp` 即免编译可用，无匹配平台时 binstall 自动回退 `cargo install` 源码编译兜底。反转 ADR-0021 Consequences 的「GitHub Release 预编译维持不做」。

## Decisions

- **渠道定位**：一等安装渠道，与 npm 并列；文档双落点——website 快速上手 + 给 vbumpp crate 补 README（crates.io 页面现无 `readme` 字段、内容空白）。
- **平台矩阵**：7 target——napi 既有 5 平台（darwin-arm64 / linux-x64-gnu / linux-arm64-gnu / win32-x64-msvc / win32-arm64-msvc）+ `x86_64/aarch64-unknown-linux-musl`。binstall 在 glibc 系统上同时探测 gnu 与 musl target，musl 静态产物兼服容器/Alpine 场景；darwin-x64（Intel Mac）不入矩阵，回退编译兜底。
- **产物约定（零 binstall 元数据）**：命名 `vbumpp-{target}.tar.gz`——命中 binstall 默认文件名模板 `{ name }-{ target }{ archive-suffix }`（无版本号变体，与 cargo-dist / taiki-e 惯例同款）；归档内顶层目录 `vbumpp-{target}/` 放二进制——命中默认 bin-dir 探测；`tgz` 即默认 pkg-fmt；本仓 tag 约定 `v{version}` 在默认探测路径（`releases/download/v{ version }/`）内。故 `crates/vbumpp` 的 Cargo.toml **不写任何** `[package.metadata.binstall]` 键。
- **checksums**：每产物附 `.sha256` 一并上传。binstall 不消费 sha256 文件（其签名验证仅支持 minisign 且需 signing 元数据段）——定位为用户手动校验的防篡改/防截断凭据。
- **CI 结构**：新增独立 `build-cli` job，7 target matrix，不与 napi build 线纠缠；交叉场景（musl×2、linux-arm64-gnu）统一走 **cargo-zigbuild**（mise 装 zig），darwin/win 原生 cargo build。
- **上传与失败语义**：`publish-crates` 之后以 `gh release upload --clobber` 把产物追加到 bump 流程已建的 GitHub Release（Release 由 bump 末段自动创建，body 为 changelog）；**硬失败**——产物缺失令流水线红，靠 `--clobber` 幂等 + 「Re-run failed jobs」收敛，恢复模型与 ADR-0021 同款。
- **不回填**：v6.0.0 及以前不补挂产物，下一 tag 起生效。

## Considered Options

- **扩展既有 napi build matrix（加 kind 字段）**：checkout/mise 步骤复用，但两线工具链需求不同（napi-cross vs zigbuild）、条件分支多、失败面混杂——拒绝，独立 job 隔离。
- **cross（docker 驱动）**：GitHub runner 自带 docker 可用，但镜像拉取慢、本地复现需 docker——拒绝。
- **原生 gcc/musl-tools + RUSTFLAGS**：零新工具，但每个交叉 target 一套环境变量与 linker 配置，脚本最脆——拒绝；cargo-zigbuild 一个工具统一 musl 与 arm64-gnu 交叉。
- **softprops/action-gh-release / taiki-e/upload-rust-binary-action**：声明式或自带 binstall 友好命名，但各引入第三方 action 信任面，且与本仓手写脚本族（`scripts/*.mjs`）风格不一——拒绝，gh CLI 直白可控。
- **cargo-dist**：开箱即用且产物约定天然兼容 binstall，但引入整套发布编排工具，与既有 tag CI + 手写脚本体系重叠——拒绝，手写 7 target 构建可控且复用既有恢复模型。
- **minisign 签名（checksums + signing 元数据段）**：binstall 可验，但密钥对管理是新维护面且签名验证非 binstall 默认强制——降级为仅 checksums；未来有供应链要求时可纯增量补 signing 段。
- **自定义命名 + `[package.metadata.binstall]`**：命名自由，但多一处随 binstall 版本演进需维护的清单配置——拒绝，默认模板零元数据。

## Consequences

- ADR-0021 Consequences「GitHub Release 预编译维持不做」由本 ADR 反转（已就地注记修订指针）。
- `build-cli` 红 = 发版不完整：一等渠道不允许静默缺产物；bump 已建 Release 与 npm/crates 上架不受影响，re-run 收敛。
- 产物命名/目录结构成为对 binstall 默认模板的**隐式契约**：未来改动归档命名、顶层目录或 tag 约定，必须同步补 `[package.metadata.binstall]`，否则渠道静默退化为全量编译且无人报错——改动时以 `cargo binstall --manifest-path crates/vbumpp/Cargo.toml vbumpp` 本地验证。
- `crates/vbumpp` 补 README 字段与文件；website 快速上手安装段增列 cargo-binstall。
- 平台 Release 的职能扩宽：除 changelog 承载体外，新增预编译产物分发载体（bump 流程创建不变，CI 仅追加 assets）。
