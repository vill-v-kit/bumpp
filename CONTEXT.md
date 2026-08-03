# vill-v/bump

版本发布工具集：以 Rust 核心驱动版本号更新、changelog 生成与多平台（GitHub/GitLab/Gitee/GitCode）release 创建。

## Language

**Bump**:
一次完整的版本发布操作——更新各文件中的版本号、生成 changelog、git commit/tag/push、执行 npm scripts。
_Avoid_: release（指平台 release 创建）、publish

**CLI**:
`vbumpp` 命令行——argv 语法（子命令、flag、help 文案、错误提示、退出码）唯一归属 Rust（Core 内的 cli 模块，手写解析器，ADR-0016）。npm bin 与规划中的原生 CLI 二进制为共享同一 `run_from_argv` 的两个薄壳前端；Node 侧零依赖、零逻辑。
_Avoid_: cac 路由（ADR-0014 时代的旧制，ADR-0016 移除）

**Release type**:
决定新版本号如何计算的选择项：`major` / `minor` / `patch` / `next` / `conventional` / `premajor` / `preminor` / `prepatch` / `prerelease`，以及 prompt 中的 `none`（保持不变）和 `custom`（手动输入）。
_Avoid_: bump type、version type

**Preid**:
预发行标识符（如 `1.0.0-beta.1` 中的 `beta`）。计算候选版本时沿用当前版本的预发行标识，否则用入参（上游 normalizeOptions 缺省为 `'beta'`）；预发行号从 1 开始（上游的 `0→1` 修正）。

**Scripts**:
bump 流程三时序槽位（`preversion` / `version` / `postversion`）的通用 shell 命令，声明于配置文件的 `scripts` 字段（ADR-0011）。
_Avoid_: npm scripts（上游旧义——从 package.json scripts 读取经 `npm run`，通道已移除）

**配置文件**:
工具配置的两级文件——项目级 `.vbumpprc.{json,jsonc,toml}` 与全局级 `~/.vbumpp/config.{json,jsonc,toml}`（ADR-0015；`.jsonc` 为 `.json` 的别名，同走 JSONC 解析，注释与尾逗号均可用）。加载全权归 Rust，四层合并：overrides > 项目 > 全局 > 内建默认；bumpp 键居顶层，`changelog` 段、`scripts` 字段、`gitlab` 段并列；全项目仅这一条解析路径，解析结果不向 JS 导出。同级探测到多个配置文件即报错并全部列出；旧名不探测、静默失效（ADR-0013）。
_Avoid_: bump.config.json（旧名）、vbumpp.config（esconf 旧制，已移除）、YAML 配置（不支持，ADR-0015）

**全局配置目录 (`~/.vbumpp/`)**:
用户级数据的家——全局配置文件与 Token 存储同放此目录。`VBUMPP_HOME` 覆盖整个目录；`VBUMPP_TOKEN_STORE` 仅覆盖 token 存储文件路径（兼容保留，优先级高于 `VBUMPP_HOME`）。
_Avoid_: XDG 目录（不引入）

**Token 存储 (Token store)**:
平台 access token 的加密凭证存储（`tokens.bin` + 同目录 `key.bin`），VBTK v1 二进制格式（magic + version + iv + authTag + AES-256-GCM 密文），Rust 全权管理且与 JS 时代逐字节兼容（ADR-0014）。防护级别为「防明文落盘」（非高安全保险柜）；明文 token 不跨 napi 边界进入 JS。
_Avoid_: 凭证库（语义过强）

**平台 Release**:
向 git 托管平台（github / gitlab / gitee / gitcode）创建 release 的动作，Rust 内按 provider 适配（gitee / gitcode 复用 github-like 实现；gitlab 多一步项目 id 直查，自建实例经 `gitlab` 段的 `host` 配置，ADR-0014）。token 解析链统一为：Token 存储 → 各家环境变量（`GH_TOKEN` → `GITHUB_TOKEN` / `GITLAB_TOKEN` / `GITEE_TOKEN` / `GITCODE_TOKEN`）→ 仅 github 追加 `gh auth token` 兜底。
_Avoid_: publish

