// 一次性生成器：convert-gitmoji@0.1.5 表 → changelog/gitmoji.rs（dev-only，勿提交产物依赖）
import { writeFileSync } from 'node:fs'
import { gitmojis } from '../../../../node_modules/.pnpm/convert-gitmoji@0.1.5/node_modules/convert-gitmoji/dist/index.mjs'

const entries = Object.entries(gitmojis)
const table = entries.map(([k, v]) => `  ("${k}", "${v}"),`).join('\n')
const keys = entries.map(([k]) => k.replace(/[.+?^${}()|[\]\\]/g, '\\$&')).join('|')

const out = `//! gitmoji 数据表（ADR-0012）：convert-gitmoji@0.1.5 的 74 条官方映射原样内建。
//!
//! 原实现 \`convert(content, true)\`：全文本大小写不敏感替换 \`:code:\` 为
//! emoji + 尾随空格。原实现以未转义键名拼正则（\`+\` 等字符沦为量词），
//! 属潜在 bug；本实现按字面量转义匹配——对真实输入行为一致，病理输入
//! （如 \`:heavy_plusss_sign:\`）不再误中。

use std::sync::LazyLock;

use regex::Regex;

/// 官方 gitmoji 映射（\`:code:\` → unicode emoji），序与原包一致
static GITMOJIS: [(&str, &str); 74] = [
${table}
];

/// 全键名 alternation（字面量转义，大小写不敏感）
static GITMOJI_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)${keys}").unwrap());

/// convert-gitmoji \`convert(content, true)\`：\`:code:\` → emoji + 尾随空格
pub fn convert_gitmoji(content: &str) -> String {
  GITMOJI_RE
    .replace_all(content, |caps: &regex::Captures| {
      let code = caps.get(0).unwrap().as_str().to_ascii_lowercase();
      let emoji = GITMOJIS
        .iter()
        .find(|(k, _)| *k == code)
        .map(|(_, v)| v)
        .expect("正则命中的键必在表内");
      format!("{emoji} ")
    })
    .into_owned()
}
`

writeFileSync(new URL('../../src/changelog/gitmoji.rs', import.meta.url), out)
console.log(`written ${entries.length} entries`)
