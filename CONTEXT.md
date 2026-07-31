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
预发行标识符（如 `1.0.0-beta.1` 中的 `beta`）。计算候选版本时默认沿用当前版本的预发行标识，否则为 `'preid'`；预发行号从 1 开始（上游的 `0→1` 修正）。

**Core**:
纯 Rust + napi-rs 实现的版本引擎包 `@vill-v/bumpp-core`（`packages/core`），对外提供与上游 bumpp v11 兼容的四个 API：`versionBump` / `versionBumpInfo` / `loadBumpConfig` / `ProgressEvent`。
_Avoid_: bumpp（指上游 antfu/bumpp 依赖本身）、next（实验包，将由 core 替代后删除）

**Platform package**:
按目标平台分发预编译 `.node` 二进制的 npm 包（如 `@vill-v/bumpp-core-darwin-arm64`），作为主包的 optionalDependencies 安装。
_Avoid_: native package、binary package
