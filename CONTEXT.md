# vill-v/bump

版本发布工具集：以 Rust 核心驱动版本号更新、changelog 生成与多平台（GitHub/GitLab/Gitee/GitCode）release 创建。

## Language

**Bump**:
一次完整的版本发布操作——更新各文件中的版本号、生成 changelog、git commit/tag/push、执行 npm scripts。
_Avoid_: release（指平台 release 创建）、publish

**CLI**:
`vbumpp` 命令行——argv 语法（子命令、flag、help 文案、错误提示、退出码）唯一归属 Rust（Core 内的 cli 模块，手写解析器，ADR-0016）。npm bin 与原生 CLI 二进制（`crates/vbumpp`，纯 Rust、零 napi 依赖）共享同一 `run_from_argv`，作为两个薄壳前端；Node 侧零依赖、零逻辑。provider 身份可经 `--provider` flag 在 argv 层给出（优先于平台变体包的注入），`release` 子命令提供失败后的独立重试通路。
_Avoid_: cac 路由（不属于当前 CLI 架构）

**Release type**:
决定新版本号如何计算的选择项：`major` / `minor` / `patch` / `next` / `conventional` / `premajor` / `preminor` / `prepatch` / `prerelease`，以及 prompt 中的 `none`（保持不变）和 `custom`（手动输入）。
_Avoid_: bump type、version type

**Preid**:
预发行标识符（如 `1.0.0-beta.1` 中的 `beta`）。计算候选版本时沿用当前版本的预发行标识，否则用入参（上游 normalizeOptions 缺省为 `'beta'`）；预发行号从 1 开始（上游的 `0→1` 修正）。

**Scripts**:
bump 流程三时序槽位（`preversion` / `version` / `postversion`）的通用 shell 命令，声明于配置文件的 `scripts` 字段（ADR-0011）。是 vbumpp 用户侧概念，与仓库脚本（本仓维护用脚本）无关。
_Avoid_: npm scripts（上游旧义——从 package.json scripts 读取经 `npm run`，通道已移除）

**配置文件**:
工具配置的两级文件——项目级 `.vbumpprc.{json,jsonc,toml}` 与全局级 `~/.vbumpp/config.{json,jsonc,toml}`（ADR-0013；`.jsonc` 为 `.json` 的别名，同走 JSONC 解析，注释与尾逗号均可用）。加载全权归 Rust，四层合并：overrides > 项目 > 全局 > 内建默认；bumpp 键居顶层，`changelog` 段、`scripts` 字段、`gitlab` 段并列；全项目仅这一条解析路径，解析结果不向 JS 导出。同级探测到多个配置文件即报错并全部列出；旧名不探测、静默失效。
_Avoid_: bump.config.json（旧名）、vbumpp.config（esconf 旧制，已移除）、YAML 配置（不支持，ADR-0013）

**全局配置目录 (`~/.vbumpp/`)**:
用户级数据的家——全局配置文件与 Token 存储同放此目录。`VBUMPP_HOME` 覆盖整个目录；`VBUMPP_TOKEN_STORE` 仅覆盖 token 存储文件路径（兼容保留，优先级高于 `VBUMPP_HOME`）。
_Avoid_: XDG 目录（不引入）

**Token 存储 (Token store)**:
平台 access token 的加密凭证存储（`tokens.bin` + 同目录 `key.bin`），VBTK v1 二进制格式（magic + version + iv + authTag + AES-256-GCM 密文），Rust 全权管理且与 JS 时代逐字节兼容（ADR-0014）。内部 JSON map 的键分两级：provider 级键（如 `gitlab`）与 host 作用域键（provider 无关格式 `provider@host`，如 `gitlab@https://gitlab-a.com`——host 为规范化 base URL：无 scheme 补 `https://`（显式 `http://` 保留）、scheme/host 小写、去尾斜杠、保留端口与路径）；release 时 host 作用域键优先、provider 级键回落（向后兼容硬要求）。录入与管理经 `vbumpp token set / list / remove --host`（目前仅 gitlab 开放——其他 provider 无 host 配置通路，未来 GHE 支持时解除）；remove 为交互矩阵：四目标形态（provider 精确 / `--host` 精确 / provider `--all` / 全量 `--all`）× 执行修饰（`--dry-run` 只列清单优先、`--yes` 跳过确认、默认 No 二次确认、非 TTY 报错引导 `--yes`）。防护级别为「防明文落盘」（非高安全保险柜）；明文 token 不跨 napi 边界进入 JS。
_Avoid_: 凭证库（语义过强）

