# 配置 Schema 单一事实源与编辑器分发

配置形状此前散落多处——顶层键白名单 const、默认值表、changelog 手写解析、npm 包手写 TS interface——彼此靠人工同步；且文件层只校验键名不校验类型，类型不符静默回落默认（如 `files: "x"` 致空跑）。决定以 Rust 结构体为单一事实源，机械导出 JSON Schema 与 TS 类型；文件层升级为键名+类型双重校验；schema 经 npm 包副本、website 静态导出与 `vbumpp schema` 子命令分发，让编辑器提示与配置验证工程化。

## Decisions

- **单一事实源**：配置形状在 vbumpp-core 以 Rust 结构体定义（serde `Deserialize` + schemars `JsonSchema` 派生），JSON Schema 由结构体机械导出。白名单 const 保留为定制报错 UX 的载体（未知键一次全列、`customVersion` / gitlab 段 / changelog 遗留键的专属文案均为 pre-pass，serde 拿不到这些形态），与结构体键集以测试钉死防漂移。
- **文件层类型校验**：`read_config` 在键名 pre-pass 之后经结构体反序列化做类型校验——类型不符从静默回落默认改为报错（`files` 非数组、`scripts` 非对象、`commit` 非 bool|string 等即时报错）。`$schema` 键进白名单并在 schema 中声明为合法属性，供编辑器关联。
- **overrides 类型化边界**：napi `bumpVersion` 入参由 `Map<String, Value>` 改为 `#[napi(object)]` 结构体，TS 类型由 napi 自动生成；npm 包手写 `Config` / `ChangelogOptions` 门面删除，改为再导出生成类型（平台变体包同）。文件层结构与 napi 结构同处相邻定义（联合类型字段 napi 侧用 `Either`，孤儿规则使单一结构体不可行），round-trip 测试钉死两视图一致。
- **overrides 键名校验让位于编译期**：napi object 边界静默丢弃未知键，overrides 的键名把关只剩 TS 编译期（对象字面量 excess property check）；类型不符仍由 napi 运行期报错。接受纯 JS 调用方与非常量对象维持静默——编程式调用方以 TS 为主，手写配置的主战场在文件层，而文件层已有运行期严格校验与编辑器 schema 实时校验。
- **合并载体不变**：四层合并仍走 `serde_json::Map`——浅替换 + `changelog.types` 按键深合并 + `false` 删键是值域操作，struct 字段级合并表达不了。结构体是校验与 schema 的载体，Map 是合并的载体。
- **`vbumpp schema` 子命令**：stdout 打印纯 JSON（CI 再生、管道重定向共用）；`--write` 落盘，`--project`（默认）写 `./vbumpprc.schema.json`、`--global` 写 `~/.vbumpp/schema.json`，落点按显示路径规范打印。离线用户以相对路径 `$schema: "./vbumpprc.schema.json"`（VS Code）或 `#:schema` 指令（Taplo）引用本地副本。
- **分发与防腐**：schema JSON 提交进仓库两处（npm/bump 包内副本、website 静态导出），由 `scripts/` TS 脚本调 `vbumpp schema` 再生，ci.yml 加漂移校验腿（重跑 diff，不一致即红，同 demo-cast 模式）；文档站 Pages URL 为规范地址。三格式共用同一份 schema，不维护 TOML 单独版本。（2026-08-16 修正：取消向 SchemaStore 提交收录的计划——编辑器提示只走显式 `$schema` 键 / Taplo `#:schema` 指令与离线本地副本，TOML 侧不再有目录自动关联通路。）

## Consequences

- 文件层报错承诺升级：文档从「写错键名报错」扩为「键名与类型双重校验」；对旧配置里类型写错的用户，报错是修复静默失效而非破坏，migration 页无需新增条目。
- 编程式 API 入参行为变更（类型不符从静默回落改为 napi 运行期报错、未知键由 TS 编译期把关）随 minor 发版并在 release note 说明，不攒 v7。
- schema 内 description 属用户可见字符串，唯一语言英文（ADR-0017）。
- Pages URL 长期稳定，内容随发版更新、地址不变——文档与用户的 `$schema` / `#:schema` 引用均指向该地址。
- `vbumpp schema` 扩展 CLI argv 面，归属与解析仍循 ADR-0016 手写解析器。
