# 用纯 Rust + napi 核心替换 bumpp 依赖

将对上游 `antfu/bumpp` 的依赖（以及实验性的 `packages/next`）替换为自研的纯 Rust 实现 `@vill-v/bumpp-core`（`napi/bumpp-core`，自 ADR-0005 起由 `npm/` 迁入），通过 napi-rs 向 Node.js 暴露接口。行为语义全量对齐上游 bumpp v11，但配置文件只支持 JSON。

## Decisions

- **范围**：四个 API 全部进 Rust——`versionBump`（文件版本更新 + git commit/tag/push + npm scripts）、`versionBumpInfo`（候选版本计算 + 交互 prompt）、`loadBumpConfig`、`ProgressEvent`。
- **配置仅支持 JSON**：`loadBumpConfig` 只加载 JSON 配置文件，不再执行 `bumpp.config.ts`；上游的 `customVersion` 函数选项随之砍掉（JSON 无法承载函数），prompt 中不出现 "from config" 选项。
- **prompt 由 Rust 渲染**（dialoguer/inquire 类 crate），JS 只是薄壳转发；上游的 autocomplete + custom 二次输入行为保留。
- **语义全量对齐上游 bumpp v11**：含 preid 规则（沿用当前预发行标识，否则用入参，上游 normalizeOptions 缺省为 `'beta'`）、`0→1` 修正、`next`/`conventional`（含 git log + 约定式提交解析）、`custom`/`none`。
- **git/npm 操作 shell out** 到 `git`/`npm` 二进制，继承用户的 git config / SSH / GPG / credential helper。
- **progress 事件**经 napi 异步任务 + ThreadsafeFunction 实时回传 JS，不阻塞事件循环。
- **删除 `packages/next`**：本重写即其替代品。
- **分发为预编译平台包**：主包 + 每平台 optionalDependencies。v1 目标：darwin-arm64、linux-x64-gnu、linux-arm64-gnu、win32-x64-msvc、win32-arm64-msvc，OpenHarmony（ohos 三元组）best-effort。
- **版本线统一**：`@vill-v/bumpp-core` 加入全仓统一版本线。**本任务完成不发版**——后续可能还有 oxc 加载 TS 配置、changelog 生成 Rust 化等变更；因含破坏性变更，未来实际发版时应为 major。
- **工具链**：Rust 由 mise 安装与版本管理。
- **仓库布局**：`crates/` 存纯 Rust 库，`napi/` 存不发版的内部绑定，`npm/` 存要发版的 npm 包；`npm/` 优先级高于 `napi/`——会发版的绑定包直接放 `npm/`（见 AGENTS.md 仓库布局）。
- **测试双层**：cargo test 移植上游关键单测（版本计算矩阵、文件更新、模板替换）；vitest 对编译产物 `.node` 做全链路集成测试（含 git 临时仓库）。

## Considered Options

- **git2 crate（libgit2）**：纯库调用但需自行实现 SSH 认证 / GPG 签名 / credential helper，行为与 git CLI 有差异——拒绝。
- **Rust 内嵌 JS 引擎执行 TS 配置**（boa/deno_core）：复杂度爆炸——拒绝，改为仅支持 JSON 配置。
- **安装时 build from source**：要求用户装 Rust 工具链，对 CLI 工具不友好——拒绝，采用预编译平台包。
- **对齐 `packages/next` 的简化语义**（固定 7 选项、preid 默认 `'beta'`）：与生产行为有出入——拒绝，全量对齐上游。

## Consequences

- 破坏性变更：TS 配置与 `customVersion` 不再支持。本任务完成不发版；CHANGELOG 迁移说明与 major 版本动作留待未来实际发版时处理。
- 发版构建经 **GitHub Actions 官方 napi-rs CI 模板**（仓库已迁移至 GitHub）：每个 target 在对应平台家族原生 runner 构建（macos 编 darwin-arm64、ubuntu 编 linux-x64-gnu + napi-cross 编 linux-arm64-gnu、windows 编两个 msvc），test-bindings 矩阵在各平台真实跑 vitest；开发内循环保持本机架构单编。OpenHarmony（ohos 三元组）best-effort 待 NDK 就绪后补入矩阵。
- `npm/bump` 的 `bumpp` 依赖替换为 `@vill-v/bumpp-core`，API 形状兼容，调用点（`bump.ts` / `config.ts` / `types.ts`）几乎不改。
