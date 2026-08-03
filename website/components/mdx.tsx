import defaultMdxComponents from 'fumadocs-ui/mdx';
import { Tab, Tabs } from 'fumadocs-ui/components/tabs';
import type { MDXComponents } from 'mdx/types';
import { PMTabsList } from './pm-tabs';

export function getMDXComponents(components?: MDXComponents) {
  return {
    ...defaultMdxComponents,
    // base-ui v16 默认组件集不含 Tabs（包管理器/系统切换等场景），显式注册
    Tabs,
    Tab,
    PMTabsList,
    ...components,
  } satisfies MDXComponents;
}

export const useMDXComponents = getMDXComponents;

declare global {
  type MDXProvidedComponents = ReturnType<typeof getMDXComponents>;
}
