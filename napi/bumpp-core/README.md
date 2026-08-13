# @vill-v/bumpp-core

Rust 版本引擎（纯 Rust + [napi-rs](https://napi.rs/)），是 `@vill-v/bumpp` 的原生核心。向 Node.js 暴露的导出面刻意收窄为两个函数，其余能力收归 Rust 内部、经 CLI 单入口暴露：

- `cliRun(argv, provider?)` — CLI 单入口：argv 全权交 Rust 解析执行，返回退出码由调用壳回写 `process.exitCode`；`provider` 为平台变体身份，由 `@vill-v/bumpp-github` 等变体 bin 注入
- `bumpVersion(overrides?, provider?, cwd?)` — 完整 bump 编排：统一配置解析 → 交互选版本 → changelog → 文件更新 / scripts 钩子 / git commit/tag/push；`provider` 传 `'github' | 'gitlab' | 'gitee' | 'gitcode'` 时末段追加创建平台 Release

bump 成功但平台 Release 失败（网络 / 密钥过期等）后的独立补发，由 CLI `vbumpp release` 子命令承接。

## 支持平台

以预编译 Platform package 分发（主包 + optionalDependencies；平台包目录由 `pnpm create:npm-dirs` 从 `napi.targets` 生成，ADR-0029）：

| 平台包 | target |
| --- | --- |
| `@vill-v/bumpp-core-darwin-arm64` | aarch64-apple-darwin |
| `@vill-v/bumpp-core-linux-x64-gnu` | x86_64-unknown-linux-gnu |
| `@vill-v/bumpp-core-linux-arm64-gnu` | aarch64-unknown-linux-gnu |
| `@vill-v/bumpp-core-linux-x64-musl` | x86_64-unknown-linux-musl |
| `@vill-v/bumpp-core-linux-arm64-musl` | aarch64-unknown-linux-musl |
| `@vill-v/bumpp-core-win32-x64-msvc` | x86_64-pc-windows-msvc |
| `@vill-v/bumpp-core-win32-arm64-msvc` | aarch64-pc-windows-msvc |

运行时按当前平台加载对应 `.node`；无匹配平台包时 loader 抛出 `Cannot find native binding`（cause 链附各候选平台包的加载失败明细）。

## 配置

配置文件为两级多格式：项目级 `.vbumpprc.{json,jsonc,toml}`（`.json` / `.jsonc` 同走 JSONC 解析，支持注释与尾逗号），全局通用配置放 `~/.vbumpp/config.{json,jsonc,toml}`；合并优先级：overrides > 项目 > 全局 > 内建默认。仅支持纯数据格式，不执行 TS/JS 配置文件。

## License

[MIT](../../LICENSE)
