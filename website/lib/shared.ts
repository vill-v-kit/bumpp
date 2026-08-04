export const appName = 'vbumpp';
export const docsRoute = '/docs';
export const docsImageRoute = '/og/docs';
export const docsContentRoute = '/llms.mdx/docs';

// 与 next.config.mjs 的 basePath 同步（public 资源手动加前缀用）
export const basePath = '/bumpp';

// 「在 GitHub 上编辑」链接指向部署源仓库（ADR-0020）
export const gitConfig = {
  user: 'vill-v-kit',
  repo: 'bumpp',
  branch: 'main',
};
