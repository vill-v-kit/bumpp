import { Provider } from '@/components/provider';
import type { Metadata } from 'next';
import './global.css';

export const metadata: Metadata = {
  // GitHub Pages 项目页：OG/Twitter 图片绝对 URL 的解析基准
  metadataBase: new URL('https://vill-v-kit.github.io/bumpp'),
  title: {
    default: 'vbumpp 文档',
    template: '%s | vbumpp 文档',
  },
  description:
    'vbumpp（@vill-v/bumpp）——遵循 semver 的版本发布工具：一条命令完成版本号更新、CHANGELOG 生成与多平台 Release 创建。',
};

export default function Layout({ children }: LayoutProps<'/'>) {
  // 不引 next/font/google：中文内容拉丁子集字体无意义，且消除构建期网络依赖
  return (
    <html lang="zh-CN" suppressHydrationWarning>
      <body className="flex flex-col min-h-screen">
        <Provider>{children}</Provider>
      </body>
    </html>
  );
}
