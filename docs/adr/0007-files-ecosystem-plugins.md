# files 生态插件文件夹

版本文件的解析与更新按**生态**组织于 `crates/bumpp-core/src/files/`：每个文件代表一个生态的版本更新逻辑（`js_manifest.rs` / `cargo_toml.rs` / `text.rs`），`files.rs` 为模块根（编排 + `VersionFilePlugin` trait + 静态插件链 + 错误类型）。maven / gradle 等未来生态以同 trait 插件加入本目录（Text 之前）。

## Decisions

- **插件 trait + 内置静态插件链**：

  ```rust
  pub(crate) trait VersionFilePlugin: Sync {
    fn matches(&self, rel_path: &Path) -> bool;
    fn ecosystem(&self) -> Option<Ecosystem>;
    fn update(&self, path: &Path, rel_path: &Path, current: &str, new: &str) -> Result<UpdateOutcome, FilesError>;
  }
  ```

  内置有序链：`JsManifestPlugin`（8 种 basename + package-lock `packages[""].version` 规则）→ `CargoTomlPlugin`（ADR-0003）→ `TextPlugin`（上游 `(\b|v){version}\b` 替换，兜底，不归属任何生态）。按 `matches` 顺序分发，命中即走对应通道。
- **静态分发**：开放注册 API 留待真实外部插件出现时再引（当前无注册方，不做运行时 registry）。
- **生态归属挂在链上**：`ecosystem()` 使 install 侧（ADR-0008）无需重复 basename 规则——链是生态知识的单一事实源。
- **清单收集知识同在链上**：`manifest_basenames()` 聚合为 recursive 整树收集模式表（`recursive_manifest_globs()`），经 napi 导出供 CLI 的 `-r` 使用（ADR-0003 opt-in 语义）；新增生态时收集与更新一并纳入，JS 侧零改动。
- **错误模型单一**：`FilesError`（Io / Parse / Lock）贯通编排与插件，无跨边界映射层。

## Considered Options

- **独立 crate 承载插件层**：消费方只有 `bumpp-core` 一个时，crate 边界只有持续成本（双错误枚举与 From 映射、发版版本线、lock/元数据噪音）——拒绝；出现真实复用方时再议。
- **运行时 registry**：无外部注册方，投机性设计——拒绝。

## Consequences

- 新增生态 = `src/files/` 落一个实现同 trait 的文件 + 链上登记；编排层零改动。
- `tests/files/` 目录单 target 镜像 `src/files/` 结构：编排矩阵 + 每生态一行为矩阵。
