# napi/ 目录按受众判别收纳内部机制包

`napi/` 的定位从"Rust↔Node 的 napi 绑定包，不发布到 npm"扩展为**内部机制包**目录：虽然发布到 npm、但设计上就不是给用户直接使用的包也放这里（首批迁入：`npm/bumpp-core` → `napi/bumpp-core`，以及 5 个平台二进制包）。目录分流规则由"npm/ 高于 napi/（要发版 → npm/）"翻转为**受众优先（无论是否发布）**。

## Decisions

- **受众判别**：分流问句是"用户会直接 npm install 它吗？"——不会 → `napi/`（内部机制包）；会 → `npm/`（面向用户的包）。判别依据是包的设计受众，与是否发布、产物形态均无关。
- **内部机制包**作为领域术语进入 CONTEXT.md：`@vill-v/bumpp-core`（napi 绑定本体）与 5 个平台二进制包（optionalDependencies 分发机制）是首批成员。它们发布到 npm 仅因 npm 不支持 workspace 协议引用与按平台分发二进制的机制需要——用户没有直接安装它们的场景。
- **原子搬迁**：约定改写与 6 包物理搬迁（`npm/` → `napi/`）同变更落地，引用（根 `Cargo.toml` members/exclude、ci.yml 产物路径、`repository.directory`、`vbumpp.config.ts`、文档）一次接齐——不接受"规则先行、代码后动"的脱节中间态。
- **目录名保留 `napi/`**：机制包在技术上就是 napi 绑定及其分发产物，名目沿用；不为纯 JS 内部包的假想情形提前更名。

## Considered Options

- **发布判别（要发版 → npm/）**：即旧规则。它让 `@vill-v/bumpp-core` 这种用户从不直接安装的绑定包与 `@vill-v/bumpp` 这种面向用户的 CLI 混居 `npm/`，目录名与受众错位——拒绝。
- **产物形态判别（含 .node 产物 → napi/）**：当前包集合上结果与受众判别相同，但依据的是产物而非意图；未来出现纯 JS 内部包（如共享脚本包）会被误判到面向用户区——拒绝，但承认其为受众判别的近似。
- **规则先行、搬迁另立票**：中间态约定与现实脱节，正是本次要修的问题（旧规则文档说一套、示例包做另一套）——拒绝，同变更原子落地。

## Consequences

- `npm/` 只剩面向用户的包（`@vill-v/bumpp` + 4 个 registry provider 包）；`napi/` 收全部内部机制包。新人按目录即可推断包的受众。
- 未来新增包先回答受众问句再落目录；纯 JS 内部包出现时，`napi/` 名目是否合身再议（届时考虑更名 internal/ 等）。
- 旧规则"npm/ 高于 napi/"作废；`npm/bumpp-core` 从规则示例变为迁移完成的归位案例。
