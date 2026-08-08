# Cargo.toml 与 Cargo.lock 版本同步

Cargo 生态清单由插件底座结构化处理。更新 `Cargo.toml` 时只修改 `[package].version` 或 `[workspace.package].version`，并在适用时按 crate name 定向同步最近的上级 `Cargo.lock`。

## Decisions

- 使用 `toml_edit` 保格式编辑版本字段，不触碰依赖表等其他内容。
- `[package].version` 为字符串时直接更新；成员使用 `version.workspace = true` 时不写入成员字面量，由根清单的 `[workspace.package].version` 统一更新。版本字段缺失或已是目标版本时跳过。
- 从清单目录向上查找首个 `Cargo.lock`。找不到 lock 时仅更新清单；找到时，按 crate name 和当前版本匹配无 `source` 的 workspace 条目并同步。
- lock 解析失败、目标条目缺失或当前版本漂移均报错；所有检查成功后才写盘，避免清单先行改写。
- 同步产生的 `Cargo.lock` 作为附带更新文件紧随主清单进入 `updated_files`，与清单在同一次 git 提交中暂存。
- `Cargo.toml` 是 Cargo 插件声明的清单 basename。默认模式包含根级 `Cargo.toml`，recursive 模式包含 `**/Cargo.toml`；`matches` 对 basename 大小写不敏感。
- 本仓根 `[workspace.package].version` 是 Rust 版本字面量的唯一维护点，成员均通过 `version.workspace = true` 继承；项目配置无需显式列出 Cargo 清单。

## Consequences

- Bump 可更新纯 Cargo 项目和混合项目，并保持 workspace 清单与 lock 一致。
- 显式配置 `files` 会替换默认清单；需要自定义范围时必须同时纳入所有期望更新的根清单。
- 新增 workspace crate 继续继承根版本即可，无需新增版本字面量或项目配置项。
