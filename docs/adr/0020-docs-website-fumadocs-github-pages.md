# 文档网站：Fumadocs + GitHub Pages

## Decisions

- 用户文档落在独立的 `website/` Fumadocs/Next.js 应用；内容位于 `website/content/docs/`，与工程内部 ADR、agent 约定和迁移指南源稿所在的 `docs/` 分离。
- `website` 以精确包名加入 pnpm workspace，不改变 Cargo 的 `crates/*`、`napi/*`、`npm/*` 分层。站点纯中文、单版本，随最新 release 更新；ADR-0017 的英文规则只约束代码内用户可见字符串。
- 首期覆盖快速上手、CLI/config 参考、平台 Release 指南、v5→v6 迁移指南和外链。README 保持简介定位，细节逐步收敛到站点。
- 站点使用 Next.js 静态导出，部署到 GitHub Pages 项目子路径 `/bumpp`。版本 tag `v*` 或手动 `workflow_dispatch` 触发独立的构建→上传 artifact→部署流程；搜索使用 Fumadocs 的静态 Orama，不依赖外部搜索服务。
- GitHub Pages 是唯一部署点；国内访问不稳定已接受，暂不维护镜像。

## Consequences

- 仓库布局必须持续保留 `website/` 与 `docs/` 的受众边界；用户可见 flag、子命令和配置键的详细文档落在 `website/content/docs/`。
- GitHub 仓库需一次性将 Pages source 设置为 GitHub Actions。站点部署失败不阻断 `ci.yml` 的构建与发布链路。
- 新增站点发布面时，保持静态产物与应用代码版本同步；需要多语言或多版本时另行决策。
