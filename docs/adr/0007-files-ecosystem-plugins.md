# 生态插件底座

版本清单识别、版本读取与更新、install 适配和 recursive 收集统一归属 `crates/vbumpp-core/src/plugins/`。插件按生态组织，通过单一静态链提供各能力；Text 是仅承担版本文本替换的兜底通道。

## Decisions

- `plugins.rs` 持有 `VersionFilePlugin` trait、`Ecosystem`、静态插件链和编排；插件类型位于 `plugins/javascript.rs`、`plugins/cargo.rs`、`plugins/text.rs`，具体能力委托到 `version/`、`install/`、`recursive/` 子目录的同名实现。
- 内置链顺序为 JavaScript → Cargo → Text，按 `matches` 首命中分发。当前没有外部注册方，不提供运行时 registry；出现真实复用方时再评估独立 crate 或注册 API。
- trait 同时提供 `matches`、`ecosystem`、`manifest_basenames`、`read_version`、`update` 与 `install`，避免清单 basename、生态归属和 install 分发形成多份注册表。
- JavaScript 插件识别其结构化清单并保格式更新版本；Cargo 插件处理 `Cargo.toml` 与 Cargo.lock（见 ADR-0003）；Text 插件使用版本文本替换规则兜底，不贡献生态、默认清单或版本来源。
- 默认 files 是链上 `manifest_basenames` 的根级并集；recursive 模式是同一集合的 `**/` 并集。不存在的文件由 glob 展开自然消失，新增生态只需在插件链声明清单。
- 当前版本从规范化后的文件列表依次经插件读取，首个合法 semver 为准。JavaScript 和 Cargo 清单可作为版本来源，Text 不提供版本来源。
- 用户配置的 `files` 保留，用于限定范围或显式纳入 README、`.env` 等 Text 文件；显式值替换默认清单。
- `--install` 只在本次存在实际更新文件时触发。按更新文件命中的生态、依链序去重执行：JavaScript 运行检测到的 `<pm> install`，Cargo 运行 `cargo check --workspace`。仅 Text 更新时回退 JavaScript；全 skip 时不执行 install。
- 测试目录镜像插件能力结构，分别覆盖链分发、各生态版本行为、install 与 recursive/default 模式。

## Consequences

- 新增生态需要定义插件类型并实现适用的 version、install、recursive 能力，再在静态链登记一次；编排层无需维护平行映射。
- 纯 Cargo 项目默认可读取和更新根 `Cargo.toml`，混合项目可在一次 bump 中按实际更新生态执行多个 install 适配。
- 默认清单和条件 install 有意超出上游 node-only 行为；Text 兜底保留用户自定义版本文件能力。
