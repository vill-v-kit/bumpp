# 用户可见字符串统一英文，changelog types 中文定制移项目级配置

包的用户可见字符串（错误信息、CLI help/用法、交互 prompt、进度打印、panic 兜底文案）与 changelog 内建 types 标题原本均为中文定制（fork 自 init 提交即中文；上游 antfu/bumpp 本体无 types 默认，changelog 委托 changelogen）。决策：用户可见字符串唯一语言为英文；types 键集/声明序/emoji 不变，title 取 changelogen 英文措辞；本仓库所需的中文标题作为项目级定制移入 `.vbumpprc.toml`。

## Decisions

- **翻译范围 = 全部用户可见字符串**：`crates/bumpp-core` 的错误信息、`cli.rs` help/用法、dialoguer prompt（token 录入、版本选择、bump 确认）、progress 打印、`expect`/`unreachable!` 兜底，以及 napi loader（`napi/bumpp-core/index.js`）的平台加载报错。代码注释不动（仓库内部工作语言为中文）；测试断言同步英文化，测试中的中文 fixture 数据保留（见 Consequences）。
- **types 内建默认英文化**：11 键集/声明序/emoji 全不变，title 换 changelogen 英文措辞——feat `🚀 Enhancements`、perf `🔥 Performance`、fix `🩹 Fixes`、refactor `💅 Refactors`、examples `🏀 Examples`、docs `📖 Documentation`、chore `🏡 Chore`、build `📦 Build`、test `✅ Tests`、BreakingChange `🚨 Breaking Changes`、style `🎨 Styles`。`markdown.rs` 贡献者节头 `### ❤️ 贡献者` → `### ❤️ Contributors`——ADR-0012 申报偏差①（中文节标题直生）随之移除，golden fixtures 回到 changelogen 原生英文产出（仅剩 ungh.cc 剥除变换）。
- **中文定制移项目级配置**：本仓库 `.vbumpprc.json` → `.vbumpprc.toml`，`files` / `changelog.excludeAuthors` 原样迁入，`[changelog.types.*]` 11 组中文 title 与旧内建逐字一致——本仓库 CHANGELOG 产出与历史归档保持一致。types 按键深合并语义（ADR-0013）天然承载：项目配置只覆盖 title，键位/声明序不动。

## Considered Options

- **完全对齐 changelogen 原生默认键集**（12 键：多 `types` 🌊 Types / `ci` 🤖 CI，删 BreakingChange）：BreakingChange 组是 conventional 提交 `!` 标记的破坏性改动段落的承载键，删除会改变生成结构——拒绝，仅措辞对齐。
- **保留中文内建 + 本仓库英文配置覆盖**：语言归属倒置，与「包默认英文、定制在项目配置」的目标相反——拒绝。
- **引入 i18n 机制**：为一套静态文案做运行时语言切换属过度设计——拒绝；英文单语 + types 配置定制已覆盖诉求。

## Consequences

- 未配置 types 的下游用户，changelog 分组标题由中文变英文；全部错误信息/CLI 输出由中文变英文——发版时按用户可见行为变化显式通告，中文标题配置示例即本仓库 `.vbumpprc.toml`。
- ADR-0012 申报偏差清单由三处减为两处（② 无 ungh.cc 链接、③ `chore(deps)` 过滤内置）。
- 测试中的中文 fixture（自定义 title「✨ 新特性」「项目特性」等）保留——它们是「中文定制经配置注入仍可用」的锚点。
