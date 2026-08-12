# core 发布包不内置 .node：optionalDependencies 平台包是唯一分发通道

`napi artifacts`（ADR-0029 注入步骤消费的上游命令）刻意把每个 target 的 `.node` 同时写进平台包目录**与 core 包根**——napi-rs 的兜底设计：`--no-optional` 安装也能从包根 fallback 加载，且无开关可关。`napi/bumpp-core/package.json` 的 `files` 字段含模板遗产 `"*.node"`：旧手工 cp 注入流里 publish-npm 是 fresh checkout、包根从无 .node，该条目从不命中；ADR-0029 切换注入流后首次命中，6.1.0 core tarball 膨胀为 12 文件 / 36.4MB（7 架构全量内置），而 6.0.0 为 5 文件 / 8.4KB。本 ADR 决策发布态 core 不携带任何 .node，回到 6.0.0 已验证语义。

## Decisions

- `files` 移除 `"*.node"`：发布态 core = index.js + index.d.ts（+ README/LICENSE 自动入包），二进制经 optionalDependencies 平台包单通道分发。包根 .node fallback 仅存在于本地开发磁盘（`napi build` 产出）——`files` 只约束发布内容，不影响本地加载路径。
- 放弃上游「包根全架构兜底」设计：`--no-optional` / `omit=optional` 安装不再被内置二进制兜住，撞 loader 的缺包报错——与 6.0.0 已发布语义一致；esbuild / swc / biome 的生态惯例同为 optionalDependencies-only。36MB 体积换边缘安装模式兼容，不值。
- 修在清单而非流水线：不引入「发布前删包根 .node」的 CI 清理步骤——本地 pack 与 CI pack 行为必须一致；`napi artifacts` 的包根写入无开关，与其每次对冲不如不发布。
- publish-npm 预验证加产物断言：pack 之后断言 core tarball 不含 `.node`（glob 精确匹配 `vill-v-bumpp-core-[0-9]*.tgz`；平台包应含 .node，不在此列）。publint 查结构不查体积；「files 字段一直正确、上游行为变化」的组合回归只有断在产物上才可靠。断言双向验证过：对旧 6.1.0 tarball 正确拦停，对修复后 pack 放行。
- 已上架的 6.1.0 不处置：功能正常仅体积膨胀；npm 不允许改已发布版本，deprecate 会误伤正在正常使用的用户。修正随下一版本自然带出。

## Consequences

- 每次发版 `napi artifacts` 仍会往包根写 7 份 .node（无开关），CI 工作区残留属预期；产物断言保证它们不进 tarball。
- loader 的本地 fallback 代码路径保留不变：fresh clone 本地开发与 `NAPI_RS_NATIVE_LIBRARY_PATH` 覆盖不受影响。
- core tarball 体积与平台矩阵规模脱钩：矩阵再扩张只新增一个平台包，core 恒为 ~8KB。
- 参考锚点：ADR-0029（注入流与平台包生成）、ADR-0021（publish-npm 预验证段与 OIDC 上架）。
