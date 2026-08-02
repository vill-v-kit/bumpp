# scripts 通用化为配置声明的 shell 命令

上游 v11 的 `preversion` / `version` / `postversion` 三脚本位从 package.json `scripts` 字段读取、经 `npm run` 执行——node-only 假定：package.json 缺失即 ENOENT 错误传播，纯 cargo（及未来生态）项目 bump 在 preversion 步必炸（ADR-0009 排查发现）。决策：scripts 改为 `bump.config.json` 声明的通用 shell 命令，npm scripts 专属通道移除。

## Decisions

- **`scripts` 配置字段**：`bump.config.json`（或 overrides）声明 `scripts: { preversion?, version?, postversion? }`，值为 shell 命令字符串；三槽位时序不变（preversion → updateFiles 前，version → git 前，postversion → git 后）。
- **shell 执行**：经 `sh -c`（Windows `cmd /d /s /c`——`/d /s` 对齐 npm 默认，跳过注册表 AutoRun 钩子）——config 为可信声明，通用 hook 常需管道与复合命令；`execute` 选项的上游 no-shell tokenize 语义不同源，保持原样、互不影响。不选 PowerShell / zsh 之类"平台原生 shell"：hook 需在全部协作者机器与 CI 上行为一致，`sh` / `cmd` 是无处不在的最小公分母（zsh 在多数 Linux/CI 缺失且与 POSIX sh 有语义差；PowerShell 5.1 与 7 语法互不兼容、7 非自带）；需要特定 shell 时命令串内显式调用（`pwsh -Command ...` / `zsh script.zsh`）。
- **npm scripts 通道移除**：`run_npm_script`（读 package.json scripts + `npm run`）删除，`bump.rs` 三 `script_step!` 调用点改读 config scripts。node 项目迁移路径：bump.config.json 显式声明 `"preversion": "npm run preversion"`。
- **`ignoreScripts` 保留**：对 config scripts 生效，语义不变。
- **脚本非零退出即报错传播**：配置声明的钩子失败时发版中止（对齐 ADR-0003 失败即报错精神），有意偏离上游 npm scripts 未开 throwOnError 的不传播 parity——静默继续会让失败的构建/校验钩子产出完整发版，风险不可接受。
- **`ProgressEvent::NpmScript` → `Script`**：事件不向 Node 层导出（ADR-0002），改名无 API 影响；事件负载由脚本名改为命令本体。

## Considered Options

- **保留 npm scripts + package.json 缺失静默跳过**：纯 cargo 不再炸，但 node 生态专属通道残留，与生态化方向相悖，且 cargo 无等价物造成能力不对称——拒绝。
- **scripts 插件化**（`plugins/` 第四能力子目录）：scripts 是编排层时序 hook——触发点为 bump 流程步骤，不经文件分发链；且当前无第二生态有 hooks 概念。拒绝，未来出现时再议。
- **no-shell tokenize**（对齐 `execute`）：通用 hook 常需 `&&`、管道等复合命令，无 shell 表达不了——拒绝。

## Consequences

- 偏离上游：package.json 中的 preversion / version / postversion **不再自动执行**——node 项目须在 bump.config.json 显式声明。
- 纯 cargo 项目三脚本位可用任意命令（如 `"preversion": "cargo fmt --check"`）；bump 全流程无 node 残留阻断点。
- `scripts.rs` 由"npm scripts 执行"改造为通用脚本执行原语，仍为独立模块（编排层时序 hook，不入 `plugins/`）。
