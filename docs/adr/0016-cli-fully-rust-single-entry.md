# CLI 全权归 Rust：去 cac、cliRun 单入口与原生二进制终局

ADR-0014 把 JS 残留功能全收 Rust 时留了一个例外——「CLI 的 cac 参数路由除外」。本 ADR 移除该例外：argv 语法（子命令、flag、help 文案、错误提示、退出码）唯一归属 Rust，Node 侧只剩「把 argv 递过去」。终局目标是脱离 Node 分发的纯 Rust CLI 二进制；npm bin 与原生 bin 为共享同一 `cli::run_from_argv` 的两个薄壳前端，语法零漂移。

## Decisions

- **去 cac / consola，npm/bump 零运行时依赖**：`npm/bump` 依赖只剩 `@vill-v/bumpp-core`（workspace）；`bin/index.js` 为三行壳（argv 透传 + 退出码回写）。cac 的 help 排版风格随之弃用，help 与错误文案由 Rust 生成（沿用 dialoguer `console::style` 着色先例）。
- **单 napi 入口 `cliRun(argv, provider?)`**：返回 `Promise<number>` 即退出码，由调用壳回写 `process.exitCode`（Rust 不越权设宿主进程状态）。token 三件套（`tokenSet` / `tokenList` / `tokenRemove`）删除——唯一消费者是 cli.ts 的 switch，且 `@vill-v/bumpp-core` 从未发布，无兼容对象。
- **手写解析器，不引 clap**：语法盘子极小（默认命令 `[...files]` + `-o/--output` + `-r/--recursive`；`token <action> [name]`；`--help` / `--version`）。解析隔离在 cli 模块内部，未来子命令家族长大再迁 clap 只动该模块，入口签名不变。
- **cli 层落 `crates/bumpp-core/src/cli.rs` 模块**：与 `prompt.rs` / `progress.rs` 同层先例；不单立 `crates/bumpp-cli`——原生二进制真要发版时再机械拆分（bin crate 仅一行调用的 main.rs）。
- **变体 provider 走位置参数**：四个平台变体包（`@vill-v/bumpp-{github,gitlab,gitee,gitcode}`）的 bin 以 `cliRun(argv, 'github')` 注入身份，与 `bumpVersion(options, provider?)` 签名同构；token 子命令无视 provider。`createBaseCli` / `createCli` 与五个包的 `exports["./cli"]` 子路径整体删除。
- **ADR-0013 空 files 省略规则随迁**：cli.ts 中 `files.length ? { files } : {}` 的浅合并保护逻辑移入 Rust 的 bump 命令路径。

## Considered Options

- **cac 保留 + tagged union 结构化命令入口**：终局下 cac 与 Rust 两份语法并存、永远漂移，且 cac→union 翻译层在原生二进制落地后全成死代码——拒绝。
- **clap derive**：为不到其能力 5% 的语法让五平台 napi 构建每次多背编译税——现阶段拒绝，迁移通道经模块隔离保留。
- **即立 `crates/bumpp-cli`**：当前唯一消费者是 napi，为一个尚不存在的二进制提前付 workspace / 构建矩阵 / 发布配置脚手架——拒绝；拆分时机为原生二进制发版那一刻。
- **provider 经环境变量或 `--provider` flag**：前者为隐式通道（子进程继承、测试串味），后者污染用户可见语法且变体内语义混乱——拒绝。
- **保留 token 三件套作编程式 API**：唯一消费者为 cli.ts，包未发布——拒绝保留死面。

## Consequences

- ADR-0014 的「cac 参数路由除外」例外随本 ADR 移除，该 ADR 其余决策不变。
- 五个 npm 包 `exports` 面收缩：`./cli` 子路径删除（均未发版，无兼容对象）；`@vill-v/bumpp` 编程式 API（`bumpVersion`、类型）不变。
- argv 解析、分派、help、退出码的测试全部落 Rust `cargo test`；JS 侧不再有 CLI 逻辑可测。
- 原生 CLI 二进制成为纯增量工作：新增 bin crate + 分发通道，零设计返工。
- `--version` 输出取自 crate 版本号（ADR-0003 同步机制保证与 npm 包一致）。
