# install 按生态适配与条件触发

`--install` 从"仅 node 生态、无条件执行"扩展为**按生态适配**：`crates/bumpp-core/src/install/` 目录每文件代表一个生态对 install 的适配——`node.rs`（上游 package-manager-detector parity + `<pm> install`，自 `pm.rs` 迁入）与 `cargo.rs`（`cargo check --workspace`）；`mod.rs` 负责生态识别与条件分发。maven / gradle 等未来生态以新文件加入，与 `src/files/` 的版本更新插件按同一"生态"维度对称（ADR-0007）。

## Decisions

- **生态条件触发**：本次 bump 实际 FileUpdated 了哪些生态的版本文件，就跑哪些生态的适配。生态归属经 files 插件链判定（`VersionFilePlugin::ecosystem()`）：CargoToml 通道 → cargo，JsManifest 通道 → node。
- **Text 兜底通道不贡献生态**：仅 text 文件被更新时零生态命中，**回退 node**——与上游 `--install`（无条件 node PM install）行为一致，JS-only 项目全场景保持 parity。
- **全 skip 不跑**：本次无任何文件更新时不执行任何适配。上游此时仍跑 install——版本未变时跑 install 本无意义，此为有意偏离。
- **cargo 适配 = `cargo check --workspace`**：ADR-0003 点名的兜底刷新方式——校验 Cargo.lock 定向同步结果、兜底刷新遗漏、验证 workspace 可编译。Cargo 为单一工具链，无 node 生态意义上的"检测哪个 PM"问题。
- **生态识别挂在 files 插件链上**（trait 增 `ecosystem()`），不在 install 侧重复 basename 规则——链是生态知识的单一事实源。

## Considered Options

- **node 恒跑（上游 parity）+ 其他生态条件触发**：混合模型，且 rust-only 仓库 `--install` 仍报"Could not detect package manager"——cargo 生态提案的核心痛点不修，拒绝。
- **维持现状（永远 node PM install）**：install/ 沦为纯组织搬迁，polyglot 仓库（如本仓）的 cargo 侧校验缺失，rust-only 报错保留——拒绝。
- **`cargo update --workspace` 作为 cargo 适配**：会升级全部依赖到最新兼容版，远超版本同步语义——拒绝。

## Consequences

- rust-only 仓库 `--install` 从报错变为正确执行 cargo 适配；node+cargo 混合仓（如本仓）一次 bump 跑两个适配（固定顺序 Node → Cargo）。
- 未来 maven / gradle：files/ 落版本插件、install/ 落适配文件，生态集合自动涵盖，无需改编排。
- 与上游的偏离（条件触发、全 skip 不跑）集中记录于本 ADR；JS-only 项目行为与上游一致。
