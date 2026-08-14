#!/usr/bin/env node
// 生成演示用预置加密 keyring（key.bin + tokens.bin）——token list 与 release
// dry-run 两段 fixture 的 token 来源（ADR-0036「预置加密 keyring」）。
//
// 与 crates/vbumpp-core/src/token.rs 落盘格式逐字节兼容：
//   key.bin   —— 32 字节 AES-256 密钥
//   tokens.bin —— magic "VBTK"(4B) | version 0x01(1B) | iv(12B) | authTag(16B) | ciphertext
//   明文为紧凑 JSON { "key": "plaintext-token", … }（键按字典序，对齐 serde_json
//   的 BTreeMap 序列化）
//
// 确定性：密钥与 IV 全部钉死（真实运行是随机 IV——演示产物要求字节级可复现，
// 且内容本来就是假 token，钉死 IV 无安全含义）。复跑产物字节一致。
//
// 用法：node make-demo-keyring.ts <输出目录>

import { createCipheriv } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2];
if (!dir) {
  console.error('usage: node make-demo-keyring.ts <输出目录>');
  process.exit(2);
}

// 钉死的密钥 / IV（32B / 12B）
const KEY = Buffer.from('vbumpp-demo-key-vbumpp-demo-key-', 'utf8');
const IV = Buffer.from('vbumpp-demo-', 'utf8');
if (KEY.length !== 32 || IV.length !== 12) {
  console.error('error: 内置密钥/IV 长度不符（32B / 12B）——脚本常量被改坏');
  process.exit(1);
}

// 假 token（永不出现在演示输出：list 只打键名，release 请求 URL 脱敏；
// 形状仿各家真实 token 前缀，一眼可辨是演示道具）
const TOKENS = {
  github: 'ghp_vbumppDemoFakeToken0000000000000000000',
  'gitlab@https://gitlab.com': 'glpat-vbumpp-demo-fake-token-0000000000',
};
// 键序对齐 Rust 侧 BTreeMap 字典序（@ 0x40 < 小写字母不影响：gitlab 是
// gitlab@… 的前缀，短串在前）
const json = JSON.stringify(
  Object.fromEntries(
    Object.entries(TOKENS).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)),
  ),
);

const cipher = createCipheriv('aes-256-gcm', KEY, IV);
const ciphertext = Buffer.concat([cipher.update(json, 'utf8'), cipher.final()]);
const blob = Buffer.concat([
  Buffer.from('VBTK', 'utf8'),
  Buffer.from([1]),
  IV,
  cipher.getAuthTag(),
  ciphertext,
]);

mkdirSync(dir, { recursive: true, mode: 0o700 });
writeFileSync(join(dir, 'key.bin'), KEY, { mode: 0o600 });
writeFileSync(join(dir, 'tokens.bin'), blob, { mode: 0o600 });
console.log(`ok: ${dir}/{key.bin,tokens.bin}`);
