# 用户可见字符串统一使用英文

包向终端和调用方暴露的错误、help、prompt、进度与兜底文案统一使用英文。Changelog 内建分组标题也使用英文；本地化由项目配置承担，不引入运行时 i18n。

## Decisions

- **范围**：`vbumpp-core` 的错误、CLI help/用法、交互 prompt、进度、panic 兜底，以及 napi loader 平台错误均使用英文。代码注释属于仓库内部内容，不受此规则约束。
- **Changelog 默认值**：既有 types 键集、顺序和 emoji 不变，标题使用英文：`🚀 Enhancements`、`🔥 Performance`、`🩹 Fixes`、`💅 Refactors`、`🏀 Examples`、`📖 Documentation`、`🏡 Chore`、`📦 Build`、`✅ Tests`、`🚨 Breaking Changes`、`🎨 Styles`。贡献者标题为 `### ❤️ Contributors`。
- **本地化**：项目可在 `.vbumpprc.{json,jsonc,toml}` 的 `changelog.types` 中覆盖标题。本仓库使用 `.vbumpprc.toml` 保持中文 changelog 标题。

## Consequences

- 未配置本地化的用户获得英文 CLI、错误和 changelog 标题。
- 配置注入的任意语言标题仍受测试覆盖；无需在程序内维护多语言文案与选择机制。
