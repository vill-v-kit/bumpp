import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, expect, it } from 'vitest'
import { runNpmScript } from './index.js'

let dirs: string[] = []

const tempDir = (manifest: string): string => {
  const dir = mkdtempSync(join(tmpdir(), 'bumpp-npm-'))
  dirs.push(dir)
  writeFileSync(join(dir, 'package.json'), manifest)
  return dir
}

afterEach(() => {
  dirs.forEach((dir) => rmSync(dir, { recursive: true, force: true }))
  dirs = []
})

it('脚本存在时执行并产出事件，ignoreScripts 跳过', () => {
  const dir = tempDir(
    '{\n  "version": "1.0.0",\n  "scripts": {\n    "version": "node -e \\"require(\'fs\').writeFileSync(\'ran.txt\',\'\')\\""\n  }\n}\n',
  )
  expect(runNpmScript(dir, 'version', true)).toBeNull()
  expect(existsSync(join(dir, 'ran.txt'))).toBe(false)
  const outcome = runNpmScript(dir, 'version', false)
  expect(outcome?.event).toBe('npm script')
  expect(outcome?.script).toBe('version')
  expect(existsSync(join(dir, 'ran.txt'))).toBe(true)
})

it('脚本失败不传播（上游 parity），不存在的脚本返回 null', () => {
  const dir = tempDir(
    '{\n  "version": "1.0.0",\n  "scripts": { "postversion": "exit 1" }\n}\n',
  )
  expect(runNpmScript(dir, 'postversion', false)?.event).toBe('npm script')
  expect(runNpmScript(dir, 'preversion', false)).toBeNull()
})

it('非 manifest 的 package.json 不执行脚本（isManifest 门）', () => {
  const dir = tempDir('{\n  "version": 42,\n  "scripts": { "version": "exit 0" }\n}\n')
  expect(runNpmScript(dir, 'version', false)).toBeNull()
})

it('falsy 脚本值不执行（上游 Boolean(scripts[x]) 语义）', () => {
  for (const value of ['""', 'null', 'false', '0']) {
    const dir = tempDir(`{\n  "version": "1.0.0",\n  "scripts": { "version": ${value} }\n}\n`)
    expect(runNpmScript(dir, 'version', false), `scripts.version = ${value}`).toBeNull()
  }
})
