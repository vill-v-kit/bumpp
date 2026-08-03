# 迁移指南：v5 → v6

v6 将 changelogen 使用面整体重写为 Rust（ADR-0012），并把三个配置源统一为单一
`.vbumpprc.json`（ADR-0013）。本指南覆盖全部破坏性改动与行为变化。

## 配置文件：单一 `.vbumpprc.json`

**旧文件全部静默失效**（不探测、不读取、不报错）：

- `bump.config.json` 及 `bump.config.{ts,mts,cts,js,mjs,cjs}`
- `vbumpp.config.{ts,mts,cts,js,mjs,cjs}` / `vbumpp.json`
- `changelog.config.*`
- package.json 的 `changelog` 键

**新形状**（bumpp 键居顶层，`changelog` 段与 `scripts` 字段并列）：

```json
{
  "commit": true,
  "tag": true,
  "push": true,
  "files": [],
  "scripts": {
    "preversion": "cargo fmt --check",
    "version": "",
    "postversion": ""
  },
  "changelog": {
    "output": "CHANGELOG.md",
    "types": {
      "feat": { "title": "🚀 特性" },
      "chore": false
    },
    "repo": "owner/repo",
    "scopeMap": {},
    "noAuthors": false,
    "hideAuthorEmail": true,
    "excludeAuthors": [],
    "templates": { "tagBody": "v{{newVersion}}" },
    "commitMessage": "chore: update {{output}}"
  }
}
```

**迁移对照**：

| 旧位置 | 新位置 |
| --- | --- |
| `bump.config.json` 顶层键 | `.vbumpprc.json` 顶层键（键名不变） |
| `vbumpp.config.*` 的 `bumpp` 键 | 顶层（拍平） |
| `vbumpp.config.*` 的 `changelog` 键 | `changelog` 段 |
| `changelog.config.*` 全部内容 | `changelog` 段（键集收窄，见下） |

**合并语义**：overrides > 文件 > 内建默认；`types` 按键深合并（改单个标题不抄全表，
值为 `false` 即禁用该组），其余键整体替换。

**`changelog` 段键集与默认值**：

| 键 | 默认 | 说明 |
| --- | --- | --- |
| `output` | `"CHANGELOG.md"` | 写出路径 |
| `types` | 11 组中文标题 | 声明序即分组序；`false` 禁用 |
| `repo` | 无（自 git remote / package.json `repository` 解析） | string 或 `{provider, domain, repo}` |
| `scopeMap` | `{}` | scope 显示重命名 |
| `noAuthors` | `false` | 整节关闭贡献者列表 |
| `hideAuthorEmail` | **`true`**（翻转 changelogen 默认） | 贡献者行隐邮箱 |
| `excludeAuthors` | `[]` | 子串匹配排除 |
| `templates.tagBody` | `"v{{newVersion}}"` | `## ` 头模板（仅支持此一键） |
| `commitMessage` | `"chore: update {{output}}"` | changelog 提交信息 |

**严格 schema**（写入即报错并报键名）：未知键；changelogen 遗产键 `tokens` /
`publish` / `templates.commitMessage` / `templates.tagMessage`；运行时入参 `from` /
`to` / `newVersion`（它们永远由调用方在运行时传入）。

## 程序化 API（`@vill-v/bumpp`）

- **`Config` 扁平化**：`{ bumpp: {...}, changelog: {...} }` → `{ ...bumpp键, changelog: {...} }`，
  与 `.vbumpprc.json` 同形
- **`defineConfig` 移除**（`@vill-v/bumpp` 及四个 release 包的再导出同步删除）——
  配置就是普通对象，不再需要辅助函数
- **`ResolveConfig.changelog` 为用户透传段**：解析统一发生在 Rust 内部
  （单一解析路径），JS 不再有 changelog 配置解析态

## 子路径改名与导出收窄

`@vill-v/bumpp/changelogen` → **`@vill-v/bumpp/changelog`**。

| 旧导入 | 新位置 |
| --- | --- |
| `resolveRepoConfig` | `@vill-v/bumpp/changelog`（Rust 重写，返回结构不变） |
| `getCurrentGitBranch` | 同上 |
| `getLastGitTag` / `getGitDiff` / `generateChangelog` | 同上（新可用） |
| changelogen 其余导出（`loadChangelogConfig` / `parseCommits` / `generateMarkDown` / `createGithubRelease` / `syncGithubRelease` / `bumpVersion` / `parseChangelogMarkdown` 等） | **不再提供**（使用面外能力未移植） |

## 行为变化

- **贡献者行默认隐邮箱**（`hideAuthorEmail` 默认翻转 `false` → `true`）
- **贡献者节无 `@username` 链接**：ungh.cc 作者解析不移植——changelog 生成全程
  无网络、产出不再随网络环境漂移，贡献者邮箱也不再外发第三方
- **changelog 的 git 提交跟随 bumpp `commit` 开关**：`commit: false` 时只写文件
  （原行为为无条件提交）；提交信息经 `changelog.commitMessage` 配置
- **`ci` / `types` 类型提交不再出现在 changelog**：原行为系 c12 defu 缝合
  changelogen 英文默认组的副产物，未纳入内建默认；需要时可自行在 `types` 声明
- **gitmoji 键名按字面量匹配**（原实现未转义拼正则，`:heavy_plus_sign:` 等含 `+`
  键在病理输入下行为不同）

## 修复（原为 bug）

- **纯 cargo 项目可用**：changelog 步骤不再因 `git add package.json` 的 pathspec
  报错整步失败；`git add` 仅含实际写出的 output 文件
- **非 v 前缀 tag 支持**：diff 与 compare 链接以 `getLastGitTag` 返回的真实 tag
  名为界（原硬编码 `v` 前缀，非 v 前缀项目找不到 ref）

## 依赖变化

`@vill-v/bumpp` 移除：`changelogen`（及其传递链 c12 / ofetch / convert-gitmoji /
semver 等）、`@esconf/core`、`@esconf/preset-mini`、`defu`、`tinyglobby`、`tinyexec`。
