# v6 重写约束归档（压缩自原 0001–0039 共 28 份 ADR）

v6 将 bumpp（JS）整体重写为 Rust 核心。本文是全部架构决策的压缩归档：只保留至今仍然约束开发行为的结论，不保留决策过程与备选讨论。按领域组织，编号已废弃——新增决策从 0041 起另立新篇。

## 1. 核心架构：纯 Rust 引擎 + 薄壳前端

- 版本计算、文件更新、changelog、git 操作、token 存储、平台 Release、HTTP 全部归 Rust（`crates/vbumpp-core`）。Node 侧零业务逻辑。
- napi 面仅两个导出：`bumpVersion(options, provider?)` 与 `cliRun(argv, provider?)`。低层能力（git、版本、changelog、token、createXRelease）不公开。
- overrides 经 napi 类型化结构体边界入 Rust，TS 类型由 napi 自动生成，npm 侧不手写门面 interface。
- git 操作经用户环境的 `git` 命令执行（继承 git config / SSH / GPG / credential helper）。
- HTTP 用同步 `ureq` + rustls（ring 后端），不引入 async runtime；依赖图不含 OpenSSL。
- 进度由 Rust 内置打印，`ProgressEvent` 不向 Node 导出；进度、警告、路径错误统一显示路径规则：cwd 内相对、cwd 外绝对、一律 POSIX（存储与 API 返回值保持绝对原生路径）。
- 用户可见字符串（错误、help、prompt、进度、panic 兜底、loader 报错、schema description）唯一语言英文；本地化走配置（如本仓 `.vbumpprc.toml` 的中文 changelog 标题），不引入运行时 i18n。代码注释不受此约束。

## 2. CLI：全权归 Rust，单入口双薄壳

- argv 语法（子命令、flag、help、错误、退出码）唯一归属 Rust 手写解析器，不引入 clap——cac 时代 argv 语义（truthy、短簇、`--`、`--flag=value`）已有全套测试与文档作 parity 基准。
- npm bin 与原生二进制 `crates/vbumpp`（零 napi 依赖）共享 `run_from_argv`，均为无逻辑薄壳，行为不漂移。
- `--provider` flag 优先于平台变体包注入；provider 不从配置或 git remote 推断。
- CLI 默认值不物化进 overrides：仅用户显式给出的 flag（files / `-r` / `-o`）才注入——「配置写对即生效」，配置文件永不被 CLI 默认值顶掉。
- `vbumpp release <version>` 是 bump 的独立重试通路：从 changelog 文件提取版本节，前置校验（版本节存在、本地 tag 存在）失败即 exit 1；远端已存在不自动更新。
- `token set/list/remove` 直调 Rust token 模块；`vbumpp schema` 输出配置 JSON Schema（stdout / `--write` 落盘）。

## 3. 配置：两级文件、单一解析路径、严格校验

- 项目级 `.vbumpprc.{json,jsonc,toml}` + 全局级 `~/.vbumpp/config.{json,jsonc,toml}`；四层合并：overrides > 项目 > 全局 > 内建默认。加载全权归 Rust，解析结果不向 JS 导出。
- `.json` 与 `.jsonc` 同走 JSONC 解析；TOML datetime 拒绝（JSON 值域无表达）。YAML、TS/JS 配置函数不支持。
- 旧名（`bump.config.*` 等）不探测、静默失效；同级命中多个配置文件报错并全部列出。
- 键名与类型双重校验：未知键一次全列，类型不符即报错而非静默回落默认；`$schema` 键合法。
- 合并语义：bumpp 键浅替换；仅 `changelog.types` 按键深合并，值 `false` 禁用该组。
- 配置形状以 Rust 结构体为单一事实源，机械导出 JSON Schema；schema 产物提交仓库（npm 包内副本 + website 静态导出），CI 漂移校验腿防腐。编辑器提示只走显式 `$schema` 键 / Taplo `#:schema` 指令，不走 SchemaStore。
- `scripts` 三槽位（preversion/version/postversion）是配置声明的通用 shell 命令（Unix `sh -c` / Windows `cmd`），不从 package.json 读取 npm scripts；非零退出立即中止 bump。
- changelog.output 优先级链：`-o` flag > 配置 `changelog.output` > 内建默认 CHANGELOG.md；bump 与 release 同路解析。

## 4. 生态插件底座（src/plugins/）

