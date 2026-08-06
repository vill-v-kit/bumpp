# 剩余功能全面 Rust 化与 napi 面收缩

JS 侧残留三块功能——Token 存储（accesstoken.ts）、`bump.ts` 薄编排、四平台包的 release HTTP（ofetch）——且渐进迁移期的垫脚石 napi 导出（`updateFiles` / `gitCommit` 等）已无 JS 消费者。决策：三块全部收归 Rust（CLI 的 cac 参数路由除外——该例外后由 ADR-0016 移除），napi 导出面同步收缩。

## Decisions

- **范围全收**：Token 存储、编排、平台 Release 三块进 Rust；token 录入的密码交互一并进 Rust（`dialoguer::Password`，`prompt.rs` 已有 dialoguer 先例），`@inquirer/password` 依赖移除。JS 剩余：cac 命令路由 + re-export 壳。
- **HTTP 选型 ureq 3.x + rustls**：同步 API 与 core 现状一致；无 OpenSSL（五平台交叉编译链不动）；无并发需求不引入 tokio。HTTP 调用收敛在单一模块内，未来换 reqwest 不伤编排。（本条一度被 ADR-0024 暂时取代为 native-tls + vendored openssl——v6.0.0 上古交叉链两连炸的临时偏离；ADR-0027 换 zig 交叉链后已撤销偏离，本条恢复生效）
- **Token 存储逐字节兼容**：VBTK v1 布局（magic 4B + version 1B + iv 12B + authTag 16B + AES-256-GCM 密文）、`key.bin`、0600/0700 权限位、`VBUMPP_TOKEN_STORE` 覆盖、损坏自愈重写、清空删文件——全部对齐 JS 时代行为；crypto 用 `aes-gcm` crate；golden test 用 Node 版预生成样本校验解密。
- **token 解析链统一**：Token 存储 → 各家环境变量 →（仅 github）`gh auth token` CLI 兜底。环境变量：github 为 `GH_TOKEN` → `GITHUB_TOKEN`（拼错的 `GITHOB_TOKEN` 移除——已发布但属 typo 修复，随大版本）；gitlab / gitee / gitcode 补 `GITLAB_TOKEN` / `GITEE_TOKEN` / `GITCODE_TOKEN`（CI 场景此前无通道）。
- **napi 面收缩**：删 `plus100`（脚手架残留）；藏 `updateFiles` / `gitCommit` / `gitTag` / `gitPush` / `versionFileManifestGlobs` / `loadBumpConfig` / `versionBump` / `versionBumpInfo` / changelog 系五函数（`generateChangelog` / `getLastGitTag` / `getGitDiff` / `getCurrentGitBranch` / `resolveRepoConfig`）；`@vill-v/bumpp/changelog` 子路径删除。`loadBumpConfig` 收编后回归 ADR-0013「解析结果不向 JS 导出」原则。
- **编排形状**：单 napi `bumpVersion(options, provider?)`——provider 缺省仅 bump，传值则 bump 后接平台 Release；spinner 动画换 Rust 进度打印（ADR-0002 先例）；`bumpVersionWithBaseRelease` 与 `picospinner` 移除。
- **明文 token 不出 Rust 边界**：`BumpVersion` 收缩为 `{ bumpp, changelog? }`（`config` 字段与 `ResolveConfig` 类型删除）；独立 `createXRelease` 入参同形，token / repo / host 由 Rust 内部解析。
- **Release 导出面**：per-provider 四个 napi 函数（`createGithubRelease` / `createGitlabRelease` / `createGiteeRelease` / `createGitcodeRelease`），共享实现为 Rust 内部细节；gitee / gitcode 对 `@vill-v/bumpp-github` 的跨包依赖消除；`createGithubLikeRelease` 工厂与 `ICreateGithubLikeRelease` 从公共 API 消失。
- **gitlab.host 修复**：原 `Config.gitlab.host`（module augmentation 声明）因 `bump.ts` 构造 config 时丢失 `gitlab` 键从未生效；修复为配置 schema 顶层 `gitlab?: { host?: string }` 段（纳入严格 schema 白名单），缺省 `https://gitlab.com`，全局配置层生效后自建实例一次配置全项目可用。项目 id 查询由「搜索 + web_url 后缀匹配」两步改为 `GET /api/v4/projects/<url编码的 owner/repo>` 直查。

## Considered Options

- **reqwest（async + tokio 或 blocking）**：为 1~2 个串行请求引入整个 tokio 编译成本，且 core 当前零 async 运行时——拒绝。
- **保留 versionBump 系上游 parity 导出**：`@vill-v/bumpp` 主路径从未转发它们，要用只能直装内部机制包（ADR-0005 明判无此场景）——死公共 API，拒绝保留。
- **保留 `@vill-v/bumpp/changelog` 子路径**：Rust 实现仍在、边际成本为零，但「实现全内部」的收窄价值优先——用户裁决砍掉。
- **单一 `createRelease(provider, options)` 导出**：provider 字符串化、存量 `createGithubRelease` 调用方面临 API 变更——拒绝。
- **token 存储借机换格式（v2 / 新 KDF）**：现格式无缺陷，破坏兼容只伤害存量用户——拒绝。

## Consequences

- 破坏性改动（随大版本）：`@vill-v/bumpp/changelog` 子路径删除；`BumpVersion` 形状收缩（`config` 字段删除）；`bumpVersionWithBaseRelease` 删除；`GITHOB_TOKEN` 环境变量移除；`Config` 类型增 `gitlab` 段（npm/gitlab 的 module augmentation 删除）。
- 依赖清理：npm/bump 删 `picospinner` / `@inquirer/password`；四平台包删 `ofetch` / `tinyexec`；gitee / gitcode 删对 `@vill-v/bumpp-github` 的依赖。
- Rust 依赖增量：`ureq` / `aes-gcm`（及 ADR-0015 的 `toml`）。
- 五个平台二进制包包含 HTTP/TLS 栈，体积增长，发布前需复测交叉编译链。

## 落地注记（2026-08，ADR-0018 拆分同期）

- 「明文 token 不出本模块」对**错误消息**同样成立：`ReleaseError::redact(token)` 在四家 provider 注入缝出口统一脱敏——原始形态与 form 编码形态（gitcode 经 query 注入，ureq 传输报错与服务端错误回显都可能携带 URL/请求体）一律替换为 `[redacted]`。行为锚点：`tests/release/{gitcode,gitee}.rs` 的 `*_error_never_leaks_token`。