**平台 Release**:
向 git 托管平台（github / gitlab / gitee / gitcode）创建 release 的动作，Rust 内按 provider 适配（gitee / gitcode 复用 github-like 实现；gitlab 多一步项目 id 直查，自建实例经 `gitlab` 段的 `host` 配置，ADR-0014）。两通路：bump 流程末段自动创建（body 为当次生成的 changelog markdown）；`vbumpp release <version>` 独立重试（body 从 changelog 文件提取指定版本节，应对网络失败 / 密钥过期后的补发，ADR-0016）。token 解析链统一为：Token 存储（gitlab 先 host 作用域键精确匹配、再 provider 级键回落）→ 各家环境变量（`GH_TOKEN` → `GITHUB_TOKEN` / `GITLAB_TOKEN` / `GITEE_TOKEN` / `GITCODE_TOKEN`）→ 仅 github 追加 `gh auth token` 兜底。
_Avoid_: publish

**Dry run**:
`--dry-run` 预览模式（bump 与 release 子命令共用；token remove 亦提供同语义预览——只列将删清单，不确认、不删除）——走完真实执行的全部只读计算与前置校验（校验失败照常报错 exit 1，可当 CI 预检门禁），拦截全部副作用（文件写盘、git commit/tag/push、scripts 与 install/execute、平台 release HTTP），改为逐行打印执行计划：glob 命中文件的预演判定（update → x.y.z / up-to-date / missing）、当前版本及其来源、新版本、将写盘文件、将执行的脚本与命令文本、格式化后的 commit message 与 tag 名、push 序列、平台 release 的目标与 body。版本选择交互保留（不定版本则无计划可预览），"Bump?" 确认跳过（零写盘无需二次确认）；changelog 生成全文预览、所见即所得；token 走解析链并报告来源（store / env / gh），缺失只警告不报错。
_Avoid_: 试运行、pretend

**上架 (Registry publish)**:
向 registry（npm / crates.io）上传包的发布动作——本仓 13 个 npm 包（ADR-0025 增 musl 平台包 ×2）与 2 个 crate（`vbumpp-core`、`vbumpp`）经 `ci.yml` 的 publish jobs 完成：tag 触发、无人工门、skip-if-published 幂等（ADR-0021）。与 Bump（含 commit/tag/push 的完整发布操作）、平台 Release（git 托管平台 release）三者分立。
_Avoid_: publish（英文对应词，不作术语）、发布（过宽——兼指 Bump 与平台 Release）

**首发仪式 (First-publish ceremony)**:
全新 npm 包名的首次上架流程——npm OIDC trusted publishing 要求包已存在且已配置 trusted publisher，新包名没有配置页，首发必走一次性经典认证：本地 `pnpm login`（OTP）手动首发 → npmjs.com 包设置配 trusted publisher → 重跑 CI 由 OIDC 收后续版本（ADR-0021 决策④、ADR-0029）。平台矩阵每扩一个新 target 即新增一个包名，首发仪式是该次发版的前置条件；漏做则 publish-npm 在拓扑序中段 404，造成部分上架（v6.1.0 实例）。触发绑定已由 npm-publish.ts 前置检测闭环：发布计划含从未上架的包名时 CI 在上传任何包之前拦停并输出仪式指引，本地手动首发只警告不拦。
_Avoid_: 首发（过宽——需含完整流程义）

**免编译安装 (cargo-binstall 渠道)**:
用户侧安装通路之一（与 npm 并列的一等渠道，ADR-0025）——`cargo binstall vbumpp` 依据 crates.io 元数据的 `repository` 字段探测 GitHub Release，拉取预编译 CLI 二进制免编译安装；无匹配平台的用户自动回退 `cargo install` 源码编译。产物约定跟随 binstall 默认模板（`vbumpp-{target}.tar.gz`、顶层目录 `vbumpp-{target}/`、tag `v{version}`），故 `crates/vbumpp` 的 Cargo.toml 零 binstall 元数据；覆盖 7 target（napi 5 平台 + linux musl×2），每产物附 `.sha256` 供用户手动校验（binstall 不消费）。与上架（向 registry 上传包）分立：本渠道消费的产物由 tag CI 的 build-cli job 构建、`gh release upload --clobber` 追加到 bump 流程已建的平台 Release，产物缺失令流水线硬失败。
_Avoid_: 发布到 binstall（binstall 无 registry，纯按约定探测 GitHub Release）、预编译 npm 包（指 Platform package，机制不同）

