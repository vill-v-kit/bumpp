# 收集器迁移 ignore crate：暂缓

## Decisions

- 收集器维持现行「`glob` crate 展开 + `git check-ignore` 后过滤」方案。它已经修复递归模式下进入 gitignored 目录的问题；本 ADR 是调研结论，不构成迁移承诺。
- 只有出现大仓库 walk 的 profile/性能投诉、非 git 目录也必须尊重 `.gitignore`，或明确要统一收集语义并移除 `glob` 依赖时，才重启评估。
- 若重启，候选是 `ignore::WalkBuilder`（`require_git(false)`）配合 `OverrideBuilder`；必须逐项对拍默认清单、递归展开、用户字面量/glob、绝对路径和 `..` 逃逸等现有语义，不能把迁移视为免费替换。

## Consequences

- 现状的已知代价被接受：后过滤会走过被忽略的整树，非 git 目录中的 `.gitignore` fail-open。
- 在触发条件出现前不增加 `ignore` 依赖，也不改变 COL-61 钉死的字面路径与 glob 分流行为。
- 参考锚点：COL-61（现行实现与裁定）和 COL-63（构建链调研参考）。
