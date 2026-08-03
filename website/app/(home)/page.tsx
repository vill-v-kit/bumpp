import Link from 'next/link';
import {
  Code2,
  FileText,
  Globe,
  Layers,
  Package,
  ShieldCheck,
  Zap,
} from 'lucide-react';
import { siGithub, siKimi, siRust } from 'simple-icons';
import { PMIcon } from '@/components/pm-tabs';

const features = [
  {
    icon: <Zap className="size-5" />,
    title: '一条命令发版',
    desc: '版本号、CHANGELOG、commit / tag / push 一次完成，告别手工步骤',
  },
  {
    icon: <FileText className="size-5" />,
    title: 'CHANGELOG 自动生成',
    desc: '从 conventional commits 生成结构化变更记录，分组标题可定制',
  },
  {
    icon: <Layers className="size-5" />,
    title: 'monorepo 整树递归',
    desc: '一个 -r 同步更新所有包的版本号，lockfile 也可自动刷新',
  },
  {
    icon: <Globe className="size-5" />,
    title: '四家平台 Release',
    desc: 'GitHub / GitLab / Gitee / GitCode 自动创建，失败可单独重试',
  },
  {
    icon: <ShieldCheck className="size-5" />,
    title: 'token 加密存储',
    desc: '一次录入本机加密保存，CI 环境直接用各家环境变量',
  },
  {
    icon: <Package className="size-5" />,
    title: 'node 与 cargo 双生态',
    desc: 'package.json、Cargo.toml 等清单文件结构化更新',
  },
];

const upstream = [
  {
    href: 'https://github.com/antfu/bumpp',
    repo: 'antfu/bumpp',
    desc: '版本号更新与 git 发版流程——bump 语义的改写来源',
  },
  {
    href: 'https://github.com/unjs/changelogen',
    repo: 'unjs/changelogen',
    desc: 'changelog 生成——变更记录语义的改写来源',
  },
];

const builtWith = [
  {
    icon: <PMIcon path={siRust.path} title="Rust" />,
    title: 'Rust 重写',
    desc: '核心引擎纯 Rust，启动快、行为一致',
  },
  {
    icon: <Code2 className="size-5" />,
    title: 'ZCode',
    desc: '全程 vibe coding 的编码代理',
  },
  {
    icon: <PMIcon path={siKimi.path} title="Kimi" />,
    title: 'Kimi K3',
    desc: '驱动全部代码编写的模型',
  },
];

export default function HomePage() {
  return (
    <div className="flex flex-col flex-1 px-6 py-16 mx-auto w-full max-w-4xl">
      <h1 className="text-4xl font-bold mb-3">vbumpp</h1>
      <p className="text-lg text-fd-muted-foreground mb-6">
        遵循 semver 的版本发布工具——一条命令完成版本号更新、CHANGELOG 生成、git
        tag / push，并在 GitHub / GitLab / Gitee / GitCode 上创建 Release。
      </p>
      <div className="flex gap-3 mb-16">
        <Link
          href="/docs/quick-start"
          className="rounded-lg bg-fd-primary text-fd-primary-foreground px-5 py-2.5 font-medium transition-opacity hover:opacity-90"
        >
          快速开始
        </Link>
        <Link
          href="/docs"
          className="rounded-lg border border-fd-border px-5 py-2.5 font-medium transition-colors hover:bg-fd-accent"
        >
          文档
        </Link>
      </div>

      <h2 className="text-xl font-semibold mb-4">优势</h2>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 mb-16">
        {features.map((f) => (
          <div key={f.title} className="rounded-lg border border-fd-border p-4">
            <div className="text-fd-primary mb-2">{f.icon}</div>
            <div className="font-medium mb-1">{f.title}</div>
            <div className="text-sm text-fd-muted-foreground">{f.desc}</div>
          </div>
        ))}
      </div>

      <h2 className="text-xl font-semibold mb-4">改写自</h2>
      <div className="grid gap-4 sm:grid-cols-2 mb-16">
        {upstream.map((u) => (
          <a
            key={u.repo}
            href={u.href}
            target="_blank"
            rel="noreferrer"
            className="rounded-lg border border-fd-border p-4 transition-colors hover:bg-fd-accent"
          >
            <div className="flex items-center gap-2 font-medium mb-1">
              <PMIcon path={siGithub.path} title="GitHub" />
              {u.repo} ↗
            </div>
            <div className="text-sm text-fd-muted-foreground">{u.desc}</div>
          </a>
        ))}
      </div>

      <h2 className="text-xl font-semibold mb-4">构建</h2>
      <div className="grid gap-4 sm:grid-cols-3">
        {builtWith.map((b) => (
          <div key={b.title} className="rounded-lg border border-fd-border p-4">
            <div className="text-fd-primary mb-2">{b.icon}</div>
            <div className="font-medium mb-1">{b.title}</div>
            <div className="text-sm text-fd-muted-foreground">{b.desc}</div>
          </div>
        ))}
      </div>
    </div>
  );
}
