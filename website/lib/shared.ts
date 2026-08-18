export const appName = 'vbumpp';
export const docsRoute = '/docs';
export const docsImageRoute = '/og/docs';
export const docsContentRoute = '/llms.mdx/docs';

// 与 next.config.ts 的 basePath 同步（public 资源手动加前缀用）
export const basePath = '/bumpp';

// 站点对外完整地址（llms.txt 等站外 AI 消费文件需要可抓取的绝对 URL）
export const siteUrl = 'https://vill-v-kit.github.io/bumpp';

// 「在 GitHub 上编辑」链接指向部署源仓库
export const gitConfig = {
  user: 'vill-v-kit',
  repo: 'bumpp',
  branch: 'main',
};