**内部机制包 (Internal machinery package)**:
虽然发布到 npm、但设计上就不是给用户直接使用的包——判别问句："用户会直接 npm install 它吗？"不会即归 `napi/` 目录（ADR-0005，与是否发布无关）。成员：Core（napi 绑定本体）、Platform package（平台二进制分发包）；它们发布仅因 npm 不支持 workspace 协议与 optionalDependencies 按平台分发的机制需要。
_Avoid_: 内部包（过宽，`crates/` 亦属内部）、native package

**生态 (Ecosystem)**:
一套工具链及其版本文件与安装机制的集合（node / cargo；未来 maven、gradle）。bumpp-core 的生态知识集中于插件底座 `src/plugins/`（ADR-0010）：版本解析与更新（ADR-0007）、install 适配（ADR-0008，按本次 bump 实际更新的生态条件触发）、recursive 整树收集三能力子目录，各生态实现落同名文件。
_Avoid_: platform（指 OS/CPU 平台）、registry（指发布平台）

**插件底座 (Plugin base)**:
`src/plugins/`——全部生态能力的单一归属。trait + 静态链 + 编排在 `mod.rs`；插件类型在根部（`plugins/node.rs` 等），方法委托至 `version/` / `install/` / `recursive/` 三能力子目录的生态同名纯函数文件（Rust coherence：单 trait 的 impl 不可拆文件，ADR-0010）。
_Avoid_: files 插件链（ADR-0007 时代的旧称）

**清单 (Manifest)**:
生态认识的结构化版本文件（node：package.json 等 8 种 basename；cargo：Cargo.toml），以 basename 识别并走对应生态通道结构化更新、读取当前版本。其 basename 集合的单一事实源为插件底座链：默认清单（根级并集）与 recursive 模式表（`**/` 并集）均由链聚合（ADR-0009）。
_Avoid_: 版本文件（过宽——还含 text 通道的任意文本文件，如 README、.env）

**Core**:
纯 Rust + napi-rs 实现的版本、changelog 与平台 Release 引擎包 `@vill-v/bumpp-core`（`napi/bumpp-core`）。napi 面（ADR-0014 收缩、ADR-0016 再收缩后）：编排 `bumpVersion(options, provider?)`、平台 Release 四导出（`createGithubRelease` / `createGitlabRelease` / `createGiteeRelease` / `createGitcodeRelease`）、CLI 单入口 `cliRun(argv, provider?)`；token 三件套已删（ADR-0016），上游 parity 面（`versionBump` 系、`loadBumpConfig`）与 changelog 系函数收归 Rust 内部，`@vill-v/bumpp/changelog` 子路径移除。进度为 Rust 内置打印（ADR-0002），`ProgressEvent` 不向 Node 层导出。
_Avoid_: bumpp（指上游 antfu/bumpp 依赖本身）、next（实验包，已由 core 替代并删除）、changelogen（上游 unjs 依赖本身，使用面已重写并移除）

**Platform package**:
按目标平台分发预编译 `.node` 二进制的 npm 包（如 `@vill-v/bumpp-core-darwin-arm64`，`napi/bumpp-core-darwin-arm64`），作为主包的 optionalDependencies 安装。
_Avoid_: native package、binary package

**平台变体包 (Platform variant)**:
面向用户的 npm 包 `@vill-v/bumpp-{github,gitlab,gitee,gitcode}`——与主包同形，差别仅在 provider 身份注入：bin 经 `cliRun(argv, provider)` 位置参数、编程式 API 经 `bumpVersion(options, provider)` 注入，bump 完成后接该平台 Release（ADR-0016）。
_Avoid_: 平台包（歧义——兼指 Platform package 二进制分发包）
