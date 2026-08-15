# Rust Crate Structure

How this repo's Rust crates (`crates/`、`napi/` 下的纯 Rust 与绑定 crate) organize directories, modules, tests, and file size. 适用于 workspace 全部 Rust crate；存量不合规范的部分不回头专项改，随下次实质触碰该文件的改动顺带迁移。

## Module entry style

全仓统一 Rust 2018 无 `mod.rs` 风格（存量已全部迁移清零，见 Linear `villv-bump` 项目治理票）：

```text
src/foo.rs        # 模块入口：公共类型、re-export、模块声明
src/foo/
├── bar.rs
└── baz.rs
```

- `src/`、`tests/`（含 `main.rs` + 子模块树）任何位置不得新增 `mod.rs`；该规则同步写入根 `CONTRIBUTING.md`，PR 中出现即打回。
- 拆分既有模块（如单文件超限治理）时，新结构直接采用无 `mod.rs` 风格。

## Imports

全部引入在文件头一次完成：`use` 聚成一块，置于 `//!` 模块文档之后、任何 item 之前（将来若出现嵌套 `mod` 块，其引入放该块顶部）。item 体内不得出现全限定路径（`crate::` / `super::` / `std::` / 外部 crate 一律适用）——体内路径的首段必须解析到文件头引入的名字或本文件定义的 item。模块名引入（如 `use crate::display;`）后以 `display::path(...)` 形态调用合规。

例外仅两条：

- 同名歧义消解：两模块导出同名项、引入即冲突时，保留内联全路径；
- `macro_rules!` 宏体内的路径：宏展开在调用方解析，绝对路径才跨模块安全。

存量已全部迁移清零（约 180 处、三个 crate 的 src 全覆盖；tests 本已为零）——这是「存量不回头专项改」原则的显式例外，同 `mod.rs` 治理先例。不设 clippy 硬门禁（无现成 lint）；该规则同步写入根 `CONTRIBUTING.md`，PR 中出现新内联路径即打回。

## Top-level modules

`src/` 顶层平铺模块不设数量上限，但新增前先评估能否并入既有子域（`bump/`、`cli/`、`changelog/`、`plugins/`、`release/` 等）；确需新顶层时在 PR 描述里说明理由。设数字上限会逼出凑数分组，所以是行为约束不是数字约束。

## File size soft caps

| 范围 | 软上限 |
|------|--------|
| `src/` 单文件 | 500 行 |
| `tests/` 单文件 | 1,000 行 |

- 口径为 `wc -l` 全量（含空行、注释、内联测试）。
- 超限不需要事先批准，但 PR 里要给出理由（高内聚的解析表、一次性 fixture 等），由 review 把关。
- 不设 CI/clippy 硬门禁：行数是症状不是病因，硬门禁会催生为凑数而拆的坏拆分。
- 拆分沿内聚边界（子命令、平台、生命周期阶段），不为达标记拆。

## Tests mirror src

- `tests/` 目录结构与 `src/` 模块一一镜像；多文件测试域用 `main.rs` + 子模块组织（先例：`tests/bump/`、`tests/plugins/`、`tests/release/`）。
- 单元测试不放 `#[cfg(test)] mod tests` 内联，统一放 `tests/` 目录，消除测试位置二选一的歧义。
- fixtures 独立放 `tests/fixtures/<域>/`，与测试代码分离（先例：`tests/fixtures/changelog/`、`tests/fixtures/token/`）。
- 共享测试工具放 `tests/common.rs`（作为模块经 `#[path]` 或 `mod` 声明引入各测试二进制；它同时是一个零用例的空测试目标，属可接受成本）。

## Scope of past decisions

`crates/vbumpp-core` 的结构治理（`cli.rs` 拆分等）见 Linear `villv-bump` 项目对应 issue；本规范是该治理沉淀出的通用约定。
