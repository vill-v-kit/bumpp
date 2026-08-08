# 核心功能全 Rust 化与 provider 分层

版本编排、Token 存储、平台 Release 与 HTTP 全部归 Rust。Node 包只保留薄入口和 re-export；Core 的 napi 面仅保留高层编排与 CLI 入口。

## Decisions

- **范围**：Token 存储、bump 编排、四个平台 Release 与 token 密码交互均在 Rust；Node 侧不持有业务逻辑。
- **HTTP/TLS**：使用同步 `ureq` 3.x 与 rustls（ring 后端），关闭 OpenSSL/native-tls。平台 Release 只有少量串行请求，不引入 async runtime。
- **Token 存储**：保持 VBTK v1 字节布局、`tokens.bin` / `key.bin`、0600/0700 权限、损坏自愈和清空删除语义。`VBUMPP_TOKEN_STORE` 可覆盖存储文件，`VBUMPP_HOME` 可覆盖全局目录。
- **Token 解析链**：Token 存储优先，其次平台环境变量；GitHub 额外以 `gh auth token` 兜底。GitHub 环境变量顺序为 `GH_TOKEN`、`GITHUB_TOKEN`；其余为 `GITLAB_TOKEN`、`GITEE_TOKEN`、`GITCODE_TOKEN`。
- **明文边界**：明文 token 不跨 napi 边界，也不得出现在错误信息中；原始和 form 编码形态在 provider 出口统一脱敏。
- **高层编排**：`bumpVersion(options, provider?)` 负责完整 bump；provider 存在时在 bump 末段创建平台 Release。进度由 Rust 打印。
- **Release 布局**：`release/{github,gitee,gitcode,gitlab}.rs` 每个 provider 单文件。GitHub、Gitee、GitCode 复用 `github_like.rs` 的请求体和 endpoint 语义；GitLab 保持专用实现。`http.rs` 放共用仓库解析与 HTTP 原语，`mod.rs` 放 `Provider`、错误、token 链与分发。
- **GitLab**：统一配置顶层支持 `gitlab.host`，默认 `https://gitlab.com`；项目通过 URL 编码的 `owner/repo` 直接查询。
- **napi 面**：仅保留 `bumpVersion(options, provider?)` 与 `cliRun(argv, provider?)`。低层 git、版本、changelog、token 与 `createXRelease` 导出均不公开；独立 Release 由 CLI 的 `release` 子命令承担。
- **测试布局**：Release 测试镜像生产目录，provider 行为各自成文件，共享 mock 与 token 链测试放公共位置。

## Consequences

- npm 主包和平台变体包仅是 Rust 入口薄壳；Gitee、GitCode 不再依赖 GitHub npm 包共享实现。
- 五个平台二进制包含 Rust HTTP/TLS 栈；现代 zig 交叉链支持 rustls/ring，依赖图不含 vendored OpenSSL。
- 新增 provider 时需增加 provider 文件、在 `Provider` 注册，并补对应行为测试；`--provider` 可选值由同一注册词汇解析。
