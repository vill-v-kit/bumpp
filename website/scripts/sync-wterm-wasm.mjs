// 把 @wterm/core 包内携带的 wterm.wasm 拷进 public/——wasm 经 basePath
// 显式加载（组件拼 `${basePath}/wterm.wasm`），不经包内 base64 内联默认。
// 挂点：postinstall（fresh clone 安装即就位）+ build/dev 前置（兜
// --ignore-scripts）。幂等：字节相同则跳过。
import { copyFileSync, existsSync, mkdirSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const websiteDir = join(dirname(fileURLToPath(import.meta.url)), '..');
const source = createRequire(join(websiteDir, 'package.json')).resolve(
  '@wterm/core/wasm',
);
const target = join(websiteDir, 'public', 'wterm.wasm');

if (!existsSync(source)) {
  console.error(`sync-wterm-wasm: source missing: ${source}`);
  process.exit(1);
}

mkdirSync(dirname(target), { recursive: true });
const same =
  existsSync(target) &&
  readFileSync(target).equals(readFileSync(source));
if (!same) {
  copyFileSync(source, target);
  console.log(`sync-wterm-wasm: copied ${source} -> ${target}`);
}
