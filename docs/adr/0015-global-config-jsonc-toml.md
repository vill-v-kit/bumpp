# 全局配置目录与 JSONC/TOML 多格式配置

取代 ADR-0013 的三条条款：「JSON-only」「非 .json 扩展名报错」「单一文件层级」。动机有二：Token 存储所在的全局目录（`~/.vbumpp/`）应同时承载全局通用配置（自建 GitLab host、个人 sign/noVerify 偏好等典型用例）；JSON 不支持注释，用户要求注释友好的格式。YAML 的 Rust serde 生态呈 fork 废墟（见下），最终格式定为 JSONC + TOML。

## Decisions

- **全局配置目录**：全局配置文件放 `~/.vbumpp/`（与 `tokens.bin` / `key.bin` 同目录），basename `config` 与项目级 `.vbumpprc` 区分层级。`VBUMPP_HOME` 覆盖整个目录；路径优先级 `VBUMPP_TOKEN_STORE`（兼容保留）> `VBUMPP_HOME` > `~/.vbumpp`；`dirs::home_dir()` 解析（与 Node `os.homedir()` 同语义），不引入 XDG。
- **四层合并**：`overrides > 项目 .vbumpprc > 全局 config > 内建默认`——ADR-0013 链条中间插一层，merge 语义不变（bumpp 键浅合并、changelog `types` 深合并逐层生效）。
- **键域同一 schema**：全局文件过同一条严格 schema 校验，无全局特判键域；`files` / `scripts` 等全局生效语义诡异的键靠文档引导（home 目录文件与用户 shell rc 同级信任）。
- **格式 JSONC + TOML**：`.json` 与 `.jsonc` 同走 JSONC 解析（注释、尾逗号可用；`.jsonc` 别名为照顾编辑器对 `.json` 内注释报错的团队场景）；`.toml` 用 `toml` crate；`jsonc-parser` 为既有依赖零增量。配置路径的 JSONC 解析走 `CollectOptions` 报错收集（带位置信息），不复用清单解析的容错吞错辅助。
- **探测文件名集合**：项目级 `.vbumpprc.{json,jsonc,toml}`，全局级 `config.{json,jsonc,toml}`。同级探测到 2 个及以上配置文件即报错并全部列出（多配置并存几乎一定是迁移事故），不做静默优先级。
- **`configFilePath` 按扩展名分派**：`.json` / `.jsonc` / `.toml` 可走；其余扩展名报错并列出支持格式（替换「仅支持 JSON」文案）。
- **严格 schema 跨格式一致**：未知键报错、`customVersion` / `from` / `to` / `newVersion` 拒收等校验不分格式；TOML datetime 等 JSON 无法表达的值遇到即报错。

## Considered Options

- **YAML**：serde 粘合层无健康选项——`serde_yaml` 归档、`serde_yml` 弃用（RUSTSEC-2025-0068 后成 noyalib 兼容壳）、`serde_yaml_ng` / `serde_norway` 为单人 fork；解析器层虽健康（`yaml-rust2` 月下载 436 万），但 TOML + JSONC 已覆盖「注释 + 友好语法」诉求，YAML 增量价值不抵选型风险；`saphyr-serde` 成熟后可再议——拒绝（暂缓）。
- **Pkl 替代 YAML**：`pkl-rs` crate 实质死亡（总下载 2.4K、2024-02 停更）；Pkl 评估器为 Kotlin/JVM 实现，Rust 绑定需外部 `pkl` CLI 评估进程，与 npm 预编译二进制分发模型冲突；且 Pkl 是可执行配置语言，与 ADR-0013「不执行脚本配置」原则相悖——拒绝。
- **全局与项目同名 `.vbumpprc.*`**：同一 loader 同一名称更简，但「不同层级不同名」在并列排查时更不易误读——拒绝。
- **全局键域黑名单**（禁 `files` / `scripts` 等）：多一份特判代码与报错面，单一 schema 单一路径优先——拒绝。
- **静默优先级替代多文件报错**（如 json > jsonc > toml）：把迁移事故藏进行为差异，违背 fail-fast 惯例——拒绝。

## Consequences

- ADR-0013 维持不变的部分：单一解析路径、严格 schema、merge 语义、`configFilePath` override、旧名不探测静默失效。
- 报错文案更新：「仅支持 JSON」→ 列出 `.json` / `.jsonc` / `.toml`。
- `gitlab.host`（ADR-0014 修复）成为全局配置的典型用例。
- TS `Config` 类型与文档需说明两级配置的合并优先级。
