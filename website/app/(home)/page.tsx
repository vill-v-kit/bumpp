import Link from 'next/link';
import { siGithub } from 'simple-icons';
import { PMIcon } from '@/components/pm-tabs';
import { ScrollyDemo } from './scrolly-demo';

const upstream = [
  {
    href: 'https://github.com/antfu/bumpp',
    repo: 'antfu/bumpp',
    desc: '版本号更新与 git 发版流程的语义参考',
  },
  {
    href: 'https://github.com/unjs/changelogen',
    repo: 'unjs/changelogen',
    desc: 'changelog 生成的语义参考',
  },
];

export default function HomePage() {
  return (
    <div className="flex flex-1 flex-col">
      <div className="mx-auto w-full max-w-4xl px-6 py-16">
        <Link
          href="/docs/migration-v6"
          className="mb-6 inline-flex w-fit items-center gap-2 rounded-full border border-fd-border px-3 py-1 text-sm text-fd-muted-foreground transition-colors hover:bg-fd-accent hover:text-fd-foreground"
        >
          <span className="size-1.5 rounded-full bg-fd-primary" />
          v6 · Rust 重写 →
        </Link>
        <h1 className="text-5xl sm:text-6xl font-bold tracking-tight mb-4">
          vbumpp
        </h1>
        <p className="text-lg text-fd-muted-foreground mb-8 max-w-2xl">
          遵循 semver 的版本发布工具——
          <span className="text-fd-primary font-medium">一条命令</span>
          完成版本号更新、CHANGELOG 生成、git tag / push，并在 GitHub / GitLab /
          Gitee / GitCode 上创建 Release。
        </p>
        <div className="flex gap-3">
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
      </div>

      {/* 滚动演示区（ADR-0036）：sticky 终端四步 scrollytelling；
          lg 断点以下与 prefers-reduced-motion 降级为静态终态卡片 */}
      <ScrollyDemo />

      <div className="mx-auto w-full max-w-4xl px-6 py-16">
        <h2 className="text-xl font-semibold mb-4">语义参考</h2>
        <div className="grid gap-4 sm:grid-cols-2">
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
      </div>
    </div>
  );
}
