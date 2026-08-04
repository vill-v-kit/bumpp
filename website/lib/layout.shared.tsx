import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { siNpm } from 'simple-icons';
import { Logo } from '@/components/logo';
import { PMIcon } from '@/components/pm-tabs';
import { appName, gitConfig } from './shared';

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <span className="inline-flex items-center gap-2">
          <Logo className="size-6" />
          {appName}
        </span>
      ),
    },
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
    links: [
      // 导航栏图标外链：npm 包页
      {
        type: 'icon',
        label: 'npm 包页（npmx）',
        text: 'npm 包页（npmx）',
        icon: <PMIcon path={siNpm.path} title="npm" />,
        url: 'https://npmx.dev/package/@vill-v/bumpp',
        external: true,
      },
    ],
  };
}
