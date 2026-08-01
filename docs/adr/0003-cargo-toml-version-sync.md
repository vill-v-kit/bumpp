# 发版支持 Cargo.toml 版本号同步

`vbumpp -r` 发版时除 `package.json` 外，同步更新显式列入 files 的 `Cargo.toml` 的 `[package].version`（toml_edit 保格式编辑，绝不触碰 `[dependencies]` 等其他表），并按 crate name 定向同步根 `Cargo.lock` 中对应 `[[package]]` 条目。解决自身 rust 包无法一键版本更新的问题。

## Decisions

- **toml_edit 编辑**：精确定位 `[package]` 表的 `version` 字段做保格式替换；引入 `toml_edit` 依赖（cargo 自家使用的 TOML 编辑库）。
- **workspace 版本继承场景**：先探测 `package.version` 形态——字面量字符串则更新；`version.workspace = true`（成员继承根 `[workspace.package]`）则**跳过该文件**（强写字面量会造成键冲突/破坏继承），改为更新根 `[workspace.package].version`（若存在）；两者皆无才按"version 缺失"走 FileSkipped。
- **basename 识别**：core 的 files 模块将 `cargo.toml`（小写比较）加入 manifest 识别名单，命中时走 TOML 通道而非文本替换。
- **显式纳入 files**：Cargo.toml 进入发版清单靠显式配置（本仓 `vbumpp.config.ts` 列明 `crates/bumpp-core/Cargo.toml` 与 `npm/bumpp-core/Cargo.toml`），不做 recursive 自动收集——避免误伤其他仓库中无关的 Cargo.toml。
- **Cargo.lock 同步**：优先以同一 toml_edit 机制按 name 定向更新 `[[package]]` 条目（确定性、无需跑 cargo）；条目缺失等同步失败场景**失败即报错**（发版一致性优先）。`cargo check --workspace` 作为兜底的备选刷新方式。
- **跳过规则复用**：与 manifest 相同——`version` 缺失或已是新版本时不改写（FileSkipped）。

## Considered Options

- **手写 span 替换（零新依赖）**：`[package]` 表边界、行内注释等边界需自行兜底——拒绝，toml_edit 维护性更强。
- **recursive 自动收集 `**/Cargo.toml`**：会波及所有 `vbumpp -r` 用户仓库中无关的 Cargo.toml——拒绝，显式配置可控。
- **lock 同步失败仅警告**：静默不一致比直接失败更糟——拒绝。

## Consequences

- 根 `Cargo.lock` 与两处 Cargo.toml 在发版提交中保持一致，CONTRIBUTING.md 的"版本线漂移点"随之消除（文档同步更新）。
- 未来新增 rust crate 时，需在 `vbumpp.config.ts` 的 files 中补充其 Cargo.toml。
