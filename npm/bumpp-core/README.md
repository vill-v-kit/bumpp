# @vill-v/bumpp-core

自研 Rust 版本引擎（纯 Rust + [napi-rs](https://napi.rs/)），语义全量对齐上游 [bumpp](https://github.com/antfu/bumpp) v11，向 Node.js 暴露四个兼容 API：

- `versionBump` — 文件版本更新 + npm scripts + git commit/tag/push（进度由 Rust 内置打印）
- `versionBumpInfo` — 候选版本计算 + Rust 渲染的交互 prompt
- `loadBumpConfig` — 配置加载合并（仅支持 JSON 配置文件）

## 支持平台

以预编译 Platform package 分发（主包 + optionalDependencies）：

| 平台包 | target |
| --- | --- |
| `@vill-v/bumpp-core-darwin-arm64` | aarch64-apple-darwin |
| `@vill-v/bumpp-core-linux-x64-gnu` | x86_64-unknown-linux-gnu |
| `@vill-v/bumpp-core-linux-arm64-gnu` | aarch64-unknown-linux-gnu |
| `@vill-v/bumpp-core-win32-x64-msvc` | x86_64-pc-windows-msvc |
| `@vill-v/bumpp-core-win32-arm64-msvc` | aarch64-pc-windows-msvc |

OpenHarmony（ohos 三元组）为 best-effort 目标。运行时按当前平台加载对应 `.node`；无匹配平台包时报错会列出全部已支持平台。

## 与上游 bumpp 的差异

- **配置仅支持 JSON**（`bump.config.json`）；检测到 TS/JS 配置文件会报错并给出迁移指引
- `customVersion` 函数选项移除（JSON 无法承载函数），prompt 中不出现 "from config"
- 其余语义全量对齐上游 v11：preid 规则（沿用当前预发行标识，默认 `'beta'`）、预发行号 `0→1` 修正、`next` / `conventional`（约定式提交推断）、`custom` / `none`

## License

[MIT](../../LICENSE)
