# Changelog

## v6.1.1

[compare changes](https://github.com/vill-v-kit/bumpp/compare/v6.1.0...v6.1.1)

### 🩹 修复

- **napi:** Core 发布包剔除内置 .node——平台包为二进制唯一分发通道 ([9d54c53](https://github.com/vill-v-kit/bumpp/commit/9d54c53))

### 📖 文档

- 发版质量门前移决策与新 npm 包名首发仪式术语——v6.1.0 两起发版事故复盘 ([5099df8](https://github.com/vill-v-kit/bumpp/commit/5099df8))
- 记录 core 发布包不内置 .node——optionalDependencies 为二进制唯一分发通道 ([e2544e4](https://github.com/vill-v-kit/bumpp/commit/e2544e4))

### 🏡 框架

- Rust 钉版 1.97.1 + hk git hook 质量门——pre-commit 跑 fmt check、pre-push 跑 clippy,prepare 自动装配 hook;v6.1.0 发版 fmt 漏检事故的前移防线 ([1184a32](https://github.com/vill-v-kit/bumpp/commit/1184a32))
- Prepare 瘦身至裸 hk install——环境引导责任移入贡献指南,失败即提示先 mise install ([e66a8aa](https://github.com/vill-v-kit/bumpp/commit/e66a8aa))
- Gitignore 增 .artifacts——首发仪式下载 CI 产物的暂存目录不再污染 git status ([931bb71](https://github.com/vill-v-kit/bumpp/commit/931bb71))

### ❤️ Contributors

- Whitekite

## v6.1.0

[compare changes](https://github.com/vill-v-kit/bumpp/compare/v6.0.0...v6.1.0)

### 🚀 特性

- **scripts:** 发版自检 verify-tag-ci——tag push 后轮询 Actions runs，GitHub 丢事件当场告警（COL-62） ([3558355](https://github.com/vill-v-kit/bumpp/commit/3558355))
- **core:** 控制台显示路径统一——cwd 内相对、cwd 外绝对、一律 POSIX（ADR-0023） ([8ddf26c](https://github.com/vill-v-kit/bumpp/commit/8ddf26c))
- **core:** Ecosystem::Node → Ecosystem::JavaScript 术语更正 ([2f9d093](https://github.com/vill-v-kit/bumpp/commit/2f9d093))
- **ci:** 文档站部署前后冒烟验证——产物 basePath 断言 + 线上关键资源轮询 ([9112d09](https://github.com/vill-v-kit/bumpp/commit/9112d09))
- **napi:** 平台包目录切 napi.targets 驱动的生成流，平台矩阵扩 musl ×2 对齐 7 target ([91f8030](https://github.com/vill-v-kit/bumpp/commit/91f8030))
- **core:** Install 检测增列 devEngines.packageManager 包管理器声明——同级第三优先级只消费对象形态 name，无效形态静默回退（COL-76） ([70ed77d](https://github.com/vill-v-kit/bumpp/commit/70ed77d))

### 🩹 修复

- **core:** 配置 release 键接通 bump 编排 + 顶层键名白名单校验（COL-60） ([5fcc188](https://github.com/vill-v-kit/bumpp/commit/5fcc188))
- **core:** -r 收集感知 gitignore + commit 过滤未跟踪 pathspec（COL-61，v6.0.0 发版中断根因） ([b418f56](https://github.com/vill-v-kit/bumpp/commit/b418f56))
- **website:** 静态搜索客户端显式携带 basePath 抓取索引 ([c2ca697](https://github.com/vill-v-kit/bumpp/commit/c2ca697))

### 📖 文档

- **adr:** ADR-0022 收集器迁 ignore crate 暂缓记录——剪枝性能/非 git 目录两优势与重启信号存档，防未来大仓库性能问题重新调研 ([e353df7](https://github.com/vill-v-kit/bumpp/commit/e353df7))
- **adr:** ADR-0027 补记良性 linker 警告的维护者裁定（COL-63 验收闭环） ([7ea25b7](https://github.com/vill-v-kit/bumpp/commit/7ea25b7))
- **adr:** ADR-0025 cargo-binstall 免编译安装渠道——GitHub Release 挂预编译 CLI ([287faae](https://github.com/vill-v-kit/bumpp/commit/287faae))
- **adr:** ADR-0026 napi 平台矩阵扩 musl ×2——修 Alpine npm 安装硬失败 ([b8ea1a6](https://github.com/vill-v-kit/bumpp/commit/b8ea1a6))
- **website:** 删除首页「构建」区 ([73d0a52](https://github.com/vill-v-kit/bumpp/commit/73d0a52))
- **website:** 新增「生态集成」文档（JavaScript / Cargo） ([d19f762](https://github.com/vill-v-kit/bumpp/commit/d19f762))
- **website:** 清理程序员常识与设计意图文案 ([f0c76d8](https://github.com/vill-v-kit/bumpp/commit/f0c76d8))
- Consolidate current architecture decisions ([607ed24](https://github.com/vill-v-kit/bumpp/commit/607ed24))
- 明确 npm 渠道不支持 darwin-x64，安装文档引导 Intel Mac 用户走 cargo 渠道 ([e32af9c](https://github.com/vill-v-kit/bumpp/commit/e32af9c))
- Darwin-x64 决策的替代路径表述对齐免编译安装（cargo-binstall 渠道）术语 ([fc80898](https://github.com/vill-v-kit/bumpp/commit/fc80898))
- 裁定 private 包随整树收集锁步为既定行为——版本侧不按 private 过滤，收集器不新增过滤分支 ([9011a11](https://github.com/vill-v-kit/bumpp/commit/9011a11))
- Vbumpp crate 补 README——crates.io 页面并列 binstall 免编译与 cargo install 通路，注明 7 target 覆盖与回退语义 ([cc55c49](https://github.com/vill-v-kit/bumpp/commit/cc55c49))
- 网站安装文档增列 cargo-binstall 免编译渠道——快速上手安装段与 crates.io 页并列 npm/binstall/cargo install 通路 ([23560da](https://github.com/vill-v-kit/bumpp/commit/23560da))
- CONTEXT 术语对齐平台包生成流——增 napi.targets 单一真相源条目，平台包目录改写为 create-npm-dirs 生成不提交（COL-75） ([55692f7](https://github.com/vill-v-kit/bumpp/commit/55692f7))

### 🏡 框架

- **meta:** 发布元数据批修——repository.url 补 .git ×11 + 平台包 type:commonjs ×5 + homepage 统一指向 GitHub Pages（npm 11 + crates 2） ([ae2ea3d](https://github.com/vill-v-kit/bumpp/commit/ae2ea3d))
- **skills:** Tsdown skill 上游同步（rolldown/tsdown） ([2d66235](https://github.com/vill-v-kit/bumpp/commit/2d66235))
- **gitignore:** 忽略 .zcode/plans ([4ab2316](https://github.com/vill-v-kit/bumpp/commit/4ab2316))

### 📦 打包

- **ci:** Linux-arm64 交叉链换 zig 淘汰 gcc 4.8.5，TLS 回 rustls（COL-63，ADR-0027） ([4749db6](https://github.com/vill-v-kit/bumpp/commit/4749db6))
- **mise:** Zig/cargo-zigbuild 加 os=["linux"] 过滤——macOS/Windows 腿与本地免装 ([d5bfc03](https://github.com/vill-v-kit/bumpp/commit/d5bfc03))

### ❤️ Contributors

- Whitekite

## v6.0.0

[compare changes](https://github.com/vill-v-kit/bumpp/compare/v5.1.0...v6.0.0)

> **⚠️ v6 大版本更新要点**
>
> - **纯 Rust 重写**：核心引擎全部 Rust 化——npm 通路走 napi 平台预编译包，另有原生 CLI 单二进制（`cargo install vbumpp`，无需 Node.js），功能完全一致
> - **依赖大幅瘦身**：安装体积 8.6 MB / 59 包 → ≈5 MB / 3 包（npm 通路）；原生 CLI 为 4.6 MB 零依赖单二进制
> - **配置文件统一**：收归 `.vbumpprc.json`（也支持 `.jsonc` / `.toml`）；`bump.config.*`、`vbumpp.config.*`、`changelog.config.*`、package.json 的 `changelog` 键**一律不再读取（静默失效）**
> - **编程式 API 收紧**：`defineConfig` 移除、`@vill-v/bumpp/changelogen` 子路径移除、token/release 操作收归 CLI 子命令
> - **changelog 产出完全离线**：分组标题默认英文（中文需在 `changelog.types` 显式声明）、贡献者行不再请求第三方服务、不再显示邮箱与 @链接
> - **新文档站上线**：含迁移指南、CLI 参考、配置参考
>
> 升级前请阅读 [v5 → v6 迁移指南](https://vill-v-kit.github.io/bumpp/docs/migration-v6) · 文档站：<https://vill-v-kit.github.io/bumpp>

### 🚀 特性

- **core:** 新增 @vill-v/bumpp-core napi 脚手架，接通 mise/turbo 工具链 (COL-6) ([b712faf](https://github.com/vill-v-kit/bumpp/commit/b712faf))
- **core:** LoadBumpConfig 纯 Rust 实现（仅 JSON 配置） (COL-8) ([9fcc92f](https://github.com/vill-v-kit/bumpp/commit/9fcc92f))
- **core:** 候选版本计算纯函数，全量对齐上游 bumpp v11 (COL-9) ([9c97a82](https://github.com/vill-v-kit/bumpp/commit/9c97a82))
- **core:** 文件版本更新（manifest 保格式 + 文本模板替换） (COL-10) ([8e1feb3](https://github.com/vill-v-kit/bumpp/commit/8e1feb3))
- **core:** Git 操作 + npm scripts，全部 shell out (COL-11) ([1f201e0](https://github.com/vill-v-kit/bumpp/commit/1f201e0))
- **core:** 预编译平台包布局 + GitHub Actions 构建/测试矩阵 (COL-12) ([9bace0e](https://github.com/vill-v-kit/bumpp/commit/9bace0e))
- **core:** Conventional 提交解析与版本推断 (COL-13) ([433ae07](https://github.com/vill-v-kit/bumpp/commit/433ae07))
- **core:** VersionBumpInfo 全链路（Rust 渲染 prompt） (COL-14) ([8ee224a](https://github.com/vill-v-kit/bumpp/commit/8ee224a))
- **core:** VersionBump 全链路编排 + ThreadsafeFunction 进度 (COL-15) ([8d0cf51](https://github.com/vill-v-kit/bumpp/commit/8d0cf51))
- **bump:** 切换至 @vill-v/bumpp-core（自举前置） (COL-16) ([baca57b](https://github.com/vill-v-kit/bumpp/commit/baca57b))
- **core:** Progress 内置 Rust，拆除 JS 进度面 (COL-19) ([0364c6e](https://github.com/vill-v-kit/bumpp/commit/0364c6e))
- **core:** Version-files 插件底座 crate，JsManifest/Text 纯迁移 (COL-22) ([8628075](https://github.com/vill-v-kit/bumpp/commit/8628075))
- **core:** Cargo.toml 版本同步——CargoTomlPlugin + Cargo.lock 定向同步 (COL-23) ([e1aa4d6](https://github.com/vill-v-kit/bumpp/commit/e1aa4d6))
- **structure:** Napi/ 受众判别约定 + bumpp-core/平台包原子搬迁 (COL-25) ([9fae9a2](https://github.com/vill-v-kit/bumpp/commit/9fae9a2))
- **core:** Package-manager 检测对齐上游名单 + pm.rs 模块独立 (COL-26) ([4044a31](https://github.com/vill-v-kit/bumpp/commit/4044a31))
- **core:** Install 生态适配文件夹与条件触发 (COL-28) ([752b7fe](https://github.com/vill-v-kit/bumpp/commit/752b7fe))
- **core:** Recursive 生态感知——链上清单模式表 + napi 导出 + CLI -r 重接 (COL-31) ([1c3070a](https://github.com/vill-v-kit/bumpp/commit/1c3070a))
- **core:** 插件底座落地——默认清单/版本来源生态化 + scripts 通用化 (COL-32) ([f347526](https://github.com/vill-v-kit/bumpp/commit/f347526))
- **core:** 配置文件改名 .vbumpprc.json——旧名不探测 + recursive 展开收归加载器 (COL-33) ([0967167](https://github.com/vill-v-kit/bumpp/commit/0967167))
- **core:** Git 只读历史操作 + napi 导出——tag/diff/branch/remote 解析对齐 changelogen (COL-34) ([a55959a](https://github.com/vill-v-kit/bumpp/commit/a55959a))
- **core:** Changelog 配置段解析——内建默认 + types 深合并 + 严格 schema (COL-35) ([804d167](https://github.com/vill-v-kit/bumpp/commit/804d167))
- **core:** Changelog markdown 引擎——展示层解析 + 结构 parity + golden fixtures (COL-36) ([7e1375b](https://github.com/vill-v-kit/bumpp/commit/7e1375b))
- **core:** GenerateChangelog 编排 + napi——changelog.ts 内置与 N1/C1/C2/C3 修复 (COL-37) ([92198da](https://github.com/vill-v-kit/bumpp/commit/92198da))
- **bump:** 薄编排重接——三删一迁 + Config 扁平化 + ./changelog 子路径 + 依赖瘦身 (COL-38) ([ecb21eb](https://github.com/vill-v-kit/bumpp/commit/ecb21eb))
- **release:** 导入改 ./changelog 子路径 + defineConfig 移除 (COL-39) ([c868d8a](https://github.com/vill-v-kit/bumpp/commit/c868d8a))
- **core:** 剩余功能全面 Rust 化 + 全局配置与 JSONC/TOML 多格式（ADR-0014/0015） ([ab3f302](https://github.com/vill-v-kit/bumpp/commit/ab3f302))
- **cli:** CLI 全权归 Rust——cliRun 单入口与五包收缩（ADR-0016，COL-41/42/43） ([169f6b7](https://github.com/vill-v-kit/bumpp/commit/169f6b7))
- **core:** 用户可见字符串统一英文，changelog types 中文定制移项目级配置（ADR-0017） ([69458a1](https://github.com/vill-v-kit/bumpp/commit/69458a1))
- **cli:** 纯 Rust CLI + vbumpp release 重试子命令 + napi 面三收缩（ADR-0019，COL-48） ([e014b7d](https://github.com/vill-v-kit/bumpp/commit/e014b7d))
- **docs:** 文档网站——fumadocs 落地 website/ + GitHub Pages 部署（ADR-0020，COL-49） ([d9f1b5e](https://github.com/vill-v-kit/bumpp/commit/d9f1b5e))
- **website:** 品牌 logo——#ff6736 水波圆徽 + 导航/favicon 接线（COL-49） ([13e6920](https://github.com/vill-v-kit/bumpp/commit/13e6920))
- **website:** 首页终端演示素材捕获管道——pty 实跑 vbumpp 交互发版 + ANSI 帧塌缩 + 钉日期确定性复跑（COL-57） ([8d259d3](https://github.com/vill-v-kit/bumpp/commit/8d259d3))
- **website:** 首页 hero 增强——v6 迁移 pill + 标题放大 5xl/6xl + 品牌橙强调「一条命令」（COL-58） ([d45400b](https://github.com/vill-v-kit/bumpp/commit/d45400b))
- **website:** 首页嵌入 macOS 风终端窗口——红绿灯标题栏 + COL-57 实捕发版输出，纯 HTML/CSS 零新依赖（COL-59） ([e6f7d92](https://github.com/vill-v-kit/bumpp/commit/e6f7d92))
- **crates:** Vbumpp-core/vbumpp 上架开闸——移除 publish=false，workspace dep 补宽松版本区间 >=5.1,<7（^6 在 5.x 现世不可解析，ADR-0021 同步修订）（COL-50） ([f922dd2](https://github.com/vill-v-kit/bumpp/commit/f922dd2))
- **ci:** 上架幂等守卫 publish-guard——npm/crates.io 双查 + 放行/跳过/失败三态退出码（COL-51） ([865d265](https://github.com/vill-v-kit/bumpp/commit/865d265))
- **ci:** Publish-npm job——artifact 注入 + 前置校验 + 守卫过滤上架（COL-53） ([817e5f2](https://github.com/vill-v-kit/bumpp/commit/817e5f2))
- **ci:** Publish-crates job——dry-run 前置 + core→cli 顺序上架（COL-54） ([49d377c](https://github.com/vill-v-kit/bumpp/commit/49d377c))

### 🩹 修复

- **core:** Prompt 选项去内嵌 bold——修复活动行 ANSI 裸显 (COL-30) ([b793eae](https://github.com/vill-v-kit/bumpp/commit/b793eae))
- **core:** Release 报错脱敏——明文 token 不出错误消息（ADR-0014 注记，COL-47） ([a06089a](https://github.com/vill-v-kit/bumpp/commit/a06089a))
- **website:** Llms 系生成物 URL 补全 basePath——llms.txt/llms-full.txt/content.md 输出绝对地址，修复子路径部署下全量 404 ([2dff9e0](https://github.com/vill-v-kit/bumpp/commit/2dff9e0))
- **website:** 演示素材 push 行洗白——fixture remote 改绝对路径，输出 To ~/my-project.git 取代 ../remote.git 内部痕迹（code-review COL-57） ([4f8178f](https://github.com/vill-v-kit/bumpp/commit/4f8178f))

### 💅 重构

- **core:** 迁移 packages/core → npm/bumpp-core，确立 crates/napi/npm 分层约定 ([386e292](https://github.com/vill-v-kit/bumpp/commit/386e292))
- 废弃 packages/，全部可发版包归入 npm/ ([fa6e365](https://github.com/vill-v-kit/bumpp/commit/fa6e365))
- **core:** Version-files 合并回 bumpp-core，生态插件文件夹化 (COL-27) ([bc0688a](https://github.com/vill-v-kit/bumpp/commit/bc0688a))
- **core:** Clippy 历史警告清理——manual_contains + 测试模块位置 ([37a5056](https://github.com/vill-v-kit/bumpp/commit/37a5056))
- **core:** Release 按 provider 单文件 + 共用原语抽 http.rs（ADR-0018，COL-46） ([1ae4340](https://github.com/vill-v-kit/bumpp/commit/1ae4340))
- **crates:** Crates.io 家族更名 vbumpp-*——bumpp-core→vbumpp-core、bumpp-cli→vbumpp 落地 ADR-0021（2026-08 修订），CONTEXT 增上架术语、website 安装文档同步 ([948dadb](https://github.com/vill-v-kit/bumpp/commit/948dadb))
- **scripts:** Code-review 收尾——GUARD_UA 更名 + relay 折叠 + shebang 统一 + ADR-0021 决策②补 index.d.ts 括注 ([4811789](https://github.com/vill-v-kit/bumpp/commit/4811789))

### 📖 文档

- 补充工程技能约定与 rust napi 重写 ADR ([676d8b9](https://github.com/vill-v-kit/bumpp/commit/676d8b9))
- 架构/平台矩阵文档与贡献指南 (COL-17) ([db8a7b2](https://github.com/vill-v-kit/bumpp/commit/db8a7b2))
- ADR-0002 progress 内置 + ADR-0003 Cargo.toml 版本同步决策 ([be10bd4](https://github.com/vill-v-kit/bumpp/commit/be10bd4))
- ADR-0004 version-files 插件底座 crate ([184d1fc](https://github.com/vill-v-kit/bumpp/commit/184d1fc))
- **compliance:** 上游开源标识三层闭合——LICENSE 版权行 + README 致谢 + 发包携带 (COL-24) ([aabe009](https://github.com/vill-v-kit/bumpp/commit/aabe009))
- **adr:** 矛盾决策记录精简——0007 权威化、0004 删除、0005 去标签 (COL-29) ([d8658a6](https://github.com/vill-v-kit/bumpp/commit/d8658a6))
- **adr:** Changelogen 使用面重写与配置文件统一决策记录——ADR-0012/0013 + 词汇表 (COL-32) ([bd66f2b](https://github.com/vill-v-kit/bumpp/commit/bd66f2b))
- V6 迁移指南 + napi 全链路冒烟脚本 (COL-40) ([2794e8a](https://github.com/vill-v-kit/bumpp/commit/2794e8a))
- ADR-0016（CLI 全权归 Rust）+ CONTEXT 词条 + ADR-0014 例外标注 ([9e994aa](https://github.com/vill-v-kit/bumpp/commit/9e994aa))
- **adr:** 0018 补注记——napi 四导出已由 ADR-0019 移除（COL-48） ([45e02b1](https://github.com/vill-v-kit/bumpp/commit/45e02b1))
- **adr:** 0019 补注记——crates.io 分发已决策随 v6 上架 bumpp-cli ([bd09706](https://github.com/vill-v-kit/bumpp/commit/bd09706))
- **website:** Crates-io 安装页移除未上架 callout——页面随 v6 发版部署，届时 crate 已上架（ADR-0019 注记） ([b87a003](https://github.com/vill-v-kit/bumpp/commit/b87a003))
- **website:** Cli 参考首语法块替换为完整 --help 输出——原样捕获自 vbumpp 二进制 ([db02a83](https://github.com/vill-v-kit/bumpp/commit/db02a83))
- **website:** 迁移指南依赖瘦身节补实测体积数据——v5 8.6MB/59包 → napi ≈5MB/3包 → 原生 CLI 4.6MB 单二进制 ([0cb8791](https://github.com/vill-v-kit/bumpp/commit/0cb8791))
- **contributing:** 发布流程节改写为实际上架链路 + ADR-0021 双向互链（COL-55） ([de87aee](https://github.com/vill-v-kit/bumpp/commit/de87aee))

### 🏡 框架

- 删除 packages/next 实验包 (COL-7) ([04c6535](https://github.com/vill-v-kit/bumpp/commit/04c6535))
- 包声明邮箱统一替换为 xuxjigsaw@qq.com ([65f60b1](https://github.com/vill-v-kit/bumpp/commit/65f60b1))
- CHANGELOG 历史记录邮箱同步替换为 xuxjigsaw@qq.com ([87ee599](https://github.com/vill-v-kit/bumpp/commit/87ee599))
- 升级开发依赖与 pnpm 版本 ([3b8f059](https://github.com/vill-v-kit/bumpp/commit/3b8f059))
- 发版 files 显式配置移除——默认清单已覆盖根 Cargo.toml（COL-44） ([8dee7f1](https://github.com/vill-v-kit/bumpp/commit/8dee7f1))

### 📦 打包

- Xcode 27 链接器 Mach-O 串池错位规避——darwin 声明 SDK 26.0 ([c8f05c8](https://github.com/vill-v-kit/bumpp/commit/c8f05c8))
- Cargo 成员引用与版本号收编根 workspace 统一管理（ADR-0003 修订，COL-44） ([8e851ae](https://github.com/vill-v-kit/bumpp/commit/8e851ae))

### ✅ 测试

- **core:** Loader 负例改到干净子进程执行（隔离 CI 环境 NODE_PATH 干扰） ([bcaab17](https://github.com/vill-v-kit/bumpp/commit/bcaab17))
- **core:** Update-files parity 路径列表改集合比较（上游 glob 排序跨平台不稳定） ([204647b](https://github.com/vill-v-kit/bumpp/commit/204647b))
- **napi:** LoadBumpConfig 测试对齐 .vbumpprc.json 与旧名静默 (COL-33) ([700ad35](https://github.com/vill-v-kit/bumpp/commit/700ad35))

### 🎨 样式

- **core:** Code-review 清理——imports 置顶、注释对齐现状 (COL-19) ([325943e](https://github.com/vill-v-kit/bumpp/commit/325943e))
- **test:** Tests/git.rs rustfmt 补齐（COL-34 漏格式化） ([ea17503](https://github.com/vill-v-kit/bumpp/commit/ea17503))
- **test:** Tests/config.rs rustfmt 补齐（既有漂移，随本次 fmt --all 顺带） ([a5c8c57](https://github.com/vill-v-kit/bumpp/commit/a5c8c57))
- 在制文件 rustfmt 统一——零语义变化（COL-46/047 后续） ([8b3f3dd](https://github.com/vill-v-kit/bumpp/commit/8b3f3dd))
- Release 模块声明字母序重排——零语义变化 ([dbeabb8](https://github.com/vill-v-kit/bumpp/commit/dbeabb8))

### ❤️ Contributors

- Whitekite

## v5.1.0

[compare changes](https://gitee.com/vill-v/bump/compare/v5.0.1...v5.1.0)

## v5.0.1

[compare changes](https://gitee.com/vill-v/bump/compare/v5.0.0...v5.0.1)

### 💅 重构

- **bump:** ⚠️  Token 存储密钥改为随机生成并落盘保存，不再由机器信息派生 (e8dfc11)

### 🏡 框架

- **lint:** 移除 oxlint 配置中未使用的规则与配置项 (fc19509)

#### 🚨 破坏性改动

- **bump:** ⚠️  Token 存储密钥改为随机生成并落盘保存，不再由机器信息派生 (e8dfc11)

### ❤️ 贡献者

- Whitekite <xuxjigsaw@qq.com>

## v5.0.0

[compare changes](https://gitee.com/vill-v/bump/compare/v4.2.0...v5.0.0)

### 💅 重构

- **bump:** ⚠️  AccessToken 修改为非明文储存，并增加对应 cli `vbumpp token set <provider>` 进行 accessToken 设置 (99ecab9)

### 🏡 框架

- Update CHANGELOG.md (b88686d)

#### 🚨 破坏性改动

- **bump:** ⚠️  AccessToken 修改为非明文储存，并增加对应 cli `vbumpp token set <provider>` 进行 accessToken 设置 (99ecab9)

### ❤️ 贡献者

- Whitekite <xuxjigsaw@qq.com>

## v4.2.1

[compare changes](https://gitee.com/vill-v/bump/compare/v4.2.0...v4.2.1)

## v4.2.0

[compare changes](https://gitee.com/vill-v/bump/compare/v4.1.0...v4.2.0)

### 🚀 特性

- **skills:** Add pnpm, tsdown, and turborepo agent skills (e9adeb4)
- **gitcode:** Add GitCode release support (31b3ee2)

### 🩹 修复

- **bump:** Handle string type for breaking change title (53117e0)

### 📖 文档

- Replace npm link to `npmx.dev` (2bd9a6b)

### ❤️ 贡献者

- Whitekite <xuxjigsaw@qq.com>

## v4.1.0

[compare changes](https://gitee.com/vill-v/bump/compare/v4.0.0...v4.1.0)

### 🚀 特性

- Upgrade `bumpp` v10 to v11, `cac` v6 to v7 (9636685)

### 🏡 框架

- Replace use `oxc` to lint code (0c0cb3f)
- **dep:** 不影响实际功能的常规依赖版本升级 (7ca8dd2)
- 使用 `mise`  管理项目 node 版本 (22f818a)
- **dep:** 不影响实际功能的常规依赖版本升级 (ce629b0)
- **dep:** 替换 `ora`  为`picospinner` 缩写依赖体积 (395c11c)
- Replace tsdown deprecated config (36ec6f1)

### ❤️ 贡献者

- Whitekite <xuxjigsaw@qq.com>

## v4.0.0

[compare changes](https://gitee.com/vill-v/bump/compare/v3.0.0...v4.0.0)

### 🚀 特性

- ⚠️ Upgrade minimum node version to 20 (315079f)

### 🏡 框架

- Using the pnpm catalog feature (f74725a)

#### 🚨 破坏性改动

- ⚠️ Upgrade minimum node version to 20 (315079f)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](https://github.com/Colourlessglow))

## v3.0.0

[compare changes](https://gitee.com/vill-v/bump/compare/v2.2.5...v3.0.0)

### 🚀 特性

- **dep:** Update dep `bumpp@10.1.0` `changelogen@0.6.1` (f78817b)

### 🩹 修复

- 修复初次执行时创建 git 仓库发布失败的问题 (7da8e55)

### 📦 打包

- ⚠️ Build ESM-only (011317a)

#### 🚨 破坏性改动

- ⚠️ Build ESM-only (011317a)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](https://github.com/Colourlessglow))

## v2.2.5

[compare changes](https://gitee.com/vill-v/bump/compare/v2.2.4...v2.2.5)

### 🚀 特性

- Changelog 设置复写增加 `style` (2c7c6be)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.2.4

[compare changes](https://gitee.com/vill-v/bump/compare/v2.2.3...v2.2.4)

### 🩹 修复

- 修复 changelog git 提交信息错误 (48bf19a)

### 🏡 框架

- Update CHANGELOG.md (3f2a2ad)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.2.4-beta.1

[compare changes](https://gitee.com/vill-v/bump/compare/v2.2.3...v2.2.4-beta.1)

## v2.2.3

[compare changes](https://gitee.com/vill-v/bump/compare/v2.2.2...v2.2.3)

### 🏡 框架

- **dep:** Replace `globby` to `tinyglobby` (419d3f8)
- **dep:** Replace `execa` to `tinyexec` (72ab83c)
- Bump `bumpp@9.7.1` `changelogen@0.5.7` `esconf@0.5.0` (3787f48)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.2.2

[compare changes](https://gitee.com/vill-v/bump/compare/v2.2.1...v2.2.2)

### 🩹 修复

- **gitlab:** 修复 openapi 调用地址错误 (df573e5)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.2.1

[compare changes](https://gitee.com/vill-v/bump/compare/v2.2.0...v2.2.1)

### 🩹 修复

- **gitlab:** 修复 gitlab openapi 并未传递项目 id 导致执行失败的问题 (874d5db)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.2.0

[compare changes](https://gitee.com/vill-v/bump/compare/v2.1.2...v2.2.0)

### 🚀 特性

- 新增 gitlab release 功能 (ffc21e7)

### 🏡 框架

- Use pnpm catalog (aa8855a)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.1.2

[compare changes](https://gitee.com/vill-v/bump/compare/v2.1.1...v2.1.2)

### 🚀 特性

- 升级 `esconf@0.3.3`,以替代 `rc9` 加载全局配置 文件 (462bba2)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.1.1

[compare changes](https://gitee.com/vill-v/bump/compare/v2.1.0...v2.1.1)

### 🩹 修复

- **github:** 修复 repo 输出显示错误 (e38b615)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.1.0

[compare changes](https://gitee.com/vill-v/bump/compare/v2.0.5...v2.1.0)

### 🚀 特性

- **github:** 尝试从环境变量与 github cli 获取 accesstoken (fb00628)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.0.5

[compare changes](https://gitee.com/vill-v/bump/compare/v2.0.4...v2.0.5)

### 🩹 修复

- 修复 `2.0.4` 无法解析全局 accesstoken 的问题 (14968f9)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.0.4

[compare changes](https://gitee.com/vill-v/bump/compare/v2.0.3...v2.0.4)

### 🩹 修复

- 修复 `2.0.3` 无法解析全局 accesstoken 的问题 (d0d6959)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.0.3

[compare changes](https://gitee.com/vill-v/bump/compare/v2.0.2...v2.0.3)

### 🩹 修复

- 修复 `2.0.2` 无法解析全局 accesstoken 的问题 (beee7f1)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.0.2

[compare changes](https://gitee.com/vill-v/bump/compare/v2.0.1...v2.0.2)

### 🩹 修复

- 替换 `c12` 为 `esconf` , 解决 `2.0.0` 无法解析全局 accesstoken 的问题 (33db264)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v2.0.1

[compare changes](https://gitee.com/vill-v/bump/compare/v2.0.0...v2.0.1)

## v2.0.0

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.6...v2.0.0)

### 🚀 特性

- **bumpp:** 修改内部代码结构，提供一些帮助方法,减少 `bumpp-gitee` 重复的依赖安装与重复代码 (0efb863)
- ⚠️ 重新设计大部分 api以增加 github release 功能 (4e066f1)

### 📖 文档

- Update README.md (6ed9602)

### 🏡 框架

- Update root workspace type to module (b0c6d8f)

#### 🚨 破坏性改动

- ⚠️ 重新设计大部分 api以增加 github release 功能 (4e066f1)

### ❤️ 贡献者

- Whitekite ([@Colourlessglow](http://github.com/Colourlessglow))

## v1.0.6

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.5...v1.0.6)

### 🏡 框架

- Update ci (c0a88e7)

### ❤️ 贡献者

- Whitekite <xuxjigsaw@qq.com>

## v1.0.5

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.4...v1.0.5)

### 🏡 框架

- Bump `bumpp@9.4.1` `execa@9.2.0` `ofetch@1.3.4` `rc9@2.1.2` (82e7555)

### ❤️ 贡献者

- Whitekite <xuxjigsaw@qq.com>

## v1.0.4

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.3...v1.0.4)

### 🩹 修复

- **bumpp:** 修复因为依赖 `bumpp@9.4.0` 内部 api 的破坏性改动导致的功能异常 (8705fd4)

### ❤️ 贡献者

- Whitekite <xuxjigsaw@qq.com>

## v1.0.3

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.2...v1.0.3)

### 🏡 框架

- **build:** 修改项目打包目标为 node18 (15cba0a)

### ❤️ 贡献者

- Whitekite <xuxjigsaw@qq.com>

## v1.0.2

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.1...v1.0.2)

## v1.0.1

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.0...v1.0.1)

### 🩹 Fixes

- 修复尊崇 `changelogen` 配置文件导致的模块自身默认配置被忽略 (fca754d)
- **bumpp:** 修复配置项 `changelog` 允许的配置不准确，ps-虽然尊重 `changelogen` `bumpp` 各自的配置文件，但实际可配置内容,由于本插件使用的性质，并未完全开放，推荐使用插件自身的配置文件 `vbumpp.config.{mc}{tj}s` (746b011)

### ❤️ 贡献者

- Whitekite <xuxjigsaw@qq.com>

## v1.0.0

[compare changes](https://gitee.com/vill-v/bump/compare/v0.4.4...v1.0.0)

### 🚀 Enhancements

- **bumpp:** 尊重 `changelog` `bumpp` 各自的配置文件 (d600aab)
- **bumpp-gitee:** 使用 `consola/utils` 替换 `chalk` (22e5787)

### 🩹 Fixes

- **bumpp:** 修复 changelog markdown 标题替换失败 (578ac5d)

### 🏡 Chore

- **bumpp:** Update dep `bumpp^9.3.0` `changelogen^0.5.5` (92d8cf8)
- **dep:** ⚠️ 由于以下依赖升级 `ora^8.0.1` `globby^14.0.1` `execa^8.0.1`，修改最低 node 版本 v18 (c832993)
- Update .gitignore (7188e99)

#### 🚨 破坏性改动

- **dep:** ⚠️ 由于以下依赖升级 `ora^8.0.1` `globby^14.0.1` `execa^8.0.1`，修改最低 node 版本 v18 (c832993)

### ❤️ 贡献者

- Whitekite <xuxjigsaw@qq.com>

## v0.4.4

### 🚀 特性

- 定制 break change，Contributors 生成changelog的文案 (aab98d0)

### ❤️ 贡献者

- Whitekite <xuxjigsaw@qq.com>

## v0.4.3

### 🏡 框架

- **dep:** Changelogen update to 0.5.4 (1f68bb9)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.4.2

### 🚀 特性

- **gitee:** 由于 open-api gitee pages build 功能 功能仅限 付费用户，遂删除该功能 (342ba37)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.4.1

### 🚀 特性

- **gitee:** Change gitee pages build cli command (5f07084)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.4.0

### 🏡 框架

- ⚠️ Change package to es module (0ec80b3)
- 补充部分代码注释信息 (28636de)

#### ⚠️ Breaking Changes

- ⚠️ Change package to es module (0ec80b3)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.3.2

### 🚀 特性

- **gitee:** Add gitee pages build util (dd8e44b)

### 🏡 框架

- **gitee:** Update README.md (92dab92)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.3.1

### 🚀 特性

- **gitee:** Update console log (165071c)

### 📖 文档

- Update README.md (498c530)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.3.0

### 🚀 特性

- **gitee:** 增加一个简易的模块 `@vill-v/bumpp-gitee` 提供release 后的gitee操作 (fe4a1ae)

### 🏡 框架

- Update CHANGELOG.md (9c6f730)
- Update CHANGELOG.md (1176f67)
- Update CHANGELOG.md (f5b009e)
- Update CHANGELOG.md (d51f8aa)
- Update CHANGELOG.md (e94016e)
- Update CHANGELOG.md (449ab35)
- Update CHANGELOG.md (552b21c)
- Update CHANGELOG.md (b8050b4)
- Release v0.2.4 (132603d)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.4

### 🚀 特性

- **gitee:** 增加一个简易的模块 `@vill-v/bumpp-gitee` 提供release 后的gitee操作 (fe4a1ae)

### 🏡 框架

- Update CHANGELOG.md (9c6f730)
- Update CHANGELOG.md (1176f67)
- Update CHANGELOG.md (f5b009e)
- Update CHANGELOG.md (d51f8aa)
- Update CHANGELOG.md (e94016e)
- Update CHANGELOG.md (449ab35)
- Update CHANGELOG.md (552b21c)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.4

### 🏡 框架

- Update CHANGELOG.md (9c6f730)
- Update CHANGELOG.md (1176f67)
- Update CHANGELOG.md (f5b009e)
- Update CHANGELOG.md (d51f8aa)
- Update CHANGELOG.md (e94016e)
- Update CHANGELOG.md (449ab35)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.4

### 🏡 框架

- Update CHANGELOG.md (9c6f730)
- Update CHANGELOG.md (1176f67)
- Update CHANGELOG.md (f5b009e)
- Update CHANGELOG.md (d51f8aa)
- Update CHANGELOG.md (e94016e)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.4

### 🏡 框架

- Update CHANGELOG.md (9c6f730)
- Update CHANGELOG.md (1176f67)
- Update CHANGELOG.md (f5b009e)
- Update CHANGELOG.md (d51f8aa)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.4

### 🏡 框架

- Update CHANGELOG.md (9c6f730)
- Update CHANGELOG.md (1176f67)
- Update CHANGELOG.md (f5b009e)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.4

### 🏡 框架

- Update CHANGELOG.md (9c6f730)
- Update CHANGELOG.md (1176f67)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.4

### 🏡 框架

- Update CHANGELOG.md (9c6f730)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.4

## v0.2.3

### 🚀 特性

- **deps:** `bumpp@9.1.0` `ora@6.3.0` `changelogen@.5.2` (3b3c5d0)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.2

### 🚀 特性

- **deps:** `execa@7.1.1` `6.2.0@ora` `c12@1.2.0` (6f3c1c4)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.1

### 🩹 修复

- 修复 git commit changelog file失败的问题 (2bb753c)
- 修复 git commit changelog file失败的问题 (ac17568)
- 修复 git commit changelog file失败的问题 (eb889b8)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.1

### 🩹 修复

- 修复 git commit changelog file失败的问题 (2bb753c)
- 修复 git commit changelog file失败的问题 (ac17568)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.1

### 🩹 修复

- 修复 git commit changelog file失败的问题 (2bb753c)
- 修复 git commit changelog file失败的问题 (ac17568)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.1

### 🩹 修复

- 修复 git commit changelog file失败的问题 (2bb753c)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.2.0

### 🚀 特性

- 升级部分依赖 `changelogen@0.5.1` `execa@7.1.0` (8df5a97)
- 修改changelog生成默认配置 (9894abd)
- 修改内部changelog git command 使用`execa@7.1.0` `$` 特性 (932869a)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.1.1

### 🚀 特性

- 还原 `bumpp@9.0.0` `recursive` 特性的适配 (c4ce200)
- 增加默认的 `bumpp` 默认文件 `package-lock.json` (b6fd1c6)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.1.0

### 🚀 特性

- 适配 `bumpp@9.0.0` `recursive` 特性 (4c136cd)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.0.4

### 🏡 框架

- Update README.md (d2bc879)
- 修改 changelog 加载动画 (2d8cbfc)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.0.3

### 🚀 特性

- Changelog 加载动画换行 (5d9551a)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.0.2

### 🚀 特性

- Changelog 增加加载动画 (e8d50c8)
- 增加 bumpp.recursive 属性,支持monorepo 项目 (e20fee4)
- 默认 `bumpp` 当前 `process.cwd()` 下 `package,json` (d5db56e)
- Bumpp 不再需要二次确认 (020339a)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>

## v0.0.1

### 🏡 框架

- Add README.md (c4de77e)
- Update README.md (aaec057)

### ❤️ Contributors

- Whitekite <xuxjigsaw@qq.com>