- `VersionFilePlugin` trait + 静态链 JavaScript → Cargo → Text，按 `matches` 首命中分发；Text 是仅做版本文本替换的兜底，不贡献生态、默认清单或版本来源。无运行时 registry。
- 清单 basename 集合的单一事实源是插件链：默认 files = 链上 basenames 根级并集，recursive = `**/` 并集；显式配置 files 整体替换默认。
- 更新两段式：判定段（`plan`，只读）与写盘段（经 `Effects` 效应边界执行，零决策）。附带同步文件紧随主文件（如 Cargo.toml → 最近上级 Cargo.lock 按 crate name 定向同步；lock 解析失败 / 条目缺失 / 版本漂移即报错，全部检查通过才写盘）。
- Cargo 通道：toml_edit 保格式只改版本字段；成员 `version.workspace = true` 不写字面量，由根 `[workspace.package].version` 统一更新。
- `--install` 仅在本次有实际更新时按命中生态去重执行（JS：检测到的 `<pm> install`；Cargo：`cargo check --workspace`）；仅 Text 更新回退 JavaScript；全 skip 不执行。
- 包管理器检测：从 cwd 逐级向上，目录外层优先；每级依次 lockfile/workspace 文件 → `packageManager` → `devEngines.packageManager`（只取 name）。不复刻 Corepack 语义。
- `-r` 整树收集不按 `"private": true` 过滤——private 仅表示不上架，版本随整树锁步（单一版本锁步模型）。收集器过滤层仅内置目录排除 + gitignore 感知；迁移 ignore crate 暂缓（无性能投诉不重启）。

## 5. Changelog 与 dry-run

- changelog 全程 Rust、零网络：不调 ungh.cc 解析用户名（不泄露邮箱），`hideAuthorEmail` 默认 true；gitmoji 内建静态映射；起点用实际最近 tag 不硬编码 `v` 前缀。
- `--dry-run` 与真实执行骑同一条流水线，副作用在 `Effects` 边界（文件写盘、子进程、平台 HTTP 三原语）拦截为计划行——保真由结构保证，不靠纪律。
- dry-run 口径：前置校验照常硬失败 exit 1（可当 CI 预检门禁）；token 缺失只警告不报错、存在则报告来源；零写盘零 git 写零网络；版本选择交互保留、`Bump?` 确认跳过。
- 明文 token 不跨 napi 边界、不进错误信息；release 预览的 token 明文不出 release 模块，拦截行经同一脱敏原语。

## 6. Token 存储

- VBTK v1 加密信封（AES-256-GCM），与 JS 时代逐字节兼容，旧键零迁移；`tokens.bin` + `key.bin` 同放 `~/.vbumpp/`，0600/0700 权限。防护级别「防明文落盘」。
- 键两级：provider 级键 + host 作用域复合键 `provider@host`；host 经统一规范化函数键化（补 https、小写、去尾斜杠、留端口路径）。解析链：host 精确键 → provider 级键回落（向后兼容硬要求）→ 环境变量 → 仅 github 追加 `gh auth token`。
- `--host` 目前仅 gitlab 开放；remove 交互矩阵：`--dry-run` 只列清单优先、无 `--yes` 默认 No 二次确认、非 TTY 无 `--yes` 报错。

## 7. 平台 Release

- 四 provider（github/gitlab/gitee/gitcode）Rust 内适配：gitee/gitcode 复用 github-like；gitlab 专用（项目 id 直查，自建实例经 `gitlab.host` 配置）。
- token 解析链同第 6 节；环境变量顺序 `GH_TOKEN` → `GITHUB_TOKEN` / `GITLAB_TOKEN` / `GITEE_TOKEN` / `GITCODE_TOKEN`。
- bump 末段自动创建（body = 当次 changelog）；`release` 子命令独立重试（body 从文件提取版本节）。

## 8. 包布局与分发

- `napi/` 收纳内部机制包、`npm/` 收纳用户包——判别问句「用户会直接 npm install 它吗？」，与是否发布无关。根 Cargo workspace members/exclude、CI 路径必须与该边界一致。
- 平台矩阵 7 targets（napi 5 平台 + linux musl×2）；**不含 darwin-x64**——npm 渠道 Intel Mac 硬失败（loader 生成物标准报错），替代路径是 cargo 渠道。
- `napi.targets` 是支持平台的单一真相源：→ `napi create-npm-dirs` 生成平台包目录（gitignore 不提交）→ optionalDependencies `workspace:*`。fresh clone 目录缺失时 pnpm 静默跳过，loader 走本地 `.node` fallback。
- loader 是 napi-rs 官方生成物（`napi build --platform --esm`，零手写）：不提交 git，经 CI 构建腿捎带、publish 时归位 core。自研 loader 已删除（Socket.dev 供应链告警与生态同构考量；评分目标最终未达成，但维护面归零的收益独立成立）。
- 发布态 core 包不内置任何 `.node`（`files` 无 `*.node`）：optionalDependencies 平台包是唯一分发通道；包根 fallback 仅存本地开发磁盘。publish 预验证断言 core tarball 不含 `.node`。

