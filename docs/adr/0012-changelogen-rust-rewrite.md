# Changelog 使用面由 Rust 实现

changelog 生成所需的 git 历史读取、提交解析、仓库解析、Markdown 生成和配置加载由 Rust 实现，不依赖 changelogen 的运行时与传递依赖。生成过程保持确定性，不向第三方服务发送贡献者邮箱。

## Decisions

- **范围与位置**：changelog 编排与实现位于 `vbumpp-core` 的 `changelog/` 模块；通用 git 读取能力归 `git.rs`，提交解析能力归 `commits.rs`。不建立额外 crate。
- **单一配置源**：changelog 使用 ADR-0013 的统一配置文档；配置解析不跨语言边界。`changelog.config.*`、`package.json` 的 `changelog` 键和 `.env` 不参与加载。
- **编排**：Rust 完成 diff 读取、提交解析、`chore(deps)` 过滤、Markdown 生成、旧文件插入与写盘。只将实际 changelog 输出加入 git；是否 add/commit 跟随统一配置的 `commit` 开关，提交信息由 `changelog.commitMessage` 控制，默认 `chore: update {{output}}`。
- **Tag 与链接**：起点使用实际最近 tag，不硬编码 `v` 前缀；目标 tag 使用 `templates.tagBody` 渲染值。分组顺序、组内顺序、compare 链接、引用链接及版本节插入规则保持既定输出结构。
- **作者处理**：不调用 `ungh.cc` 解析 GitHub 用户名；`hideAuthorEmail` 默认 `true`，贡献者默认只显示名字。用户可见默认标题与贡献者节标题为英文，见 ADR-0017。
- **Gitmoji**：内建所需静态映射，按字面量匹配 code，大小写不敏感。
- **接口边界**：changelog 函数仅供 Rust 编排内部使用，不保留 `@vill-v/bumpp/changelog` 子路径或 changelog 专用 napi 导出。
- **测试**：Rust 单测与合成 git 仓库为主体；golden fixtures 固定输出结构和有意差异。

## Consequences

- changelog 生成全程无网络，产物不随第三方用户解析服务波动，也不泄露作者邮箱。
- npm 侧不依赖 changelogen、c12、ofetch、convert-gitmoji 等链路。
- `hideAuthorEmail` 默认开启；需要邮箱或本地化标题的用户必须显式配置。
