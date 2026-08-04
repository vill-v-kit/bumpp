import { source } from '@/lib/source';
import { siteUrl } from '@/lib/shared';
import { llms } from 'fumadocs-core/source';

export const revalidate = false;

export function GET() {
  // 站内链接补全为绝对 URL——读者是站外 LLM，/docs 相对路径缺 basePath 无法解析
  return new Response(llms(source).index().replaceAll('](/', `](${siteUrl}/`));
}
