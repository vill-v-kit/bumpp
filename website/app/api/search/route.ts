import { source } from '@/lib/source';
import { createFromSource } from 'fumadocs-core/search/server';

export const revalidate = false;

// 中文内容：不覆盖默认 tokenizer——fumadocs-core 16 / Orama v3 默认
// 'multilingual' tokenizer 零配置支持 CJK（覆盖为单一语言反会退化）
export const { staticGET: GET } = createFromSource(source);
