# 收集器迁移 ignore crate（暂缓记录——可能永久不做）

COL-61 用「glob 展开 + `git check-ignore` 子进程后过滤」修好了 `-r` 下钻 gitignored 目录的 bug。当时评估过 `ignore` crate（BurntSushi，ripgrep / fd / oxc 同款——oxc workspace 依赖 `ignore = "0.4.30"`）并**为修 bug 场景拒绝**（不换引擎）。但 `ignore` crate 相对现行方案有两个真实优势，本 ADR 把候选路径与重启信号记死，避免未来真遇大仓库性能问题时重新调研。

## Decisions

- **暂缓，可能永久不做**：收集器维持「`glob` crate 展开 + `check-ignore` 后过滤」（COL-61 落地形态）。本 ADR 不是工作承诺，是调研结论的存档。
- **候选终态（若重启）**：`ignore::WalkBuilder` + `require_git(false)` + `ignore::overrides::OverrideBuilder`（白名单 pattern 表）替换 glob walk + check-ignore 后过滤，一个 crate 同时承接「收集模式匹配」与「gitignore 裁决」两职责，可能顺势退役 `glob` 依赖。
- **重启评估的触发信号**（任一出现即回本 ADR 重评）：
  1. 大仓库 walk 性能投诉或 profile 数据——后过滤方案会**走完** `target/` 等被忽略整树再丢弃，WalkBuilder 是**剪枝**（根本不进入）；仓库越大差距越大。
  2. 非 git 目录也要尊重 `.gitignore` 的诉求——子进程方案在非 git 目录 fail-open 不过滤（COL-61 钉测的既定行为）；`require_git(false)` 可让 walker 无 `.git` 也生效。
  3. 想统一收集语义、摆脱 `glob` 依赖时。

## Considered Options

- **维持现状（采纳）**：`git check-ignore` 给的是 git 本体精确裁决（含 `.git/info/exclude`、`core.excludesFile`），零新依赖，fail-open 路径干净；COL-61 已落地并全绿。
- **迁 `ignore` crate（暂缓）**：优势真实（剪枝性能、非 git 目录可用、单一进程内语义），但 `normalize_files` 接受任意用户 pattern——绝对路径、`..` 逃逸、字面 basename、自定义 glob（`packages/*/package.json`）。gitignore 风格 glob 与 `glob` crate 语义在 `**`、字符类、转义上有微妙差异，迁移等于重写收集器并逐形态重验 parity——修 bug 场景下风险不对称，故当时拒绝；重启时这笔 parity 账仍要付，勿当免费午餐。

## Consequences

- 现状方案的两个已知代价（不影响当前使用，触发信号出现时才计价）：
  - 后过滤不剪枝——巨型 `target/` 树的 walk 成本随仓库规模线性增长；
  - 非 git 目录的 `.gitignore` 不生效（fail-open，COL-61 测试钉死）。
- 重启时的工作面预告：收集语义矩阵（默认清单 / `-r` / 配置 recursive 展开 / 用户字面与 glob pattern / 绝对路径 / `..` 逃逸）逐项对拍迁移；COL-61 的 glob/literal 分流边界（字面点名不过滤）可用 OverrideBuilder 与字面路径分两路喂 walker 保留。
- 参考锚点：COL-61（现行双层方案与裁定过程）、COL-63（构建链调研——oxc 发布矩阵参考地，oxc 的 `ignore` 用法同属可借鉴项）。
