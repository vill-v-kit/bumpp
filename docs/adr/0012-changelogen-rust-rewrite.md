# changelogen 使用面 Rust 重写

npm/bump 依赖 changelogen@0.6.2 的 7 函数 2 类型：git 历史读取（`getLastGitTag` / `getGitDiff` / `getCurrentGitBranch`）、`resolveRepoConfig`、`parseCommits`、`generateMarkDown`、`loadChangelogConfig`。其 12 个直接依赖的传递链（c12 / ofetch / semver / convert-gitmoji…）与两个隐蔽行为——`generateMarkDown` 逐作者请求 `ungh.cc` 解析 GitHub 用户名（失败静默降级，产出随网络环境漂移）、英文节标题硬编码需 JS 字符串替换修补——促使将使用面整体改写为 Rust，changelogen 依赖删除。

## Decisions

- **范围**：仅移植使用面；changelogen 其余能力（GitHub release 同步系 `createGithubRelease`/`syncGithubRelease`、自家 `bumpVersion`/`determineSemverChange`、CLI、`parseChangelogMarkdown`）不搬。验收标准 = npm/bump 的 `package.json` 删除 `changelogen` 依赖。
- **放置**：`crates/bumpp-core` 内新 `changelog/` 目录（`mod` 编排 / `markdown` / `config` / `gitmoji`）；判别标准「脱离业务语境能否自圆其说」——能则根部共享不隔离：`git.rs` 增只读历史操作（log / describe / branch / remote URL 解析），`commits.rs` 原位补全 authors/references（「暂不移植」口子补上）。不新建 crate、不新增 napi 包，5 个平台二进制矩阵不变。
- **napi 面**：高层单函数 `generateChangelog`（统一配置解析 → diff → 解析 → `chore(deps)` 过滤 → markdown → 读旧文件插入 → 写盘 → 提交，全包）+ 四个 git 历史只读导出（`getLastGitTag` / `getGitDiff` / `getCurrentGitBranch` / `resolveRepoConfig`——前三者 JS 直接消费，`getGitDiff` 对齐 changelogen 同名面同时供 Rust 编排内部使用）。`parseCommits` / `generateMarkDown` / **changelog 配置解析**不跨语言边界——全项目只有一条统一配置文件解析逻辑（ADR-0013），JS 无可调用的 `loadChangelogConfig`，用户 overrides 由 `generateChangelog` 入参透传、文件与默认的合并在 Rust 内部完成。上游 parity 三 API（`versionBump` / `versionBumpInfo` / `loadBumpConfig`）不动。
- **changelog.ts 内置时的修复清单**（原 JS 实现的坐实缺陷，随重写一并修）：
  - **N1 node-only**：`git add <output> package.json` 硬编码 package.json——changelog 跑在 versionBump 之前，该文件尚未修改（add 无效），且纯 cargo 项目无此文件 → pathspec 报错整步炸（ADR-0011 同类假定）。修复：只 add 实际写出的 output 文件。
  - **C1 tag 格式脱节**：`from`/`to` 硬编码 `v` 前缀，与 `templates.tagBody` 渲染的实际 tag 名脱节，非 v 前缀项目 `getGitDiff` 找不到 ref。修复：`from` 取 `getLastGitTag` 的真实 tag 名；compare 链接的 `to` 用 tagBody 渲染值。
  - **C2 提交不听从开关**：无条件 `git commit`。修复：changelog 的提交跟随统一配置中的 bumpp `commit` 开关——`false` 时只写文件，不 add 不 commit。
  - **C3 提交信息无配置位**：changelog 段新增 `commitMessage` 键，默认 `chore: update {{output}}`（`{{output}}` 占位符替换为 output 路径）。
