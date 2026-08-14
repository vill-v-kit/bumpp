import { createMDX } from 'fumadocs-mdx/next';
import type { NextConfig } from 'next';

const withMDX = createMDX();

const config: NextConfig = {
  output: 'export',
  reactStrictMode: true,
  // GitHub Pages 项目页子路径（ADR-0020）：vill-v-kit.github.io/bumpp
  basePath: '/bumpp',
  images: {
    // 静态导出无服务端图片优化管线
    unoptimized: true,
  },
  // wterm 组件包（wterm.dev 官方指引）：CSS 子路径指向 src/，需转译处理
  transpilePackages: ['@wterm/dom', '@wterm/react'],
};

export default withMDX(config);