**内部机制包 (Internal machinery package)**:
虽然发布到 npm、但设计上就不是给用户直接使用的包——判别问句："用户会直接 npm install 它吗？"不会即归 `napi/` 目录（ADR-0005，与是否发布无关）。成员：Core（napi 绑定本体）、Platform package（平台二进制分发包）；它们发布仅因 npm 不支持 workspace 协议与 optionalDependencies 按平台分发的机制需要。
_Avoid_: 内部包（过宽，`crates/` 亦属内部）、native package

**生态 (Ecosystem)**:
一套工具链及其版本文件与安装机制的集合（node / cargo；未来 maven、gradle）。vbumpp-core 的生态知识集中于插件底座 `src/plugins/`（ADR-0007）：版本解析与更新、按本次 bump 实际更新的生态条件触发的 install 适配、recursive 整树收集三能力子目录，各生态实现落同名文件。
_Avoid_: platform（指 OS/CPU 平台）、registry（指发布平台）

**插件底座 (Plugin base)**:
`src/plugins/`——全部生态能力的单一归属。trait + 静态链 + 编排在 `mod.rs`；插件类型在根部（`plugins/node.rs` 等），方法委托至 `version/` / `install/` / `recursive/` 三能力子目录的生态同名纯函数文件（Rust coherence：单 trait 的 impl 不可拆文件，ADR-0007）。
_Avoid_: files 插件链（ADR-0007 时代的旧称）

**清单 (Manifest)**:
生态认识的结构化版本文件（node：package.json 等 8 种 basename；cargo：Cargo.toml），以 basename 识别并走对应生态通道结构化更新、读取当前版本。其 basename 集合的单一事实源为插件底座链：默认清单（根级并集）与 recursive 模式表（`**/` 并集）均由链聚合（ADR-0007）。recursive 收集不按 `"private": true` 过滤——private 仅表示不上架，private 包版本随整树一并锁步（ADR-0030）；过滤层仅内置目录排除与 gitignore 感知。
_Avoid_: 版本文件（过宽——还含 text 通道的任意文本文件，如 README、.env）

**包管理器检测信号 (Package-manager detection signal)**:
JavaScript 生态中用于选择 install 命令的文件或声明，包括 lockfile、workspace 文件与包管理器声明（ADR-0006）；检测结果仅是包管理器名称，不承载版本选择或执行策略。
_Avoid_: Corepack 配置（过窄——信号也包括文件且支持范围宽于 Corepack）、包管理器扫描（易与清单扫描混淆）

**包管理器声明 (Package-manager declaration)**:
`package.json` 中提供包管理器名称的结构化声明，包括顶层 `packageManager` 与 `devEngines.packageManager`（ADR-0006）；后者仅贡献 `name`，其 `version` 与 `onFail` 不属于 vbumpp 的 install 调度语义。
_Avoid_: Corepack 声明（这些字段可由其他工具消费）、包管理器配置（暗示 vbumpp 执行完整配置语义）

**Core**:
纯 Rust + napi-rs 实现的版本、changelog 与平台 Release 引擎包 `@vill-v/bumpp-core`（`napi/bumpp-core`）。napi 面由 ADR-0014 与 ADR-0016 确定：编排 `bumpVersion(options, provider?)`、CLI 单入口 `cliRun(argv, provider?)`；独立 release 由 CLI `vbumpp release` 子命令承接；上游 parity 面（`versionBump` 系、`loadBumpConfig`）与 changelog 系函数收归 Rust 内部，`@vill-v/bumpp/changelog` 子路径移除。进度为 Rust 内置打印（ADR-0002），`ProgressEvent` 不向 Node 层导出。
_Avoid_: bumpp（指上游 antfu/bumpp 依赖本身）、next（实验包，已由 core 替代并删除）、changelogen（上游 unjs 依赖本身，使用面已重写并移除）

**Platform package**:
按目标平台分发预编译 `.node` 二进制的 npm 包（如 `@vill-v/bumpp-core-darwin-arm64`），作为主包的 optionalDependencies 安装。目录（`napi/<triple>`，如 `napi/darwin-arm64`）不提交进 git——CI 与本地均经 `pnpm create:npm-dirs`（包装 `napi create-npm-dirs`）从 `napi.targets` 现场生成；fresh clone 目录缺失时 pnpm 对 optionalDependencies 里的 `workspace:*` silent skip，loader 走本地 `.node` fallback（ADR-0029）。主包发布态不内置任何 `.node`——平台包是二进制唯一分发通道，包根 fallback 仅存于本地开发磁盘（ADR-0032）。
_Avoid_: native package、binary package

