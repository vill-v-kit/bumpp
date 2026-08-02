# 插件底座目录化（src/plugins/）

ADR-0009 把版本读取也归入插件链后，生态知识仍物理分散：`src/files/`（版本更新+读取）、`src/install/`（install 适配）、recursive 收集逻辑无独立归属（附在 files.rs 的聚合函数上）；install 侧还留着 ADR-0007 已消灭的那类硬编码注册表（生态顺序数组 + match 分发）。决策：立 `src/plugins/` 父目录为插件底座，下分 version / install / recursive 三能力子目录，各生态实现落同名文件；分发与触发语义全部保留，仅代码归属变化。

## Decisions

- **`src/plugins/` = 插件底座**：trait + 静态链 + `Ecosystem` + 编排在 `mod.rs`（update_files 分发、get_current_version 分发、run_installs 链走、recursive 模式表聚合）。
- **插件类型在根部，实现在能力子目录**：Rust coherence 规则——同一 trait 对同一类型的 impl 全 crate 仅一块，无法按能力拆进三个目录。`plugins/node.rs` / `cargo.rs` / `text.rs` 为插件本体（类型 + trait impl），每个方法一行委托至三能力子目录的生态同名文件（纯函数）。
- **三能力子目录**：`version/`（版本解析与更新：node JSONC 保格式、cargo toml_edit + lock 同步、text 兜底替换）、`install/`（ADR-0008 适配：node pm 检测 + `<pm> install`、cargo `cargo check --workspace`）、`recursive/`（manifest basenames → `**/` 模式）。Text 仅有 version 能力——`install/`、`recursive/` 无 `text.rs`，成员不齐是如实反映能力差异。
- **语义全部保留**：ADR-0007 的首命中分发与链序、ADR-0008 的条件触发 / 全 skip 不跑 / 仅 Text 更新回退 node、ADR-0009 的 `read_version` 与默认表聚合——本 ADR 只动结构不动行为。
- **recursive 目录为概念占位**：今日内容仅为每生态 basename 常量；赌注是未来实质逻辑（maven parent pom 遍历、node workspace 感知）入住。若长期停留为常量，退回为 trait 方法、目录撤销。

## Considered Options

- **install 并入 `files/` 每生态一文件**：版本解析文件混入进程执行（`exec::run`）、涨至 ~300 行——纯度换内聚，且 recursive 仍无归属；拒绝。
- **三 trait 三链**（version / install / recursive 各自 trait + 静态链）：manifest basename 知识在 version 与 recursive 间重复或交叉引用，注册点三处；拒绝。
- **维持 `files/` + `install/` 两目录**：生态知识物理劈半，加 maven 动 5 处（两目录文件 + `Ecosystem` 枚举 + install 侧顺序数组与 match 两张注册表）；拒绝。

## Consequences

- 加 maven = 4 个文件（`plugins/maven.rs` + 三子目录各一）+ 链上一行注册；编排层零改动。
- `tests/files/`、`tests/install/` 镜像迁移为 `tests/plugins/{version,install,recursive}/`（ADR-0007 的测试镜像约定延续）。
- ADR-0007/0008 的"每生态一文件"从目录级结论修订为能力子目录内的同名文件约定；本 ADR 与 ADR-0009 一次实施完成。
