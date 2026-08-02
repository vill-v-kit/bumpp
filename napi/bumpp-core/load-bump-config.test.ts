import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { loadBumpConfig as upstreamLoadBumpConfig } from 'bumpp'
import { afterEach, expect, it } from 'vitest'
import { loadBumpConfig } from './index.js'

let dirs: string[] = []

const tempDir = (files: Record<string, string> = {}): string => {
  const dir = mkdtempSync(join(tmpdir(), 'bumpp-core-'))
  dirs.push(dir)
  for (const [name, content] of Object.entries(files)) {
    writeFileSync(join(dir, name), content)
  }
  return dir
}

afterEach(() => {
  dirs.forEach((dir) => rmSync(dir, { recursive: true, force: true }))
  dirs = []
})

it('无配置文件时与上游返回形状一致', async () => {
  const dir = tempDir()
  const upstream = await upstreamLoadBumpConfig(undefined, dir)
  expect(loadBumpConfig(undefined, dir)).toEqual(upstream)
})

it('JSON 配置合并结果与上游一致', async () => {
  const dir = tempDir({ 'bump.config.json': '{ "tag": false, "preid": "beta", "files": ["a.json"] }' })
  const upstream = await upstreamLoadBumpConfig(undefined, dir)
  expect(loadBumpConfig(undefined, dir)).toEqual(upstream)
})

it('overrides 合并优先级与上游一致（undefined 剥离）', async () => {
  const dir = tempDir({ 'bump.config.json': '{ "tag": false, "push": false }' })
  const overrides = { tag: true, push: undefined, files: ['b.json'] }
  const upstream = await upstreamLoadBumpConfig(overrides, dir)
  expect(loadBumpConfig(overrides, dir)).toEqual(upstream)
})

it('TS 配置报错，明确指出仅支持 JSON 配置', () => {
  const dir = tempDir({ 'bump.config.ts': 'export default { tag: false }' })
  expect(() => loadBumpConfig(undefined, dir)).toThrowError(/仅支持 JSON 配置/)
})

it('JSON 与 TS 并存时同样报错（不静默忽略 TS）', () => {
  const dir = tempDir({
    'bump.config.json': '{ "tag": false }',
    'bump.config.ts': 'export default {}',
  })
  expect(() => loadBumpConfig(undefined, dir)).toThrowError(/仅支持 JSON 配置/)
})

it('配置文件含 customVersion 时报错提示该选项已移除', () => {
  const dir = tempDir({ 'bump.config.json': '{ "customVersion": "1.2.3" }' })
  expect(() => loadBumpConfig(undefined, dir)).toThrowError(/customVersion/)
})
