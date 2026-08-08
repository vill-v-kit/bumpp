# Scripts 使用配置声明的通用 shell 命令

版本流程的 `preversion`、`version`、`postversion` 是跨生态的时序 hook，不从 `package.json` 自动读取 npm scripts。命令由统一配置的 `scripts` 字段声明，避免纯 Cargo 项目依赖 Node 文件与 npm。

## Decisions

- **字段与时序**：`.vbumpprc.{json,jsonc,toml}`、全局配置或 overrides 可声明 `scripts.preversion`、`scripts.version`、`scripts.postversion`。执行顺序分别为更新文件前、git 操作前、git 操作后。
- **执行方式**：Unix 使用 `sh -c`，Windows 使用 `cmd /d /s /c`。需要特定 shell 时由命令显式调用，例如 `pwsh -Command ...` 或 `zsh script.zsh`。
- **失败语义**：脚本非零退出立即中止 bump 并传播错误，避免构建或校验失败后继续产生完整发布。
- **开关**：`ignoreScripts` 跳过这些配置脚本。
- **职责边界**：scripts 是编排层 hook，不进入生态插件链；`execute` 保持独立的无 shell 执行语义。

## Consequences

- `package.json` 的 `preversion`、`version`、`postversion` 不会自动执行。Node 项目若需要原行为，应在配置中显式声明 `npm run preversion` 等命令。
- Cargo 与其他生态项目可以使用同一组 hook，例如 `preversion = "cargo fmt --check"`。
