/**
 * 发版包 LICENSE 一致性校验（COL-24）：各发包目录的 LICENSE 必须与根 LICENSE
 * 逐字节一致——MIT 要求软件副本携带版权与许可文本，发版包即"副本"的载体。
 * 新增发版包时忘记放置副本，或根 LICENSE 更新后未同步，都会在此报错。
 *
 * 用法：pnpm check:licenses（CI 的 test job 同步执行）
 */
import { readdirSync, readFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
const rootLicense = readFileSync(join(root, 'LICENSE'), 'utf8')

const drifts = []
for (const entry of readdirSync(join(root, 'npm'), { withFileTypes: true })) {
  if (!entry.isDirectory()) continue
  const pkgDir = join(root, 'npm', entry.name)
  // 仅校验含 package.json 的发版包目录
  if (!existsSync(join(pkgDir, 'package.json'))) continue
  const licensePath = join(pkgDir, 'LICENSE')
  if (!existsSync(licensePath)) {
    drifts.push(`npm/${entry.name}: 缺少 LICENSE 副本`)
    continue
  }
  if (readFileSync(licensePath, 'utf8') !== rootLicense) {
    drifts.push(`npm/${entry.name}: LICENSE 与根不一致`)
  }
}

if (drifts.length > 0) {
  console.error('发版包 LICENSE 漂移：')
  for (const d of drifts) console.error(`  - ${d}`)
  console.error('请从根目录复制：cp LICENSE <pkg>/LICENSE')
  process.exit(1)
}
console.log('all package LICENSE files in sync')
