import { source } from '@/lib/source';
import { DocsLayout } from 'fumadocs-ui/layouts/docs';
import { baseOptions } from '@/lib/layout.shared';

export default function Layout({ children }: LayoutProps<'/docs'>) {
  // 分组分隔符、图标、外链项全部由 content meta/frontmatter 声明 +
  // lib/source.ts 插件解析（fumadocs 原生机制，与官方文档同构），
  // 页面树零手工改造
  return (
    <DocsLayout tree={source.getPageTree()} {...baseOptions()}>
      {children}
    </DocsLayout>
  );
}
