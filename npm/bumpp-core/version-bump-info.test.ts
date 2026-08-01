import { execSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, expect, it } from 'vitest'
import { versionBumpInfo } from './index.js'

let dirs: string[] = []

const git = (cwd: string, args: string) =>
  execSync(`git ${args}`, { cwd, encoding: 'utf8' }).trim()

const initRepo = (version = '1.2.3'): string => {
  const dir = mkdtempSync(join(tmpdir(), 'bumpp-info-'))
  dirs.push(dir)
  git(dir, 'init -b main')
  git(dir, 'config user.email test@example.com')
  git(dir, 'config user.name Test')
  writeFileSync(join(dir, 'package.json'), `{\n  "version": "${version}"\n}\n`)
  git(dir, 'add .')
  git(dir, 'commit -m "chore: init"')
  return dir
}

afterEach(() => {
  dirs.forEach((dir) => rmSync(dir, { recursive: true, force: true }))
  dirs = []
})

it('release type：计算新版本并返回上游 state 形状', async () => {
  const dir = initRepo()
  const { state } = await versionBumpInfo({ release: 'major', cwd: dir })
  expect(state).toEqual({
    release: 'major',
    currentVersion: '1.2.3',
    currentVersionSource: 'package.json',
    newVersion: '2.0.0',
    commitMessage: '',
    tagName: '',
    updatedFiles: [],
    skippedFiles: [],
  })
})

it('next：按当前版本形状解析', async () => {
  const dir = initRepo()
  const { state } = await versionBumpInfo({ release: 'next', cwd: dir })
  expect(state.newVersion).toBe('1.2.4')
  expect(state.release).toBe('next')
})

it('conventional：依据提交推断（feat → minor）', async () => {
  const dir = initRepo()
  writeFileSync(join(dir, 'f'), 'x')
  git(dir, 'add .')
  git(dir, 'commit -m "feat: new thing"')
  const { state } = await versionBumpInfo({ release: 'conventional', cwd: dir })
  expect(state.newVersion).toBe('1.3.0')
  expect(state.release).toBe('conventional')
})

it('版本号字符串：loose 解析，release 为空', async () => {
  const dir = initRepo()
  const { state } = await versionBumpInfo({ release: 'v2.0', cwd: dir })
  expect(state.newVersion).toBe('2.0.0')
  expect(state.release).toBeUndefined()
})

it('currentVersion 选项跳过文件扫描', async () => {
  const dir = initRepo()
  const { state } = await versionBumpInfo({
    release: 'minor',
    currentVersion: '9.9.9',
    cwd: dir,
  })
  expect(state.currentVersion).toBe('9.9.9')
  expect(state.newVersion).toBe('9.10.0')
})

it('字符串入参等价于 { release }（上游 parity）', async () => {
  // npm/bumpp-core 自身 package.json 版本 5.1.0（process.cwd 为包目录）
  const { state } = await versionBumpInfo('minor')
  expect(state.currentVersion).toBe('5.1.0')
  expect(state.newVersion).toBe('5.2.0')
  expect(state.release).toBe('minor')
})

it('无法确定当前版本时报上游文案错误', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'bumpp-info-empty-'))
  dirs.push(dir)
  await expect(versionBumpInfo({ release: 'major', cwd: dir })).rejects.toThrowError(
    /Unable to determine the current version number/,
  )
})
