# version-files 插件底座 crate

版本文件的解析与更新从 `bumpp-core` 的 `files` 模块拆出，独立为 `crates/version-files`：内置插件底座 trait 与静态插件链（JS 生态 JSON manifest、Cargo TOML、文本兜底），为将来 maven / gradle 等生态提供统一扩展点。`bumpp-core` 依赖它，行为完全不变。

## Decisions

- **单一新 crate `crates/version-files`**：插件底座 trait + 内置插件同 crate（不为当前唯一的生态插件提前抽 base crate，避免投机性分层）。
- **插件 trait + 内置静态插件链**：

  ```rust
  pub trait VersionFilePlugin {
    fn matches(&self, rel_path: &Path) -> bool;
    fn update(&self, path: &Path, current: &str, new: &str) -> Result<UpdateOutcome>;
  }
  ```

  内置有序链：`JsManifestPlugin`（8 种 basename + package-lock `packages[""].version` 规则）→ `CargoTomlPlugin`（COL-20 落地）→ `TextPlugin`（上游 `(\b|v){version}\b` 替换，兜底）。按 `matches` 顺序分发，命中即走对应通道。
- **静态分发**：开放注册 API 留待真实外部插件出现时再引（当前无注册方，不做运行时 registry）。
- **纯迁移先行**：JS 生态 JSON 逻辑（JSONC 容错解析、isManifest、span 替换保格式、跳过规则）原样搬入，行为零变化由现有双层测试兜底。

## Considered Options

- **底座与 JS 生态拆两个 crate**：当前只有一个生态插件，多一层间接属过度设计——拒绝，未来出现外部插件再抽。
- **运行时 registry**：无外部注册方，投机性设计——拒绝。

## Consequences

- `bumpp-core` 的 `files` 模块收缩为对 `version-files` 的编排调用（文件存在性、事件产出、路径归一）。
- maven（pom.xml）/ gradle（build.gradle[.kts]）等生态未来以实现同 trait 的插件加入链尾（TextPlugin 之前），编排层零改动。
