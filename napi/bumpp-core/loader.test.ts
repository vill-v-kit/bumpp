import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'
import { afterEach, expect, it } from 'vitest'
import pkg from './package.json' with { type: 'json' }

let dirs: string[] = []

/** 隔离目录：只有 loader 与（真实的）package.json，无平台包可解析、无本地 .node */
const setupIsolatedDir = (): string => {
  const dir = mkdtempSync(join(tmpdir(), 'bumpp-loader-'))
  dirs.push(dir)
  cpSync(new URL('./index.js', import.meta.url), join(dir, 'index.js'))
  cpSync(new URL('./package.json', import.meta.url), join(dir, 'package.json'))
  // 探针：在干净子进程中 import loader（只保留 PATH，杜绝父进程 NODE_PATH / 模块缓存干扰）
  writeFileSync(
    join(dir, 'probe.mjs'),
    "try {\n  await import('./index.js')\n  console.log('RESOLVED')\n} catch (err) {\n  console.error(err.message)\n  process.exit(1)\n}\n",
  )
  return dir
}

const importIsolated = (dir: string) =>
  spawnSync(process.execPath, [join(dir, 'probe.mjs')], {
    cwd: dir,
    env: { PATH: process.env.PATH },
    encoding: 'utf8',
  })

// 平台清单的唯一真相源是 optionalDependencies 声明（loader 同源推导，ADR-0029），
// 不再维护硬编码副本——新增平台包只需改 package.json
const SUPPORTED = Object.keys(pkg.optionalDependencies).map((name) =>
  name.replace('@vill-v/bumpp-core-', ''),
)

afterEach(() => {
  dirs.forEach((dir) => rmSync(dir, { recursive: true, force: true }))
  dirs = []
})

it('无匹配平台包与本地产物时，报错明确列出已支持平台', () => {
  const dir = setupIsolatedDir()
  const result = importIsolated(dir)
  expect(result.status).toBe(1)
  expect(result.stderr).toMatch(/Supported platforms/)
  for (const triple of SUPPORTED) {
    expect(result.stderr).toContain(triple)
  }
})

it('平台包版本不匹配时报可读错误', () => {
  // 伪造一个版本不一致的“平台包”，经 node_modules 解析
  const dir = setupIsolatedDir()
  const fakePkg = join(dir, 'node_modules/@vill-v/bumpp-core-darwin-arm64')
  mkdirSync(fakePkg, { recursive: true })
  writeFileSync(
    join(fakePkg, 'package.json'),
    '{ "name": "@vill-v/bumpp-core-darwin-arm64", "version": "0.0.0", "main": "index.js" }',
  )
  writeFileSync(join(fakePkg, 'index.js'), 'module.exports = {}')
  const expected = process.platform === 'darwin' && process.arch === 'arm64' ? /version mismatch/ : /Supported platforms/
  const result = importIsolated(dir)
  expect(result.status).toBe(1)
  expect(result.stderr).toMatch(expected)
})
