/**
 * 平台包 .node 注入：调 `napi artifacts` 把汇聚的各平台 `.node`
 * 按文件名分发进 `napi/<triple>/` 平台包目录，并把 `index.js`/`index.d.ts` 归位 core。
 *
 * `napi artifacts` 要求 `napi.targets` 全部 target 的产物齐备（缺一即硬失败），
 * 故只能在汇聚全腿产物的 publish-npm job 执行，各 build 腿不适用。
 *
 * 用法：node scripts/collect-artifacts.ts [outputDir]
 *   outputDir 缺省 .artifacts——publish-npm job download-artifact
 *   merge-multiple 的落点（7 个 .node + index.js + index.d.ts 同目录平铺）
 */
import { spawnSync } from 'node:child_process'
import { copyFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
const napiBin = fileURLToPath(
  new URL('../napi/bumpp-core/node_modules/.bin/napi', import.meta.url),
)
const outputDir = process.argv[2] ?? '.artifacts'

// index.js/index.d.ts 由 napi CLI 腿同车捎带（zigbuild 腿不产出），先归位 core
copyFileSync(join(outputDir, 'index.js'), join(root, 'napi/bumpp-core/index.js'))
copyFileSync(join(outputDir, 'index.d.ts'), join(root, 'napi/bumpp-core/index.d.ts'))

const result = spawnSync(
  napiBin,
  [
    'artifacts',
    '--package-json-path',
    'napi/bumpp-core/package.json',
    '--npm-dir',
    'napi',
    '--output-dir',
    outputDir,
  ],
  { cwd: root, stdio: 'inherit' },
)
process.exit(result.status ?? 1)
