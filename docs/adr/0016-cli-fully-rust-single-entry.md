# CLI 全权归 Rust并提供原生二进制

argv 语法、命令分派、help、错误与退出码唯一归属 Rust。npm bin 与原生 `vbumpp` 二进制共享 `vbumpp_core::cli::run_from_argv`，都是无业务逻辑的薄壳。

## Decisions

- **入口**：napi 暴露 `cliRun(argv, provider?) -> Promise<number>`，由 Node 壳回写 `process.exitCode`；原生 crate `crates/vbumpp` 只收集 argv、调用同一 Rust 入口并退出。原生 crate 仅依赖 `vbumpp-core`，零 napi 依赖。
- **解析器**：CLI 模块使用手写解析器，不引入 clap。默认 bump 命令支持文件位置参数、`-o/--output`、`-r/--recursive`；另有 `token` 与 `release` 子命令及全局 help/version。
- **Provider**：`--provider <github|gitlab|gitee|gitcode>` 对 bump 与 release 生效。优先级为 argv flag 高于平台变体 npm 包注入；二者都没有时，bump 不创建平台 Release。Provider 不从配置或 git remote 推断。
- **Release 重试**：`vbumpp release <version>` 从 `--output` 指定的 changelog 文件（默认 `CHANGELOG.md`）提取目标版本节，并复用现有 token、仓库解析和 provider 创建机制。输入版本接受 `5.1.0` 与 `v5.1.0`。
- **Release 前置校验**：目标 changelog 版本节必须存在，本地 `v<version>` tag 必须存在；任一失败均退出 1。远端 Release 已存在时保留平台错误，不自动更新。
- **Token 命令**：`token set/list/remove` 直接调用 Rust token 模块，不保留独立 napi token 函数。
- **配置覆盖**：默认 bump 命令仅在用户提供 files 时注入 `files` override，避免空数组覆盖配置文件；recursive 与 changelog output 由 argv 形成 overrides。
- **npm 面**：移除 cac、consola、JS CLI 工厂和 `./cli` 子路径。平台变体通过 `cliRun(argv, provider)` 注入默认 provider。

## Consequences

- npm bin 与原生二进制的语法和行为不会漂移；`--version` 使用 workspace 继承的 crate 版本。
- 独立平台 Release 通过 `vbumpp release <version> --provider ...` 重试，不需要公开低层 `createXRelease` API。
- CLI 的解析、help、错误与退出码测试全部位于 Rust；Node 侧只需入口冒烟测试。
