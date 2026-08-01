import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { afterEach, expect, it } from 'vitest'
import pkg from './package.json' with { type: 'json' }

let dirs: string[] = []

/** 隔离目录：只有 loader 与（真实的）package.json，无平台包可解析、无本地 .node */
const setupIsolatedDir = (extraFiles: Record<string, string> = {}): string => {
  const dir = mkdtempSync(join(tmpdir(), 'bumpp-loader-'))
  dirs.push(dir)
  cpSync(new URL('./index.js', import.meta.url), join(dir, 'index.js'))
  cpSync(new URL('./package.json', import.meta.url), join(dir, 'package.json'))
  for (const [name, content] of Object.entries(extraFiles)) {
    writeFileSync(join(dir, name), content)
  }
  return dir
}

const SUPPORTED = ['darwin-arm64', 'linux-x64-gnu', 'linux-arm64-gnu', 'win32-x64-msvc', 'win32-arm64-msvc']

afterEach(() => {
  dirs.forEach((dir) => rmSync(dir, { recursive: true, force: true }))
  dirs = []
})

it('无匹配平台包与本地产物时，报错明确列出已支持平台', async () => {
  const dir = setupIsolatedDir()
  const result = import(pathToFileURL(join(dir, 'index.js')).href)
  await expect(result).rejects.toThrowError(/已支持平台/)
  for (const triple of SUPPORTED) {
    await expect(result).rejects.toThrowError(new RegExp(triple))
  }
})

it('平台包版本不匹配时报可读错误', async () => {
  // 伪造一个版本不一致的“平台包”，经 node_modules 解析
  const dir = setupIsolatedDir()
  const fakePkg = join(dir, 'node_modules/@vill-v/bumpp-core-darwin-arm64')
  mkdirSync(fakePkg, { recursive: true })
  writeFileSync(
    join(fakePkg, 'package.json'),
    '{ "name": "@vill-v/bumpp-core-darwin-arm64", "version": "0.0.0", "main": "index.js" }',
  )
  writeFileSync(join(fakePkg, 'index.js'), 'module.exports = {}')
  const expected = process.platform === 'darwin' && process.arch === 'arm64' ? /版本不匹配/ : /已支持平台/
  await expect(import(pathToFileURL(join(dir, 'index.js')).href)).rejects.toThrowError(expected)
})

it('loader 的平台清单与 optionalDependencies 声明一致', () => {
  const declared = Object.keys(pkg.optionalDependencies).map((name) =>
    name.replace('@vill-v/bumpp-core-', ''),
  )
  expect(declared.sort()).toEqual([...SUPPORTED].sort())
})
