# vbumpp 文档网站（website/）

面向用户的产品文档站（ADR-0020）：fumadocs / Next.js 静态导出，纯中文、单版本，
部署 GitHub Pages（项目页子路径 `/bumpp`，deploy 工作流见 `.github/workflows/docs.yml`）。

- 内容：`content/docs/*.mdx`（导航序 `content/docs/meta.json`）
- 开发：`pnpm dev`；构建：`pnpm build`（产物 `out/`，turbo 任务 `website#build`）
- 搜索：fumadocs 内置 Orama 静态搜索（默认 multilingual tokenizer，零配置支持中文）
- 与 `docs/` 的分工：本目录是用户文档；`docs/` 是工程内部文档（ADR、agent 约定）
