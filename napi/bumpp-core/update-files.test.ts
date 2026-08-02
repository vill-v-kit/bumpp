import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { versionBump } from 'bumpp'
import { afterEach, expect, it } from 'vitest'
import { updateFiles } from './index.js'

let dirs: string[] = []

/** 建两个内容完全相同的临时目录：一个跑上游 versionBump，一个跑 Rust updateFiles */
const twinDirs = (files: Record<string, string>): [string, string] => {
  const pair = [mkdtempSync(join(tmpdir(), 'bumpp-up-')), mkdtempSync(join(tmpdir(), 'bumpp-rs-'))]
  dirs.push(...pair)
  for (const dir of pair) {
    for (const [name, content] of Object.entries(files)) {
      writeFileSync(join(dir, name), content)
    }
  }
  return pair as [string, string]
}

const read = (dir: string, name: string): string => readFileSync(join(dir, name), 'utf8')

const runUpstream = async (dir: string, files: string[], current = '1.0.0', next = '2.0.0') => {
  const results = await versionBump({
    release: next,
    currentVersion: current,
    files,
    cwd: dir,
    commit: false,
    tag: false,
    push: false,
    confirm: false,
    printCommits: false,
  })
  return { updatedFiles: results.updatedFiles, skippedFiles: results.skippedFiles }
}

afterEach(() => {
  dirs.forEach((dir) => rmSync(dir, { recursive: true, force: true }))
  dirs = []
})

const parity = async (
  files: Record<string, string>,
  targets: string[],
  current = '1.0.0',
  next = '2.0.0',
) => {
  const [upDir, rsDir] = twinDirs(files)
  const upstream = await runUpstream(upDir, targets, current, next)
  const ours = updateFiles(targets, rsDir, current, next)
  // 路径列表：上游用 upDir 前缀，我们用 rsDir，归一化后比较。
  // 列表顺序跨平台不稳定（上游 glob 在不同平台的排序行为不同），按集合比较；
  // 逐文件内容与格式才是 parity 主体（下方逐字节断言）
  const normalize = (list: string[], dir: string) =>
    list.map((p) => p.replace(dir, '<DIR>')).sort()
  expect(normalize(ours.updatedFiles, rsDir)).toEqual(normalize(upstream.updatedFiles, upDir))
  expect(normalize(ours.skippedFiles, rsDir)).toEqual(normalize(upstream.skippedFiles, upDir))
  // 文件内容逐字节一致
  for (const name of Object.keys(files)) {
    expect(read(rsDir, name), `file ${name}`).toBe(read(upDir, name))
  }
}

it('package.json 保格式更新，与上游逐字节一致', async () => {
  await parity(
    {
      'package.json':
        '{\n    "name": "demo",\n    "version": "1.0.0",\n    "description": "d",\n    "private": true\n}\n',
    },
    ['package.json'],
  )
})

it('package-lock.json 嵌套 version 更新，与上游逐字节一致', async () => {
  await parity(
    {
      'package.json': '{\n  "version": "1.0.0"\n}\n',
      'package-lock.json':
        '{\n  "name": "demo",\n  "version": "1.0.0",\n  "lockfileVersion": 3,\n  "packages": {\n    "": {\n      "name": "demo",\n      "version": "1.0.0"\n    },\n    "node_modules/dep": {\n      "version": "1.0.0"\n    }\n  }\n}\n',
    },
    ['package-lock.json'],
  )
})

it('文本文件模板替换（v 前缀 / 词边界），与上游逐字节一致', async () => {
  await parity(
    {
      'package.json': '{\n  "version": "1.0.0"\n}\n',
      'CHANGELOG.md':
        '## v1.0.0\n\nChanges since 1.0.0:\n- pin 11.0.0 stays\n- foo1.0.0bar stays\n- 1.0.0-beta.1 context\n',
    },
    ['CHANGELOG.md'],
  )
})

it('跳过场景一致：版本已是最新 / 文本未含旧版本', async () => {
  await parity(
    { 'package.json': '{\n  "version": "2.0.0"\n}\n', 'README.md': '# demo\n' },
    ['package.json', 'README.md'],
  )
})

it('坏 JSON 的 manifest 跳过且批次继续，与上游一致', async () => {
  await parity(
    { 'package.json': '{ not json', 'other.txt': 'at 1.0.0\n' },
    ['package.json', 'other.txt'],
  )
})

it('预发行版本号的文本替换与上游一致', async () => {
  await parity(
    { 'package.json': '{\n  "version": "1.0.0-beta.1"\n}\n', 'a.txt': 'now at 1.0.0-beta.1!\n' },
    ['a.txt'],
    '1.0.0-beta.1',
    '1.0.0-beta.2',
  )
})
