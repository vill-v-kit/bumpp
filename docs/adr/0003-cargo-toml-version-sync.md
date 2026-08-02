# 发版支持 Cargo.toml 版本号同步

`vbumpp -r` 发版时除 `package.json` 外，同步更新显式列入 files 的 `Cargo.toml` 的 `[package].version`（toml_edit 保格式编辑，绝不触碰 `[dependencies]` 等其他表），并按 crate name 定向同步根 `Cargo.lock` 中对应 `[[package]]` 条目。解决自身 rust 包无法一键版本更新的问题。

## Decisions

- **toml_edit 编辑**：精确定位 `[package]` 表的 `version` 字段做保格式替换；引入 `toml_edit` 依赖（cargo 自家使用的 TOML 编辑库）。
- **workspace 版本继承场景**：先探测 `package.version` 形态——字面量字符串则更新；`version.workspace = true`（成员继承根 `[workspace.package]`）则**跳过该文件**（强写字面量会造成键冲突/破坏继承），改为更新根 `[workspace.package].version`（若存在）；两者皆无才按"version 缺失"走 FileSkipped。
- **basename 识别**：core 的 files 模块将 `cargo.toml`（小写比较）加入 manifest 识别名单，命中时走 TOML 通道而非文本替换。
- **显式纳入 files**：Cargo.toml 进入发版清单靠显式配置（本仓 `vbumpp.config.ts` 列明各 rust crate 的 Cargo.toml——当前为 `crates/bumpp-core`、`crates/version-files`、`napi/bumpp-core` 三处），不做 recursive 自动收集——避免误伤其他仓库中无关的 Cargo.toml。
- **Cargo.lock 同步**：优先以同一 toml_edit 机制按 name 定向更新 `[[package]]` 条目（确定性、无需跑 cargo）；条目缺失等同步失败场景**失败即报错**（发版一致性优先）。`cargo check --workspace` 作为兜底的备选刷新方式。
- **跳过规则复用**：与 manifest 相同——`version` 缺失或已是新版本时不改写（FileSkipped）。

## Considered Options

- **手写 span 替换（零新依赖）**：`[package]` 表边界、行内注释等边界需自行兜底——拒绝，toml_edit 维护性更强。
- **recursive 自动收集 `**/Cargo.toml`**：会波及所有 `vbumpp -r` 用户仓库中无关的 Cargo.toml——拒绝，显式配置可控。
- **lock 同步失败仅警告**：静默不一致比直接失败更糟——拒绝。

## Consequences

- 根 `Cargo.lock` 与两处 Cargo.toml 在发版提交中保持一致，CONTRIBUTING.md 的"版本线漂移点"随之消除（文档同步更新）。
- 未来新增 rust crate 时，需在 `vbumpp.config.ts` 的 files 中补充其 Cargo.toml。

## 落地补充（COL-23）

实现于 `crates/version-files` 的 `CargoTomlPlugin`（ADR-0004 静态链：JsManifest → CargoToml → Text），以下细节为实施时确定：

- **lock 发现**：自清单所在目录向上取首个 `Cargo.lock`（workspace 成员的 lock 在仓库根）；找不到则仅更新清单（库 crate 可不提交 lock，不视为漂移）。
- **失败语义**：lock 条目缺失、`[[package]]` 版本与清单当前版本漂移、lock 解析失败均立即报错（`UpdateError::Lock`），且清单不先行改写（全部计算通过后才写盘）。
- **workspace 继承**：成员清单（`version.workspace = true` 且本文件无 `[workspace.package]`）跳过——根清单自身作为显式文件项被处理；根 package 继承本文件 `[workspace.package]` 时更新该字段。lock 同步为成员扫描：无 `source` 且 `version == current` 的 `[[package]]` 条目。两条已知边界（单文件插件职责使然）：只列成员而不列根清单时不会代定位根文件——成员以 FileSkipped 事件可见地报出，由显式收集原则保证根清单在清单内；成员扫描以"零匹配报错"为漂移防线，个别成员条目缺失（其余成员仍命中）不可检测。
- **附带文件事件**：lock 同步产物以 `UpdateOutcome::UpdatedWith` 上抛，编排层紧随主文件补发 `FileUpdated`——`updated_files` 是 git 提交暂存的依据，Cargo.lock 由此进入同一次发版提交。
- **容错**：清单不可解析 → 立即报错（`UpdateError::Parse`，文案沿用相对路径）——显式列入发版清单的文件不可解析即漂移风险，从本 ADR"失败即报错"；与 JsManifest 通道对坏 JSON 的容错是有意的不对称（后者为对齐上游 bumpp v11 `jsonc.parse` 的 parity 要求，TOML 通道无上游约束）。lock 存在但不可解析同样报错（属同步失败）。
- **本仓配置**：`vbumpp.config.ts` 的 `bumpp.files` 列明 `crates/bumpp-core`、`crates/version-files`、`napi/bumpp-core` 三处 Cargo.toml（与默认 `package.json` / `package-lock.json` 合并去重）。
