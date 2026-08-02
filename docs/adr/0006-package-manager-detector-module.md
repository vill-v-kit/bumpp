# package-manager 检测对齐上游名单，拆模块不拆 crate

`versionBump --install` 的包管理器检测（原 `bump.rs::detect_package_manager`）对齐上游 `package-manager-detector` 的默认行为并独立为 `crates/bumpp-core/src/pm.rs`；**不**为其新建独立 crate。同时修复两处现存 parity 偏差：`packageManager` 字段值不识别时误判为 npm（nub / aube / deno 项目中招）、同级冲突时字段优先于 lockfile（上游默认 lockfile 优先）。

## Decisions

- **对齐范围 = 名单 + 上爬**：agent 全表（npm / yarn / pnpm / bun / deno / nub / aube，取 `packageManager` 字段值 `<name>@<version>` 的 name 部分）与 lockfile 全表（`nub.lock`、`aube-lock.yaml`、`npm-shrinkwrap.json` 等，specific 优先）对齐上游 `AGENTS` / `LOCKS` 常量；策略顺序与迭代对齐上游默认（目录为外层，级内 lockfile → packageManager-field，逐级上爬至根）。
- **不对齐**：devEngines-field、install-metadata（均为上游**非默认**策略）、`COMMANDS`/`constructCommand` 命令映射——唯一消费方 `--install` 的命令对名单内 agent 恒为 `<agent> install`。无消费场景的对齐是为未来付现。
- **拆模块不拆 crate**：检测逻辑内聚为 `pm.rs` 单模块 + `tests/pm.rs` parity 矩阵（9 例）。不新增 workspace 成员与发版版本线（ADR-0003 后每个 crate 都是一处版本线）。
- **再拆触发条件**：出现第二消费方时，按 ADR-0007 同款流程拆 `crates/package-manager`——逻辑已内聚单模块，届时拆分是纯移动，成本极低。

## Considered Options

- **拆 `crates/package-manager` 独立 crate**：兑现独立测试与上游演进集中点，但唯一消费方下收益主要是账面的；代价是 workspace 成员 +1、发版版本线 +1，且与 ADR-0007"独立 crate 承载插件层"的拒绝理由同源：唯一消费方不提前拆包————拒绝，留触发条件。
- **全量 parity（含非默认策略与命令映射）**：超出消费点所需——拒绝，见 Decisions。
- **结构不动只补名单**：`bump.rs`（468 行）编排与检测继续混居，parity 矩阵无自然归属——拒绝。

## Consequences

- nub / aube / deno 项目的 `--install` 不再误判执行 `npm install`；`nub.lock` / `aube-lock.yaml` 项目不再报"无法检测"。
- 上游 `package-manager-detector` 名单演进时，同步点集中在 `pm.rs` 两个常量 + parity 矩阵。
- 第二消费方（如未来的 changelog Rust 化需要 PM 检测）出现时，凭触发条件拆 crate，无需重议本决策。