## 9. 发布流水线

- CI 仅 `v*` tag 推送触发，tag 推送即授权、无人工门；test → 7 平台 build → test-bindings 全绿后 publish-npm 与 publish-crates 并行。
- 认证 OIDC-only（npm trusted publishing + crates-io-auth-action）；全新包名首发需一次性经典认证仪式，由 npm-publish 脚本前置检测拦停漏做。
- 两个 publish job 幂等（skip-if-published），部分失败后唯一恢复手段是 Re-run failed jobs；crates 按 vbumpp-core → vbumpp 硬序。
- crates workspace 依赖宽松版本 `>=5.1, <7`，供 cargo publish 改写；`napi/bumpp-core` 内部 crate 不进公开 crates 清单。
- `build-cli` job 后置追加预编译 CLI 资产（`gh release upload --clobber`），失败不回滚 npm/crates。
- 质量门前移本地 hk hook：pre-commit `cargo fmt --check` + `tsc --noEmit`，pre-push clippy；rust 工具链 mise 钉版（浮动 stable 会让 fmt/clippy 漂移攒到发版首爆）。发版脚本 `HK=0` 绕 hook（hk pre-push 对 annotated tag 的上游缺陷）。

## 10. cargo-binstall 渠道

- 与 npm 并列的一等安装渠道：产物命名 `vbumpp-{target}.tar.gz` / 顶层目录 / `v{version}` tag 直接命中 binstall 默认模板，故 Cargo.toml 零 binstall 元数据；每产物附 `.sha256`。
- linux arm64-gnu 必须用 `aarch64-unknown-linux-gnu.2.17` target spec（CI 断言 GLIBC ≤ 2.17）；musl 与 arm64 交叉走 zig + cargo-zigbuild。
- 与 npm 渠道的「不支持」语义不同：binstall 无匹配产物自动回退源码编译（降级），npm 是硬失败。

## 11. 仓库工程约定

- 仓库脚本（`scripts/`、`website/scripts/` 等）一律 TypeScript 由 node 原生直跑（type stripping，node ≥22.18 由 mise lts 保证）；`erasableSyntaxOnly` 硬约束（禁 enum/namespace）、import 带完整 `.ts` 后缀；类型检查由根 `tsc --noEmit` 挂 hk pre-commit。例外：`npm/*/bin/` 发布物薄壳与 napi loader 生成物保持 JS。
- 文档网站 `website/`（fumadocs / Next.js 静态导出）：纯中文单版本，部署 GitHub Pages 子路径 `/bumpp`——所有运行时拼接 URL 必须显式带 basePath；与 `docs/` 工程内部文档物理分离。
- 首页滚动演示：自产 asciicast v2 兼容 cast 时间线（全 dry-run 离线确定性采集）+ wterm 渲染层；产物本地生成提交、CI 漂移校验腿防腐；seek 为重放语义。CLI 输出变更令漂移腿红是有意的——演示腐烂显式暴露。

## 附：编号对照（仅供考古 git 历史）

原 28 份 ADR 全文可从 git 历史找回（删除提交前一版）。主题映射：0001/0014 核心全 Rust 化 · 0002 进度内置 · 0003 Cargo 版本同步 · 0005 napi 目录受众规则 · 0006 包管理器检测 · 0007 插件底座 · 0011 scripts 通用 shell · 0012 changelog Rust 重写 · 0013 统一配置 · 0016 CLI 单入口 · 0017 英文文案 · 0020 文档网站 · 0021 CI 发布 · 0022 ignore crate 暂缓 · 0025 binstall 渠道 · 0028 无 Intel Mac · 0029 平台包生成 · 0030 private 锁步 · 0031 hk 质量门 · 0032 core 不内置 .node · 0033 loader 生成物 · 0034 dry-run 语义 · 0035 token host 键 · 0036 演示 cast 管线 · 0037 配置 schema · 0038 仓库脚本 TS 化 · 0039 changelog.output 回落。
