import { TabsList, TabsTrigger } from 'fumadocs-ui/components/tabs';
import { Package, Zap } from 'lucide-react';
import { siBun, siNpm, siPnpm, siYarn } from 'simple-icons';
import type { ReactNode } from 'react';

/** 包管理器品牌图标（simple-icons path 数据，currentColor 单色随主题明暗） */
export function PMIcon({ path, title }: { path: string; title: string }) {
  return (
    <svg viewBox="0 0 24 24" role="img" aria-label={title} className="size-4 fill-current">
      <path d={path} />
    </svg>
  );
}

// simple-icons 无收录的包管理器（aube / nub）以 lucide 通用图标区分
const managers: { value: string; title: string; icon: ReactNode }[] = [
  { value: 'npm', title: 'npm', icon: <PMIcon path={siNpm.path} title="npm" /> },
  { value: 'yarn', title: 'yarn', icon: <PMIcon path={siYarn.path} title="yarn" /> },
  { value: 'pnpm', title: 'pnpm', icon: <PMIcon path={siPnpm.path} title="pnpm" /> },
  { value: 'bun', title: 'bun', icon: <PMIcon path={siBun.path} title="bun" /> },
  { value: 'aube', title: 'aube', icon: <Package className="size-4" aria-label="aube" /> },
  { value: 'nub', title: 'nub', icon: <Zap className="size-4" aria-label="nub" /> },
];

/**
 * 包管理器 Tab 头（MDX 全局组件）：`<Tabs defaultValue="npm"><PMTabsList />…`
 * 与 Tab 的 value 一一对应（npm / yarn / pnpm / bun / aube / nub，无空白字符，
 * escapeValue 恒等）
 */
export function PMTabsList() {
  return (
    <TabsList>
      {managers.map((m) => (
        <TabsTrigger key={m.value} value={m.value} className="gap-1.5">
          {m.icon}
          {m.title}
        </TabsTrigger>
      ))}
    </TabsList>
  );
}
