# Rust + napi 核心

Bump 的版本计算、文件更新、changelog、git 操作与平台 Release 能力由纯 Rust 核心实现，并通过 napi-rs 向 Node.js 暴露稳定的薄壳接口。核心包位于 `napi/bumpp-core`，面向用户的 Node 包位于 `npm/bump`；原生 CLI `crates/vbumpp` 与 npm bin 共享 Rust 的 `run_from_argv` 入口。

## Decisions

- 核心使用纯 Rust 实现，Node 侧只负责加载原生模块、传递参数和暴露用户包入口。
- napi 面提供 `bumpVersion(options, provider?)` 与 `cliRun(argv, provider?)`；provider 可由平台变体包注入，CLI 的 `--provider` 优先于注入值。独立的 `release` 子命令由 Rust CLI 承担。
- 版本计算、配置加载、changelog 与进度等内部能力不向 Node 层暴露多余的上游兼容面；进度由 Rust 内置打印（见 ADR-0002）。
- 配置由 Rust 统一加载项目级 `.vbumpprc.{json,jsonc,toml}` 与全局 `~/.vbumpp/config.{json,jsonc,toml}`，不执行 TypeScript 配置函数。
- git 操作通过用户环境中的 `git` 命令执行，以继承 git config、SSH、GPG 与 credential helper；外部依赖不嵌入 JS 运行时。
- 发布使用预编译平台包及 optionalDependencies 分发；目标平台由 napi-rs CI 构建。
- `napi/` 收纳内部机制包，`npm/` 收纳面向用户的包，具体受众规则见 ADR-0005。

## Consequences

- Node 用户不需要 Rust 工具链即可安装预编译包；原生 CLI 可独立运行。
- Node API 不再承载上游 TypeScript 配置函数、JS 进度回调等能力；用户配置改用支持的 JSON/JSONC/TOML 文件。
- Rust 与 napi 层各自测试：Rust 使用 cargo 测试，编译产物通过 Node 集成测试验证。
