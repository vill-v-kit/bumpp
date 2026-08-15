# 项目与全局统一配置

项目配置与用户级配置共享一条 Rust 解析路径，采用同一严格 schema 和合并语义。项目级配置位于 `.vbumpprc.{json,jsonc,toml}`，全局级配置位于 `~/.vbumpp/config.{json,jsonc,toml}`；全局配置与 token 存储同属 `~/.vbumpp/`，不引入 XDG。

## Decisions

- **配置层级与路径**：加载顺序为 overrides > 项目 `.vbumpprc` > 全局 `config` > 内建默认。`VBUMPP_HOME` 覆盖全局目录；`VBUMPP_TOKEN_STORE` 仅覆盖 token 存储文件路径，且优先于 `VBUMPP_HOME`。项目级 `configFilePath` override 按指定文件精确加载，替代项目层探测，全局层仍叠加。
- **格式**：`.json` 与 `.jsonc` 均使用 JSONC 解析，支持注释和尾逗号；`.toml` 使用 TOML 解析。配置文件扩展名仅支持 `.json`、`.jsonc`、`.toml`，其他扩展名报错并列出支持格式。TOML datetime 因无法表达为 JSON 值而拒绝。
- **文件探测**：项目级探测 `.vbumpprc.json`、`.vbumpprc.jsonc`、`.vbumpprc.toml`；全局级探测 `config.json`、`config.jsonc`、`config.toml`。同级命中多个文件即报错并全部列出，不静默选择。旧名 `bump.config.*`、`vbumpp.config.*`、`vbumpp.json`、`changelog.config.*` 不探测、不读取，静默失效；`package.json` 的 `changelog` 键也不参与配置。
- **配置形状**：`bumpp` 键居顶层，`changelog` 段、`scripts` 字段与 `gitlab` 段并列；程序化 overrides 与文件同形。配置加载全权归 Rust，解析结果不向 JS 导出；overrides 经 napi 类型化结构体边界入 Rust，TS 类型由 napi 自动生成（ADR-0037）。
- **合并**：bumpp 键按浅合并整体替换；`changelog.types` 按键深合并，值为 `false` 时禁用该组；changelog 段其他键整体替换。优先级逐层生效，overrides 位于最高层。
- **严格 schema**：未知顶层键、已知但未支持的遗留键，以及配置文件内的运行时入参 `customVersion`、`from`、`to`、`newVersion` 均报错并指出键名；文件层在此基础上做类型校验，类型不符即报错而非静默回落默认，`$schema` 键为合法自引用（ADR-0037）。`gitlab` 段仅允许 `host` 字符串。
- **配置执行**：不执行 TS/JS 配置，不保留 `resolveConfig` / `defineConfig` 或第二条 changelog 配置解析面。recursive 展开和 files 去重由 Rust loader 完成；token 读取属于独立凭证存储机制，不是配置文件加载。

## Consequences

- 用户迁移到 `.vbumpprc.{json,jsonc,toml}`；全局偏好可在 `~/.vbumpp/config.{json,jsonc,toml}` 配置，并通过统一 schema 与项目配置合并。
- `scripts` 为通用 shell 命令配置字段，三槽位顺序和失败传播规则见 ADR-0011；node 项目需显式声明 `npm run ...`。
- changelog 默认值由 Rust 内建，项目可通过 `changelog.types` 定制标题；用户可见默认文案为英文，见 ADR-0017。