**napi.targets**:
`napi/bumpp-core/package.json` 的 `napi.targets` 字段（7 条 rust triple）——"支持哪些平台"的单一真相源（ADR-0029）。真相源链：`napi.targets` → create-npm-dirs 生成平台目录 → optionalDependencies（`workspace:*`），链在此处终止；loader 为 napi-rs 官方生成物（零手写，ADR-0033），其静态平台 require 集由 napi CLI 构建时从同一字段生成，无第二份手写平台清单。
_Avoid_: 支持矩阵表（暗示存在独立维护的清单）

**loader**:
`napi/bumpp-core/index.js` 的通称——Core 的 JS 绑定入口，按当前平台分派加载 `.node`（本地构建产物优先、平台包兜底，`NAPI_RS_NATIVE_LIBRARY_PATH` 为第一覆盖分支）。为 `napi build --platform --esm` 的官方生成物，零手写代码（ADR-0033）：不提交进 git，经 CI 构建腿产物捎带、publish 时归位 core；版本强校验、平台清单报错等手写增强均已删除，加载失败为生成物的 npm#4828 标准文案。
_Avoid_: 自研 loader（曾存在，触发 Socket.dev 供应链告警，已删除）

**平台变体包 (Platform variant)**:
面向用户的 npm 包 `@vill-v/bumpp-{github,gitlab,gitee,gitcode}`——与主包同形，差别仅在 provider 身份注入：bin 经 `cliRun(argv, provider)` 位置参数、编程式 API 经 `bumpVersion(options, provider)` 注入，bump 完成后接该平台 Release（ADR-0016）。
_Avoid_: 平台包（歧义——兼指 Platform package 二进制分发包）

**用户可见字符串 (User-facing string)**:
包向终端与调用方暴露的全部文案——错误信息、CLI help/用法、交互 prompt、进度打印、panic 兜底、napi loader 平台报错。唯一语言为英文（ADR-0017）；非英文需求一律走配置定制（如本仓库 `.vbumpprc.toml` 的中文 changelog types 标题），不进代码内建。代码注释不在其列（仓库内部工作语言为中文）。
_Avoid_: 控制台打印（过窄——错误信息不经打印通路亦属之）、界面文案

**显示路径 (Display path)**:
打印到控制台的路径的统一形态——cwd 之内打相对路径，cwd 之外（token 存储、全局配置、`..` 逃逸的显式参数）打绝对路径，一律 POSIX 分隔符（ADR-0002）。只约束显示层；存储与 API 返回值（`updatedFiles` 等）保持绝对原生路径不变。
_Avoid_: 完整路径、绝对路径打印

**演示时间线 (Demo cast)**:
演示网站首页各子命令卖点的终端会话数据——asciicast v2 兼容的帧时间线,由 capture 脚本在固定日期/hash 的 git fixture 上以只读形态(dry-run / list)确定性采集真实 CLI 输出生成,提交进 website 代码,CI 漂移校验腿(ci.yml demo-drift:重跑采集 diff 提交产物,不一致即红)防腐(ADR-0036);首页滚动演示区与移动端/减少动态效果的静态降级共用同一份数据。
_Avoid_: 录屏(指视频/GIF 形态)、模拟演示(手写脚本,与真实输出脱钩)

**文档网站 (Docs website)**:
面向用户的产品文档站（`website/`，fumadocs / Next.js 静态导出，ADR-0020）——与 `docs/` 的工程内部文档（ADR、agent 约定、迁移指南源稿）物理分离。纯中文、单版本（随最新 release）；内容板块：快速上手、CLI 参考、配置文件参考、平台 Release 指南、v5→v6 迁移指南、外链区（导航栏图标链接：npmx.dev 包页 + GitHub Releases）。部署 GitHub Pages（项目页子路径 `/bumpp`），接受国内访问不稳定的取舍、不做国内镜像。
_Avoid_: docs（指 `docs/` 工程内部文档目录）

**仓库脚本 (Repo scripts)**:
本仓维护用的命令行脚本——`scripts/`、`website/scripts/`、napi 冒烟与 crates fixture 生成器等，一律 TypeScript 由 node 原生直跑（type stripping，node 版本由 mise 的 lts 保证下限），不经编译步骤、不引入转译器。脚本为 ESM 形态，所在包一律声明 `"type": "module"`（根、`napi/bumpp-core`、`npm/*`、website 清一色）——node 直跑 `.ts` 不经 CJS 解析失败再重试，无 `MODULE_TYPELESS_PACKAGE_JSON` 警告。例外两类：发布给用户执行的产物（`npm/*/bin/` 薄壳，用户 node 环境不可控）与工具生成物（napi loader）。
_Avoid_: Scripts（指 `.vbumpprc` 的 `scripts` 字段，用户侧概念）、构建脚本（歧义——兼指 Rust build.rs 与包安装期脚本）
