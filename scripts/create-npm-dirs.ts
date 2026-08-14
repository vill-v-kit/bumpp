/**
 * 平台包目录生成（ADR-0029）：以 `napi/bumpp-core/package.json` 的 `napi.targets`
 * 为单一真相源，调 `napi create-npm-dirs` 在 `napi/<triple>/` 生成全部平台包
 * （package.json + README.md），再把根 LICENSE 同步进每个平台包目录——
 * create-npm-dirs 本身不写 LICENSE，而 MIT 要求发版副本携带许可文本
 * （check-licenses.ts 在校验侧守着同一约定）。
 *
 * 用法：pnpm create:npm-dirs（发版 CI 的 publish-npm job 与本地复现共用）
 *
 * 平台包目录 gitignore 不跟踪：fresh clone 无此目录，optionalDependencies 的
 * workspace:* 被 pnpm 静默跳过，loader 走本地 .node fallback（ADR-0029）。
 */
import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
const napiBin = fileURLToPath(new URL('../napi/bumpp-core/node_modules/.bin/napi', import.meta.url))

const result = spawnSync(
  napiBin,
  ['create-npm-dirs', '--package-json-path', 'napi/bumpp-core/package.json', '--npm-dir', 'napi'],
  { cwd: root, stdio: 'inherit' },
)
if (result.status !== 0) {
  process.exit(result.status ?? 1)
}

// create-npm-dirs 不产物 LICENSE：逐平台包目录同步根 LICENSE（与 check-licenses.ts 同口径）
const napiDir = join(root, 'napi')
for (const entry of readdirSync(napiDir, { withFileTypes: true })) {
  if (!entry.isDirectory() || entry.name === 'bumpp-core') continue
  const pkgDir = join(napiDir, entry.name)
  const manifestPath = join(pkgDir, 'package.json')
  if (!existsSync(manifestPath)) continue
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8')) as { name?: string }
  if (!manifest.name?.startsWith('@vill-v/bumpp-core-')) continue
  copyFileSync(join(root, 'LICENSE'), join(pkgDir, 'LICENSE'))
  console.log(`LICENSE synced: napi/${entry.name}`)
}
