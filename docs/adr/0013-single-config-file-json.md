# 配置文件统一为 .vbumpprc.json

> **部分取代说明**：「JSON-only」「非 .json 扩展名报错」「单一文件层级」三条由 ADR-0015 取代（扩展为 JSONC + TOML 与项目/全局两级）；其余决策（单一解析路径、严格 schema、merge 语义、`configFilePath` override、旧名不探测）维持不变。

三个配置源并存：Rust 加载的 `bump.config.json`、esconf 加载的 `vbumpp.config.{ts,js,mjs,cjs}` / `vbumpp.json`、c12 加载的 `changelog.config.*`（另含 package.json `changelog` 键与 `.env` 两个隐蔽源），由 `resolveConfig` 以 defu 三层缝合。决策：单一文件 `.vbumpprc.json`，加载全权收归 Rust，JSON-only（暂不执行 TS/JS 配置）。

## Decisions

- **改名 `.vbumpprc.json`**：rc 文件惯例，对齐 CLI 名 `vbumpp`；ADR-0011 中 `bump.config.json` 字样由本 ADR 取代（scripts 决策本体不变）。`configFilePath` override 仍可显式指任意 JSON 路径。
- **形状**：bumpp 键居顶层，`changelog` 段与 `scripts` 字段并列；程序化 overrides 与文件同形（TS `Config` 扁平化，Rust 合并单一路径无嵌套特判）。
- **加载语义**：overrides > 文件 > 内建默认。changelog 段 `types` 按键深合并（改单个标题不抄全 10 条默认表；值为 `false` 即禁用该组），其余键整体替换；bumpp 侧维持上游浅合并 parity 不动。中文标题等 changelog 默认值内建 Rust（原 JS `getDefaultsChangeLogConfig` 迁入）。
- **单一解析路径**：全项目只有一条配置文件解析逻辑——changelog 段与 bumpp 键同源同读；解析结果不向 JS 导出（无 `loadChangelogConfig` napi 面），由 `generateChangelog` 内部消费，JS 仅透传用户 overrides。
- **严格 schema**：未知键、已知但未支持键（`tokens` / `publish` / `templates.commitMessage` / `templates.tagMessage` 等 changelogen 遗产）、文件内的 `from` / `to` / `newVersion`（运行时入参）一律报错并报键名；沿用 `config.rs` 拒绝 `customVersion` 的先例。
- **旧文件名不探测**：loader 只认 `.vbumpprc.json`（或 `configFilePath` override）；`bump.config.{json,ts,mts,cts,js,mjs,cjs}` / `vbumpp.config.*` / `vbumpp.json` / `changelog.config.*` 静默失效、不读不报错；原 `bump.config.ts` 脚本配置检测随改名一并拆除。package.json `changelog` 键静默忽略。迁移说明由文档承担（ADR-0012 收尾工单）。
- **`resolveConfig` / `defineConfig` 移除**：`changelog` 加载与 bumpp 同源后缝合层无存在必要；其残留职责分流——recursive 展开并入 Rust `load_bump_config`（merged `recursive==true` 时内部展开插件链模式表 + 去重 + 置 `false`，对 JS 透明），accesstoken 读取留 JS（AES-256-GCM 凭证存储 `~/.vbumpp/tokens.bin`，非配置文件加载，不在 Rust 化范围）。`config.test.ts` 删除，recursive 用例移植 Rust 层。
- **TS/JS 配置不执行**：与 `bump.config.ts` 检测同先例——脚本配置无法在不嵌入 JS 运行时的情况下加载，JSON 是当前唯一诚实支持的面。

## Considered Options

- **保留 esconf 层**（`vbumpp.config.*` 继续作为 overrides 源）：三源缝合正是复杂度之源，且 TS 配置执行与「加载归 Rust」直接冲突——拒绝。
- **旧名检测报错**（探测旧文件并给迁移指引）：多一份探测代码与报错面，旧文件静默失效 + 文档迁移说明已足够——拒绝。
- **JS 导出 `loadChangelogConfig`**：第二条配置解析面，与「统一解析逻辑」相悖且下游零消费者——拒绝。
- **宽松 schema 忽略未知键**：错误推迟到「配置没生效」的困惑时刻，typos 与遗产键永不暴露——拒绝。
- **changelog types 浅合并**（对齐 bumpp 侧）：改一个标题须抄全默认表；bumpp 浅合并是上游 parity 约束，changelog 无此约束——拒绝。
- **嵌套 overrides 形状**（`{bumpp, changelog}`）：Rust 须为 `bumpp` 键特判，与文件形状不一致——拒绝。

## Consequences

- 破坏性改动（随大版本发布）：`bump.config.json` / `vbumpp.config.*` / `changelog.config.*` 静默失效（迁移说明见文档）；`defineConfig` 及四个 release 包的再导出删除；TS `Config` 扁平化。
- npm/bump 依赖再删 `@esconf/core`、`@esconf/preset-mini`、`defu`（连同 ADR-0012 的 changelogen 链）。
- 配置优先级由「程序化 > vbumpp.config > {changelog.config, bump.config}」简化为「overrides > `.vbumpprc.json` > 内建默认」。
