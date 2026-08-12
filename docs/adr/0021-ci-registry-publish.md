# CI registry 发布：OIDC、幂等重跑与 build-cli 交接

网站安装指南承诺 npm 与 crates.io；发布规模为 13 个 npm 包（5 个用户包、1 个 core 包、7 个平台包——平台包目录由 `napi create-npm-dirs` 生成、不提交进 git，见 ADR-0029）以及 2 个公开 crate（`vbumpp-core`、`vbumpp`）；`napi/bumpp-core` 内部 Cargo crate 不属于这 2 个 registry crate。

## Decisions

- `publish-npm` 与 `publish-crates` 继续挂在 `.github/workflows/ci.yml`，由版本 tag 触发，并在 `test`、7-target `build` 与 `test-bindings` 全绿后并行执行。tag 推送即授权，不设置人工批准门；`test-bindings-arm64-smoke` 是额外信心检查，不阻断发布。
- npm 发布先下载 build 产物，由 `pnpm create:npm-dirs` 生成 7 个平台包目录（`napi.targets` 单一真相源，ADR-0029）、`napi artifacts` 把 7 个 `.node` 按名注入平台包并将 `index.d.ts` 归位 core；断言 tag 与根 workspace 版本一致，构建用户包，执行 pack/publint，再由 `pnpm publish -r --no-git-checks` 按拓扑发布。发布集合覆盖 13 个 npm 包，website 与根 workspace 的 private 包自动跳过。
- crates 发布先断言 tag 与 Cargo workspace 版本一致，执行 `cargo publish --dry-run`，按 `vbumpp-core` → `vbumpp` 顺序发布。workspace 依赖使用 `vbumpp-core = { path = "crates/vbumpp-core", version = ">=5.1, <7" }`，满足 registry 发布并避免逐次同步精确版本。
- OIDC 是当前唯一认证路线：npm job 具有 `id-token: write`，通过 npm trusted publishing；crates job 使用 `rust-lang/crates-io-auth-action` 换取短期 token。13 个 npm 包和 2 个 crate 必须先在 registry 存在并逐包配置 trusted publisher；全新包的首发需一次性使用长效 token 完成占位，随后配置 OIDC 并撤销 token。不得把临时 token 重新固化为常规路径。
- 两个发布 job 必须幂等：查询 `<package>@<version>` 或 crates.io 版本，已存在则跳过，查询失败则硬失败；部分失败后的唯一恢复方式是 GitHub Re-run failed jobs，未发布项继续发布，已发布项自动跳过。npm 可重复执行，crates.io 不可撤回，不做回滚或手动补发流程。
- 发布成功后由独立的 `build-cli` job 负责预编译 CLI 的 Release 资产交接（详见 ADR-0025）：构建 7 target，生成并校验 checksum，再以 `gh release upload --clobber` 追加到 bump 流程已创建的 GitHub Release。`build-cli` 失败表示 binstall 渠道不完整，但不回滚 npm/crates；重跑通过 clobber 收敛。

## Consequences

- 版本唯一来源仍是根 `package.json` / Cargo workspace 版本约定；新增平台包会由现有 workspace glob、pack、publint 和 license 检查覆盖。
- 发布认证、守卫和编排的实现位于 `scripts/` 与 CI；本 ADR 只记录决策，不复制脚本细节。任何发布 job 的变更都必须保留并行、可重跑和 build-cli 交接语义。
- `crates/vbumpp-core` 与 `crates/vbumpp` 维持可发布状态；`napi/bumpp-core` 仍是内部机制 crate，不得因 registry 计数而混入公开 crates 发布清单。
