#!/usr/bin/env node
/**
 * schema 产物再生（COL-104）：调 `vbumpp schema` stdout 通路（ADR-0037），
 * 再生提交进仓库的两处 schema JSON 产物：
 *
 *   npm/bump/vbumpprc.schema.json        npm 包内副本（用户本地引用、cargo
 *                                        渠道用户亦有文件可指）
 *   website/public/vbumpprc.schema.json  website 静态导出（Pages 规范 URL
 *                                        https://vill-v-kit.github.io/bumpp/vbumpprc.schema.json，
 *                                        内容随发版更新、地址不变）
 *
 * 二进制解析顺序：VBUMPP_BIN 环境覆盖 > target/release/vbumpp >
 * target/debug/vbumpp。均不存在即报错提示构建——脚本不代为构建；ci.yml
 * test 腿在调用前显式 `cargo build -p vbumpp`（cargo test 对无集成测试的
 * bin crate 不产出独立 bin，只有 deps/ 测试体）。
 *
 * 用法：node scripts/regen-schema.ts
 *
 * 测试 stub（scripts/regen-schema.test.ts）：VBUMPP_BIN 指向 stub 可执行
 * （真实再生依赖 cargo 构建的二进制，契约测试不进那条路）。
 *
 * 退出码契约：0 再生成功；1 二进制缺失 / 调用失败 / 输出非纯 JSON；
 * 2 环境错误（不在 git 仓库内）。
 */
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

const ARTIFACTS = ['npm/bump/vbumpprc.schema.json', 'website/public/vbumpprc.schema.json']

let root: string
try {
  root = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim()
} catch {
  console.error('error: 不在 git 仓库内——产物落点按仓库根解析')
  process.exit(2)
}

function resolveBinary(): string | null {
  if (process.env.VBUMPP_BIN) return process.env.VBUMPP_BIN
  const name = `vbumpp${process.platform === 'win32' ? '.exe' : ''}`
  for (const profile of ['release', 'debug']) {
    const bin = join(root, 'target', profile, name)
    if (existsSync(bin)) return bin
  }
  return null
}

const bin = resolveBinary()
if (!bin) {
  console.error(
    'error: 找不到可用的 vbumpp 二进制（target/release|debug 均无）——先构建：cargo build --release -p vbumpp',
  )
  process.exit(1)
}

// stdout 通路采集：纯 JSON 是契约，解析失败即产物不可信，拒绝落盘
let text: string
try {
  text = execFileSync(bin, ['schema'], { encoding: 'utf8' })
} catch (err) {
  const e = err as Error & { status?: number | null }
  console.error(`error: ${bin} schema 调用失败（exit ${e.status ?? 'signal'}）`)
  process.exit(1)
}

const content = `${text.trimEnd()}\n`
try {
  JSON.parse(content)
} catch {
  console.error(`error: ${bin} schema 的 stdout 不是纯 JSON——混入了其他打印，产物再生中止`)
  process.exit(1)
}

for (const rel of ARTIFACTS) {
  const target = join(root, rel)
  mkdirSync(dirname(target), { recursive: true })
  writeFileSync(target, content)
  console.log(`schema artifact written: ${rel}`)
}
