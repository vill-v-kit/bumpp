# 默认清单与版本来源生态化

生态插件架构（ADR-0007/0008）落地后残留两处上游 node-only：`files` 为空时的默认清单仍是上游 6 个 node 文件，`get_current_version` 只从 `.json` 读版本——纯 cargo（及未来 maven/gradle）项目裸跑必然 `UnableToDetermineVersion`。决策：默认清单与版本来源均改为插件链聚合的生态知识；`files` 用户定制与 text 兜底经本轮复议保留。

## Decisions

- **默认清单 = 链上 manifest basenames 的根级并集**：`files` 为空时不再用上游 `DEFAULT_FILES`（6 个 node 文件），改取插件链 `manifest_basenames()` 聚合（即 recursive 模式表去掉 `**/` 前缀）。不存在的文件由 glob 展开自然消失——无运行时生态探测；纯 cargo 项目裸跑即命中 `Cargo.toml`，混合项目各生态根清单皆中。新增生态落插件即自动纳入，与 recursive 模式表同一事实源。
- **npm/bump CLI 撤除硬编码默认 files**：`config.ts` 向 `loadBumpConfig` 传的 `files: ['package.json', 'package-lock.json']` overrides 会压住新默认表，改为不传，让 core 默认生效。
- **版本读取落插件底座**：`VersionFilePlugin` 增 `read_version(path) -> Option<String>`——js 通道读 `version` 字段（`info.rs` 私有 `read_version` 迁入，与更新逻辑共用解析路径），cargo 通道读 `[package].version` / `[workspace.package].version` 字面量，text 通道恒 None（无版本概念）。`get_current_version` 对 normalize 后的文件逐个经链分发读取，首个合法 semver 即当前版本；消掉 `.json` 过滤与硬编码探测表（`["package.json", "deno.json", "deno.jsonc"]` → 链上聚合）。版本读取与写入知识同一插件文件。
- **`files` 用户定制与 text 兜底保留**：清单列举职责被默认表 / `-r` 吸收后，`files` 角色收窄为 text 通道入口（pkl / .env / README 等非清单文件的版本替换）与范围收敛（只要子树、排除某清单）。text 兜底是上游 v11 `updateTextFile` 的逐字移植，仅被用户显式列名触发；maven `revision` 等结构化格式的精确更新由未来生态插件解决（链序保证插件先于 Text），不为临时窗口砍永久能力。

## Considered Options

- **运行时生态探测选默认表**：引入探测顺序、混合项目归属、失败处理等新问题，而 glob 展开 + 静默 skip 是既有行为——拒绝。
- **显式 `ecosystem` 配置项切换默认**：新增用户要理解的旋钮，与"清单无需用户定制"的目标相悖——拒绝。
- **砍掉 text 兜底（工具只更新认识的格式）**：pkl / .env / `VERSION` 等用户自造文件不会有插件覆盖，等于永久移除该能力，且偏离已验证的上游行为；maven 误伤窗口随 maven 插件落地自动关闭——拒绝。
- **只修 `read_version`、默认表不动**：纯 cargo 裸跑"版本读得到、无文件可更新"（上游语义自洽但用户观感如损坏），A 痛点只修一半——拒绝。

## Consequences

- 默认行为偏离上游 v11 的 6 文件清单（API 形状不变）；纯 cargo 项目裸跑从报错变为完整闭环（读 `Cargo.toml`、更新、顺带 lock 同步）。
- 当前版本来源可以是 `Cargo.toml`：normalize 排序后首个可读文件为准——混合项目中 cargo 清单按字典序先于 package.json（`C` < `p`）；各生态版本号本应一致，漂移时以排序首个为准。
- core `normalize_files` 的 recursive 分支（`bump.rs` 硬编码 `packages/**/package.json`，core API 直连用户可触发）一并改为链上 `**/` 并集。
- 落地修正：Cargo 清单 basename 常量由 `cargo.toml` 改为磁盘惯例名 `Cargo.toml`——同名常量是 recursive glob 模式与探测读取的来源，大小写敏感文件系统（Linux）上小写模式命中不到真实文件；`matches` 识别面保持大小写不敏感（小写比较）不受影响。napi `versionFileManifestGlobs()` 返回值随之由 `**/cargo.toml` 变为 `**/Cargo.toml`（COL-31 发布的模式表修正）。
- 未来 maven / gradle：`read_version` 随插件一并落地，默认清单、recursive 收集、版本来源三处自动涵盖，编排层零改动。
- 已知 node-only 遗留：`npm/bump/src/changelog.ts` 的 `git add package.json` 硬编码（纯 cargo 项目 git pathspec fatal）——待 changelog Rust 改版时一并处理。