- **输出结构逐节对齐 changelogen 0.6.2**：`config.types` 声明序分组、组内 reverse（旧→新）、compare 链接（任意 provider 恒出链接，bitbucket 走 `branches/compare` 特判——引用链接才限 github/gitlab/bitbucket 三 provider 出链，其余纯文本）、引用链接、`## ` 头经 `templates.tagBody`（默认 `v{{newVersion}}`，仅此一键纳入支持面）渲染、`^###?\s+` 插入逻辑。申报三处有意偏差：① 中文节标题直生（breaking 节取 `types.BreakingChange.title`，贡献者头硬编码 `### ❤️ 贡献者`，JS replace hack 删除）；② 无 `@username` 链接；③ `chore(deps)` 过滤内置 Rust（行为不变只挪位置）。
- **ungh.cc 网络解析不移植**：非确定性（release 工件不应随网络环境漂移）、隐私（贡献者邮箱明文发第三方，用户无感知）、依赖成本（纯 Rust crate 零网络依赖，HTTP+TLS 栈换装饰性链接）、平台错位（面向四平台，GitHub 用户名解析对其余三家基本错误）。
- **`hideAuthorEmail` 默认翻转为 `true`**（changelogen 默认 `false`）：邮箱不进公开归档（爬虫饲料；changelogen 还特意跳过 noreply 优先暴露真实邮箱）；配合网络杀除，贡献者行默认为纯 `- 名字`。
- **gitmoji**：convert-gitmoji 的 74 条静态映射原样内建 `changelog/gitmoji.rs`（`:code:` → emoji + 尾随空格，大小写不敏感），依赖删除。
- **申报偏差④**：gitmoji 键名按字面量转义匹配（原实现未转义拼正则，`+` 等字符沦为量词——对真实输入行为一致，病理输入不再误中）；**非偏差补注**：上游 CLI 编排层的 type `toLowerCase()` 不在使用面函数（`parseGitCommit` / `generateMarkDown`）内，现行生产 JS 亦无此步，`Feat:` 大小写敏感丢弃行为与今日产出保持一致。
- **TS 类型**：`#[napi(object)]` 结构体生成 d.ts，单一事实源在 Rust（沿用 `VersionBumpOptions` 先例）；公开面为 `ChangelogOptions`（入参键集）与 `GenerateChangelogResult`（`markdown` / `changelogMD`）——`ResolvedChangelogConfig` 不导出（解析不跨语言边界），原 changelogen 类型 import 删除。`types` 值 `{title} | false`（false = 深合并删除哨兵）。
- **测试**：Rust 单测为主体（tempfile 合成 git 仓库，仓库已有先例）；golden fixtures 钉住 parity——真 changelogen 0.6.2 在合成仓库的产出经三处申报偏差等效变换后固化于 `crates/bumpp-core/tests/fixtures/`（头注释记录出处与变换清单，生成脚本 dev-only 留档）；JS 侧仅 napi 链路冒烟。原 `config.test.ts` 删除，recursive 用例随功能移植 Rust（ADR-0013）。

## Considered Options

- **新建 `crates/changelog-core`**：commits / config / exec 三样须跨 crate 复用（bumpp-core 变其依赖）或重复实现，仅两家消费者的现状下属过度设计——拒绝。
- **全粒度 drop-in**（7 函数全导出、JS 编排照旧）：编排层永留 JS，与「逻辑改写为 Rust」初衷相悖，且导出无独立消费者的符号是 API 负债——拒绝。
- **字节级 parity**（保留英文节标题 + JS replace hack）：把补丁固化进新实现——拒绝；结构对齐 + 申报偏差兼顾归档连续性与诚实。
- **保留 ungh.cc 解析或做成配置开关**：见 Decisions 网络杀除四条理由；将来确需 `@` 链接时以 opt-in 配置回归，不在本期。

## Consequences

- 破坏性改动（随大版本发布）：子路径 `@vill-v/bumpp/changelogen` → `./changelog` 且导出收窄为自有符号（github×2 / gitlab×1 三处仓库内导入点同步改）；`hideAuthorEmail` 默认 `true`；贡献者节无 `@username` 链接；changelog 提交行为改随 bumpp `commit` 开关（原无条件提交）。
- 依赖删除：npm/bump 移除 `changelogen` 及其传递链（c12 / confbox / ofetch / convert-gitmoji / semver / consola 传递份等）；`tinyglobby` 一并移除（文件 globbing 早已收归 Rust `normalize_files`，源码零引用，纯存量）。
- 配置源死亡：`changelog.config.*`（c12）、package.json `changelog` 键（静默忽略）、`.env`（setupDotenv）——配置文件统一见 ADR-0013。
- `generateChangelog` 全程无网络，napi 侧无需异步网络栈。
