import { loader } from 'fumadocs-core/source';
import type { LoaderPlugin } from 'fumadocs-core/source';
import { lucideIconsPlugin } from 'fumadocs-core/source/plugins/lucide-icons';
import { docsContentRoute, docsImageRoute, docsRoute } from './shared';
import { defineDocs } from 'fumadocs-mdx/macro';
import { metaSchema, pageSchema } from 'fumadocs-core/source/schema';
import { createElement } from 'react';
import { siNpm, siRust } from 'simple-icons';
import { PMIcon } from '@/components/pm-tabs';

const docs = defineDocs({
  dir: 'content/docs',
  docs: {
    schema: pageSchema,
    postprocess: {
      includeProcessedMarkdown: true,
    },
  },
  meta: {
    schema: metaSchema,
  },
});

// 品牌图标（lucide 无收录）：frontmatter / meta.json 的 icon 字符串在此映射
// ——与官方 lucideIconsPlugin 同一机制（fumadocs iconPlugin 形状）
const BRAND_ICONS = {
  Npm: { path: siNpm.path, title: 'npm' },
  CratesIo: { path: siRust.path, title: 'crates.io' },
  Cargo: { path: siRust.path, title: 'Rust' },
} as const;

function brandIconsPlugin(): LoaderPlugin {
  const replace = <T extends { icon?: unknown }>(node: T): T => {
    const brand =
      typeof node.icon === 'string'
        ? BRAND_ICONS[node.icon as keyof typeof BRAND_ICONS]
        : undefined;
    if (brand) node.icon = createElement(PMIcon, { path: brand.path, title: brand.title });
    return node;
  };
  return {
    name: 'vbumpp:brand-icons',
    transformPageTree: {
      file: replace,
      folder: replace,
      separator: replace,
    },
  } as LoaderPlugin;
}

// See https://fumadocs.dev/docs/headless/source-api for more info
export const source = loader({
  baseUrl: docsRoute,
  source: docs.toFumadocsSource(),
  plugins: [brandIconsPlugin(), lucideIconsPlugin()],
});

export function getPageImageUrl(page: (typeof source)['$inferPage']) {
  const segments = [...page.slugs, 'image.png'];

  return {
    segments,
    url: '/' + [page.locale, ...docsImageRoute.split('/'), ...segments].filter(Boolean).join('/'),
  };
}

export function getPageMarkdownUrl(page: (typeof source)['$inferPage']) {
  const segments = [...page.slugs, 'content.md'];

  return {
    segments,
    url: '/' + [page.locale, ...docsContentRoute.split('/'), ...segments].filter(Boolean).join('/'),
  };
}

export async function getLLMText(page: (typeof source)['$inferPage']) {
  const processed = await page.data.getText('processed');

  return `# ${page.data.title} (${page.url})

${processed}`;
}
