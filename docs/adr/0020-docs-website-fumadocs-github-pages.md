# 文档网站：fumadocs 落地 website/ + 纯中文 + GitHub Pages 部署

项目缺少面向用户的产品文档（README 与工程内部 `docs/` 之外无系统文档）。引入 fumadocs（Next.js 文档框架）生成文档网站，落新顶层目录 `website/`，纯中文、单版本，静态导出部署至 GitHub Pages；国内访问不稳定的取舍经评估后明确接受，不做国内镜像。

## Decisions

- **落点 `website/`**：fumadocs Next.js 应用（MDX 内容在 `website/content/docs/`），与 `docs/` 工程内部文档（ADR、agent 约定、迁移指南源稿）物理分离——用户文档与内部文档是两类受众。pnpm-workspace 以精确名 `website` 加入（不进既有 `napi/*` / `npm/*` glob）；cargo 三层布局零感知（glob 只扫 crates/napi/npm）。
- **纯中文、单版本**：与 README、迁移指南、目标受众一致；ADR-0017 的英文唯一规则管代码内用户可见字符串，不管文档站。后续需要再开 fumadocs i18n。文档单版本随最新 release，v5→v6 迁移指南覆盖历史，无多版本维护。
- **内容板块**（上线范围）：快速上手、CLI 参考（bump / release / token 子命令、flag 与退出码）、配置文件参考（`.vbumpprc.*` 四层合并全 schema）、平台 Release 指南（四 provider、token 解析链、自建 gitlab、release 重试）、v5→v6 迁移指南（`docs/migration-v6.md` 内容搬入）、外链区（导航栏图标链接：npmx.dev 的 `@vill-v/bumpp` 包页 + GitHub Releases——release notes 已迁至 GitHub 侧，Gitee 不再作为发布阵地）。
- **GitHub Pages 部署**：静态导出（`output: 'export'`），项目页子路径 `/bumpp`（`basePath`）；deploy 工作流挂 `origin`（github.com/vill-v-kit/bumpp，已有 Actions 基础设施）：版本 tag（`v*`）推送（随发版发布，与 ci.yml 触发同构）或手动 `workflow_dispatch` → 构建 → `upload-pages-artifact` → `deploy-pages`。搜索走 fumadocs 内置 Orama 静态搜索（离线可用，无外部服务）。

## Considered Options

- **`docs/` 进化为站点根**：少一个顶层目录，但工程文档与应用文件（package.json / next.config / app/）混居一根——拒绝，受众分离优先。
- **独立仓库**：本仓库零侵入，但配置参考 / 迁移指南与代码版本脱钩，发版时文档同步靠人肉——拒绝。
- **中英双语 / 纯英文**：双语维护成本翻倍（个人项目）；纯英文与中文受众脱节——均拒绝。
- **国内友好镜像（EdgeOne Pages / Cloudflare Pages / Gitee Pages 等）**：经评估动议后决策为不做——GitHub Pages 单点部署，国内访问不稳定的取舍明确接受；未来若受众扩大可追加镜像工作流（静态产物同一份，追加成本低）。

## Consequences

- AGENTS.md 仓库布局增记非 Rust 顶层目录（`website/` 与 `docs/` 的分工）；CONTEXT.md 新增「文档网站」术语。
- 用户可见能力（新 flag、新子命令、配置键）的文档化落点从此固定为 `website/content/docs/`；README 保持简介定位、细节向站点收敛。
- GitHub Pages 需在 origin 仓库设置中将 Pages source 切为 GitHub Actions（一次性手工步骤）。
- 新增部署面：deploy 工作流失败只影响站点发布，不影响发版主链路（ci.yml 独立）。
