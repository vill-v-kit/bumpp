# JavaScript 包管理器检测

JavaScript install 插件负责从当前目录逐级向上检测包管理器，并执行 `<name> install`。检测逻辑保留在 `vbumpp-core` 内，不拆独立 crate；检测结果只表示包管理器名称，不承担版本选择或完整 Corepack 语义。

## Decisions

- 目录是外层优先级：从 cwd 向文件系统根逐级检测，最近目录先命中。
- 每级依次检测 lockfile / workspace 文件、顶层 `packageManager`、`devEngines.packageManager`。
- 支持的名称为 `npm`、`yarn`、`pnpm`、`bun`、`deno`、`nub`、`aube`。lockfile 表覆盖对应上游默认名单，包括 `npm-shrinkwrap.json`、`nub.lock`、`aube-lock.yaml` 与 `aube-workspace.yaml`。
- 顶层 `packageManager` 读取 `<name>@<version>` 的 name 部分；未知或无效值视为未命中并继续检测。
- `devEngines.packageManager` 只接受单个对象，且 `name` 必须是受支持的字符串。字符串、数组、缺失或无效 name、未知名称均宽容回退，不新增警告或错误。
- `devEngines.packageManager` 的 `version`、`onFail` 及其他属性不参与调度；两个声明冲突时由更高优先级信号决定。
- 检测位于 JavaScript install 插件中，不进入版本清单扫描、napi 或 TypeScript 层。出现第二个真实消费方时再评估拆 crate。

## Consequences

- lockfile、workspace 文件、顶层声明和 `devEngines` 声明均可选择正确的 install 命令。
- vbumpp 不复刻 Corepack 的版本下载、范围校验、冲突与失败策略。
- 新检测信号和支持名单的维护点集中在 JavaScript install 插件及其测试矩阵。
