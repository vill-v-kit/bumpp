# `-r` 整树收集锁步 private 包

`-r` 整树收集对命中的清单无差别锁步版本号，包括 `"private": true` 的包（根 workspace 包、website 文档站）。本 ADR 明确：private 包随整树锁步是既定行为，不是收集器无差别 walk 的副产品；不引入 private 排除分支。

## Decisions

- `-r` 整树收集不按 `"private": true` 过滤，private 包与发版包一并锁步。该行为与上游 bumpp parity，COL-66 裁定确认保留。
- 「private」在本仓库的语义只是「不上架」（发布侧 `pnpm publish -r` 自动跳过，ADR-0021），不是「不参与版本管理」。版本号唯一维护点是根 workspace 版本，整树清单与之锁步保持单调；private 包不需要也不引入独立的版本语义。
- 排除方案被否决的理由：根 workspace 包自身就是 `"private": true` 且承载唯一版本号，任何 private 过滤都必须先为「当前包本身」造豁免特例；换来的收益只是个别 private 包的版本号停在原地，不抵规则复杂度与 parity 偏离。lerna / changesets 跳过 private 的惯例适用于「多包各自发布、各自版本」的模型，本仓库是单一版本锁步模型，惯例不迁移。
- private 包的版本字段保留并随锁步推进；不删除字段、不另立 `0.0.x` 自治语义（COL-66 的 website 版本字段问题随之消解）。
- 收集器的过滤层仍只有内置目录排除（IGNORED_DIRS）与 gitignore 感知（构建残留排除）两层，不新增 private 过滤；显式点名文件不过滤（用户意图优先）的边界不变。

## Consequences

- website 等 private 清单的版本号永远等于产品版本号；阅读 private 包版本号时把它当作锁步镜像，而不是该包自身的发布节奏。
- 与上游 bumpp 行为一致，迁移指南无需新增行为变化条目；用户文档以注记形式说明该语义。
- 重开评估的触发条件是 private 包锁步产生具体痛点（而非假想噪音）；届时排除方案须先回答根清单豁免与显式点名边界的规则设计。
- 参考锚点：ADR-0007（插件底座与 recursive 整树收集的归属）、ADR-0021（发布侧 private 自动跳过）、ADR-0022（收集器过滤层现状）。
