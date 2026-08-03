/**
 * 发版包 LICENSE 一致性校验（COL-24）：各发包目录的 LICENSE 必须与根 LICENSE
 * 逐字节一致——MIT 要求软件副本携带版权与许可文本，发版包即"副本"的载体。
 * 新增发版包时忘记放置副本，或根 LICENSE 更新后未同步，都会在此报错。
 *
 * 扫描 `npm/`（面向用户的包）与 `napi/`（内部机制包，ADR-0005）两个发版目录。
 * 用法：pnpm check:licenses（CI 的 test job 同步执行）
 */
import { readdirSync, readFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
const rootLicense = readFileSync(join(root, 'LICENSE'), 'utf8')

const drifts = []
for (const scope of ['npm', 'napi']) {
  const scopeDir = join(root, scope)
  if (!existsSync(scopeDir)) continue
  for (const entry of readdirSync(scopeDir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue
    const pkgDir = join(scopeDir, entry.name)
    // 仅校验含 package.json 的发版包目录
    if (!existsSync(join(pkgDir, 'package.json'))) continue
    const licensePath = join(pkgDir, 'LICENSE')
    if (!existsSync(licensePath)) {
      drifts.push(`${scope}/${entry.name}: missing LICENSE copy`)
      continue
    }
    if (readFileSync(licensePath, 'utf8') !== rootLicense) {
      drifts.push(`${scope}/${entry.name}: LICENSE differs from the root one`)
    }
  }
}

if (drifts.length > 0) {
  console.error('package LICENSE drift detected:')
  for (const d of drifts) console.error(`  - ${d}`)
  console.error('copy from the repo root: cp LICENSE <pkg>/LICENSE')
  process.exit(1)
}
console.log('all package LICENSE files in sync')
