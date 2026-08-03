# release 按 provider 单文件维护 + github-like 共享核保留

`src/release/mod.rs` 单文件 422 行混排四家 provider，与 `src/plugins/` 的每生态一文件惯例（ADR-0007/0010）不对称。拆为每 provider 单文件；github / gitee / gitcode 三家 API 同形、原共享一个 `create_github_like_release` 实现，拆分中保留该共享核而非复制三份。

## Decisions

- **每 provider 单文件**：`release/{github,gitee,gitcode,gitlab}.rs`。github-like 三家为薄文件（约 25 行），各持 base_url 与 token 注入形态（Bearer 头 / 请求体 `access_token` 字段 / query `access_token`）；gitlab 全特化（`PRIVATE-TOKEN` 头 + 项目 id 直查），`gitlab.host` 解析（严格 schema）作为 gitlab 专有知识随文件。
- **共享核保留**：`release/github_like.rs` 持请求体语义的单一事实源（name/tag_name v 前缀/body/target_commitish/prerelease 正则）与 releases 端点 URL——三家复用是领域事实（CONTEXT.md "平台 Release" 词条），不为"每文件自含"复制三份而引入漂移风险。
- **共享层分置**：`release/http.rs` 持四家共用原语——仓库信息解析与 HTTP 收发（resolve_owner_repo/agent/check_status/post_json）；mod.rs 持 Provider 枚举（跨模块词汇：CLI 入参、napi 边界、token 存储键，四家在此注册 parse/name/display/env_vars）、ReleaseError（含 `redact` 脱敏）、token 解析链与 create_release 分发。
- **测试镜像**：`tests/release.rs` → `tests/release/` 目录（对齐 tests/plugins/ 先例，ADR-0010）：main.rs 持手写 mock 线束与装配，token.rs 持 token 链纯测，每 provider 一个行为测试文件；生产入口与测试注入缝分层（`create` 用文件内 const 真实地址，`#[doc(hidden)] create_with_base` / `create_with_host` 可指 mock）。

## Considered Options

- **四家全独立（请求体复制三份）**：provider 间彻底解耦，但 name/tag_name/prerelease 等语义出现三份拷贝，改一处需同步三处——拒绝，漂移风险大于解耦收益。
- **trait 化注册（每文件 impl trait，通用引擎分发）**：扩展性最强，但四家两形态用 trait 属过度设计，且调用面与测试注入点全要改——拒绝。

## Consequences

- 新增 provider 的落点：加 `<provider>.rs`（薄文件或特化）+ `Provider` 枚举注册 + napi 薄导出（ADR-0016 形态）+ `tests/release/<provider>.rs`。
- github-like 请求体语义变更只动 `github_like.rs` 一处，三家同步生效；某家 API 与 github 分叉时，将该家从共享核迁出为全特化文件即可（薄文件边界已为这一天留好）。
- napi 四个导出、CLI、`Provider::parse` 签名与编排调用面不变——外部零感知。
