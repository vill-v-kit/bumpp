import { createMDX } from 'fumadocs-mdx/next';

const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
  output: 'export',
  reactStrictMode: true,
  // GitHub Pages 项目页子路径（ADR-0020）：vill-v-kit.github.io/bumpp
  basePath: '/bumpp',
  images: {
    // 静态导出无服务端图片优化管线
    unoptimized: true,
  },
};

export default withMDX(config);
