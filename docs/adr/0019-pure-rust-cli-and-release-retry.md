# 纯 Rust CLI + `vbumpp release` 重试子命令 + napi 面三收缩

两件事同落：① 原生 CLI 二进制从规划（ADR-0016 预留的"第二个薄壳前端"）变为现实——新 crate `crates/bumpp-cli`，纯 Rust、零 napi 依赖，功能与 npm bin 对齐；② CLI 新增 `vbumpp release <version>` 子命令——从 changelog 文件提取指定版本节创建平台 release，承接 bump 流程末段 release 因网络失败 / 密钥过期后的独立重试。原生二进制没有平台变体包的注入渠道，argv 语法因此新增 `--provider` flag。独立 release 改由 CLI 承接后，napi 面四个 `createXRelease` 导出（ADR-0014 的 per-provider parity 遗物）失去存在理由，随 v6 破坏性窗口移除。

## Decisions

- **新 crate `crates/bumpp-cli`**：bin crate，唯一依赖 `bumpp-core`（workspace 继承），零 napi 依赖；二进制名 `vbumpp`（与 npm bin 同名同语义——同一工具、同一 argv 语法，仅分发渠道不同）；`main` 为薄壳：收集 argv → `run_from_argv` → 以返回码 `process::exit`。`publish = false`，版本继承根 workspace；本次不做分发（本地 `cargo build --release` / `cargo install --path`）。
- **`--provider` flag**：argv 语法新增 `--provider <github|gitlab|gitee|gitcode>`，bump 默认命令与 release 子命令共用。优先级：argv flag > 平台变体包注入（`cliRun(argv, provider)` 位置参数）；两者皆无时维持现状（bump 后不接 release）。不做 git remote 推断（RepoConfig 只识别 canonical 域名，gitee/gitcode/自建实例必失败），不进配置文件。
- **`vbumpp release <version>` 子命令**：version 为位置参数，`5.1.0` / `v5.1.0` 均接受（内部归一化）；`--provider` 必填，缺失报错并列出可选值。body 从 `--output` 指定的 changelog 文件（默认 `CHANGELOG.md`，与 bump 同一文件概念）提取该版本节——节范围与 bump 通路的 release body 同形（`## <version>` 头 → compare 链接 → 各类型节，止于下一个 `## `）。
- **两道前置硬校验**（失败均退出码 1）：changelog 中找不到该版本节（防静默发空 body）；本地 `git rev-parse v<version>` 无此 tag（防平台 API 在默认分支 HEAD 静默建 tag——github-like 请求体的 `tag_name` 在 tag 缺失时的平台行为）。远端 release 已存在 → 平台错误原样透传，纯创建语义，不做更新。
- **复用现有机制**：token 解析链、`gitlab.host` 自建实例解析全走 `create_release` 现有通路；changelog 版本节提取为 core 新能力（`changelog` 模块），非 CLI 私货。
- **napi 面三收缩**：删除四个 `createXRelease` 导出 + 四个平台变体包的 re-export + `CreateReleaseOptions` 系类型 + smoke.mjs 断言。最终 napi 面只剩 `bumpVersion(options, provider?)` 与 `cliRun(argv, provider?)`。`migration-v6.md` 增补条目：独立 release 迁移至 `vbumpp release <version>`。

## Considered Options

- **bumpp-core 内加 `[[bin]]`**：少一个 crate，但库与二进制耦合（cargo test 亦编 bin），稀释 `crates/` 纯库 crate 的目录约定——拒绝。
- **provider 走配置文件字段或 git remote 推断**：配置字段引入与变体包注入的三方优先级问题；remote 推断对 gitee/gitcode/自建实例不可靠——均拒绝，argv flag 显式无状态。
- **远端已存在时自动转为更新 release body**：对"请求成功但响应丢失"的重试更顺，但引入覆盖远端内容的写语义，超出"重试"本意——拒绝，保持纯创建。
- **四导出合并为单个 `createRelease(options, provider)`**：面更小且与 `bumpVersion` 同形，但独立 release 场景已由 CLI 承接，保留价值不抵维护成本——拒绝，直接移除。
- **分发（crates.io / GitHub Release 预编译）**：crates.io 发布不可撤销、预编译需 CI 矩阵基础设施——本次不做，后续单独决策。

## Consequences

- `crates/bumpp-cli` 加入根 `Cargo.toml` members；napi 相关的全部依赖仍隔离在 `napi/bumpp-core` 一 crate，`cargo build -p bumpp-cli` 全依赖树无 napi。
- 平台变体包（`@vill-v/bumpp-{github,gitlab,gitee,gitcode}`）公开 API 收缩为仅 `bumpVersion`；外部编程式用户若调用过 `createXRelease`（仓库内零调用、README 无文档，概率低）需迁移至 CLI。
- `--provider` 对 npm bin 同样可用（共享解析器），变体包用户可临时跨平台发 release——视为特性而非漏洞（显式 argv 优先于注入身份）。
- 新增 provider 的落点在 ADR-0018 清单上加一项：`--provider` flag 的可选值列表（`Provider::parse` 已注册即自动生效）。
