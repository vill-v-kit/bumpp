# version-files 合并回 bumpp-core（生态插件文件夹）

> 本决策取代 [ADR-0004](./0004-version-files-plugin-base.md) 的 crate 边界部分；ADR-0004 的插件 trait、静态链顺序与"纯迁移、行为零变化"结论仍然有效并在合并后原样保留。

`crates/version-files`（COL-22 自 `bumpp-core` 拆出）重新合并回 `crates/bumpp-core`：生态插件组织保留为 `src/files/` 目录——每个文件代表一个生态的版本更新逻辑（`js_manifest.rs` / `cargo_toml.rs` / `text.rs`），`files.rs` 为模块根（编排 + `VersionFilePlugin` trait + 静态链 + 错误类型）。

## Decisions

- **撤销独立 crate**：trait、静态链、三生态插件与 33 例测试原样迁回（移动非重写）；`UpdateError` / `FilesError` 双枚举合并为单一 `FilesError`（Io / Parse / Lock），`From` 映射层删除。
- **生态组织保留为文件夹**：`src/files/` 每生态一文件；生态扩展点仍是 trait + 静态链（maven / gradle 未来落本目录新文件，Text 之前），扩展机制与 crate 边界无关。
- **版本线收缩**：`vbumpp.config.ts` 的发版 files 去掉 `crates/version-files/Cargo.toml`（3 → 2）；`bumpp-core` 收编 `toml_edit` 依赖。
- **测试镜像结构**：`tests/files/` 目录单 target——`main.rs` 为编排矩阵，生态矩阵为子模块，与 `src/files/` 一一对应。

## Considered Options

- **保留独立 crate**：拆分时预期的是插件生态的独立演进与复用；落地后消费方始终只有 `bumpp-core` 一个，crate 边界的持续成本（双错误枚举、发版版本线 +1、lock/元数据噪音）换不到兑现的收益——反转。
- **合并并去 trait 化（静态函数分发）**：生态扩展点消失，与 maven/gradle 的既定方向矛盾——拒绝。

## Consequences

- 生态插件的开发闭环（改插件 → 跑测试）不再跨 crate；`cargo test -p bumpp-core` 单点覆盖全部文件更新逻辑。
- ADR-0004 的"纯迁移、行为零变化"资产（33 例 parity 矩阵）完整保留于 `tests/files/`。
- 未来若出现 `bumpp-core` 之外的真实复用方，再按 ADR-0004 的原论证拆出——彼时拆分理由才成立。
