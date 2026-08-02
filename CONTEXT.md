# vill-v/bump

版本发布工具集：以 Rust 核心驱动版本号更新、changelog 生成与多平台（GitHub/GitLab/Gitee/GitCode）release 创建。

## Language

**Bump**:
一次完整的版本发布操作——更新各文件中的版本号、生成 changelog、git commit/tag/push、执行 npm scripts。
_Avoid_: release（指平台 release 创建）、publish

**Release type**:
决定新版本号如何计算的选择项：`major` / `minor` / `patch` / `next` / `conventional` / `premajor` / `preminor` / `prepatch` / `prerelease`，以及 prompt 中的 `none`（保持不变）和 `custom`（手动输入）。
_Avoid_: bump type、version type

**Preid**:
预发行标识符（如 `1.0.0-beta.1` 中的 `beta`）。计算候选版本时沿用当前版本的预发行标识，否则用入参（上游 normalizeOptions 缺省为 `'beta'`）；预发行号从 1 开始（上游的 `0→1` 修正）。

**Scripts**:
bump 流程三时序槽位（`preversion` / `version` / `postversion`）的通用 shell 命令，声明于 `.vbumpprc.json` 的 `scripts` 字段（ADR-0011）。
_Avoid_: npm scripts（上游旧义——从 package.json scripts 读取经 `npm run`，通道已移除）

**配置文件 (`.vbumpprc.json`)**:
工具单一配置文件，JSON-only，加载全权归 Rust（overrides > 文件 > 内建默认，ADR-0013）：bumpp 键居顶层，`changelog` 段与 `scripts` 字段并列；全项目仅这一条解析路径，解析结果不向 JS 导出。loader 只认此文件名（及 `configFilePath` override），旧名不探测、静默失效。
_Avoid_: bump.config.json（旧名）、vbumpp.config（esconf 旧制，已移除）

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
纯 Rust + napi-rs 实现的版本与 changelog 引擎包 `@vill-v/bumpp-core`（`napi/bumpp-core`）。对外 API 两组：与上游 bumpp v11 兼容的 `versionBump` / `versionBumpInfo` / `loadBumpConfig`；changelog 系 `generateChangelog` / `getLastGitTag` / `getCurrentGitBranch` / `resolveRepoConfig`（changelogen 使用面的 Rust 重写，ADR-0012）。进度为 Rust 内置打印（ADR-0002），`ProgressEvent` 不向 Node 层导出。
_Avoid_: bumpp（指上游 antfu/bumpp 依赖本身）、next（实验包，已由 core 替代并删除）、changelogen（上游 unjs 依赖本身，使用面已重写并移除）

**Platform package**:
按目标平台分发预编译 `.node` 二进制的 npm 包（如 `@vill-v/bumpp-core-darwin-arm64`，`napi/bumpp-core-darwin-arm64`），作为主包的 optionalDependencies 安装。
_Avoid_: native package、binary package
