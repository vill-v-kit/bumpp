# vill-v/bump

版本发布工具集：以 Rust 核心驱动版本号更新、changelog 生成与多平台（GitHub/GitLab/Gitee/GitCode）release 创建。

## Language

**Bump**:
一次完整的版本发布操作——更新各文件中的版本号、生成 changelog、git commit/tag/push、执行 npm scripts。
_Avoid_: release（指平台 release 创建）、publish

**Release type**:
决定新版本号如何计算的选择项：`major` / `minor` / `patch` / `next` / `conventional` / `premajor` / `preminor` / `prepatch` / `prerelease`，以及 prompt 中的 `none`（保持不变）和 `custom`（手动输入）。
_Avoid_: bump type、version type

**Preid**:
预发行标识符（如 `1.0.0-beta.1` 中的 `beta`）。计算候选版本时沿用当前版本的预发行标识，否则用入参（上游 normalizeOptions 缺省为 `'beta'`）；预发行号从 1 开始（上游的 `0→1` 修正）。

**内部机制包 (Internal machinery package)**:
虽然发布到 npm、但设计上就不是给用户直接使用的包——判别问句："用户会直接 npm install 它吗？"不会即归 `napi/` 目录（ADR-0005，与是否发布无关）。成员：Core（napi 绑定本体）、Platform package（平台二进制分发包）；它们发布仅因 npm 不支持 workspace 协议与 optionalDependencies 按平台分发的机制需要。
_Avoid_: 内部包（过宽，`crates/` 亦属内部）、native package

**Core**:
纯 Rust + napi-rs 实现的版本引擎包 `@vill-v/bumpp-core`（`napi/bumpp-core`），对外提供三个与上游 bumpp v11 兼容的 API：`versionBump` / `versionBumpInfo` / `loadBumpConfig`。进度为 Rust 内置打印（ADR-0002），`ProgressEvent` 不向 Node 层导出。
_Avoid_: bumpp（指上游 antfu/bumpp 依赖本身）、next（实验包，已由 core 替代并删除）

**Platform package**:
按目标平台分发预编译 `.node` 二进制的 npm 包（如 `@vill-v/bumpp-core-darwin-arm64`，`napi/bumpp-core-darwin-arm64`），作为主包的 optionalDependencies 安装。
_Avoid_: native package、binary package
