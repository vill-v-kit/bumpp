import { execSync } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, expect, it } from 'vitest'
import { versionBump } from './index.js'

let dirs: string[] = []

const git = (cwd: string, args: string): string =>
  execSync(`git ${args}`, { cwd, encoding: 'utf8' }).trim()

const initRepo = (): string => {
  const dir = mkdtempSync(join(tmpdir(), 'bumpp-vb-'))
  dirs.push(dir)
  git(dir, 'init -b main')
  git(dir, 'config user.email test@example.com')
  git(dir, 'config user.name Test')
  git(dir, 'config commit.gpgsign false')
  git(dir, 'config tag.gpgsign false')
  writeFileSync(
    join(dir, 'package.json'),
    '{\n  "version": "1.0.0",\n  "scripts": {\n    "preversion": "node -e \\"require(\'fs\').writeFileSync(\'pre.txt\',\'\')\\"",\n    "postversion": "node -e \\"require(\'fs\').writeFileSync(\'post.txt\',\'\')\\""\n  }\n}\n',
  )
  writeFileSync(join(dir, 'VERSION.txt'), 'version 1.0.0\n')
  git(dir, 'add .')
  git(dir, 'commit -m "chore: init"')
  return dir
}

afterEach(() => {
  dirs.forEach((dir) => rmSync(dir, { recursive: true, force: true }))
  dirs = []
})

it('全链路：文件更新 + scripts 时序 + commit/tag（进度内置打印，JS 无回调）', async () => {
  const dir = initRepo()
  let ticks = 0
  const probe = setInterval(() => ticks++, 5)
  const results = await versionBump({
    release: '2.0.0',
    files: ['package.json', 'VERSION.txt'],
    cwd: dir,
    commit: true,
    tag: true,
    push: false,
    confirm: false,
  })
  clearInterval(probe)

  // 文件已更新
  expect(readFileSync(join(dir, 'package.json'), 'utf8')).toContain('"version": "2.0.0"')
  expect(readFileSync(join(dir, 'VERSION.txt'), 'utf8')).toBe('version 2.0.0\n')
  // commit / tag
  expect(git(dir, 'log -1 --pretty=%s')).toBe('chore: release v2.0.0')
  expect(git(dir, 'tag -l')).toBe('v2.0.0')
  // scripts 按序执行
  expect(existsSync(join(dir, 'pre.txt'))).toBe(true)
  expect(existsSync(join(dir, 'post.txt'))).toBe(true)
  // results 形状不变
  expect(results.newVersion).toBe('2.0.0')
  expect(results.currentVersion).toBe('1.0.0')
  expect(results.commit).toBe('chore: release v2.0.0')
  expect(results.tag).toBe('v2.0.0')
  expect(results.updatedFiles).toHaveLength(2)
  expect(results.skippedFiles).toHaveLength(0)
  // 慢速 git 操作期间事件循环未被阻塞
  expect(ticks).toBeGreaterThan(0)
})

it('push 到远端，慢操作期间事件循环不被阻塞', async () => {
  const dir = initRepo()
  const bare = mkdtempSync(join(tmpdir(), 'bumpp-vb-bare-'))
  dirs.push(bare)
  git(dir, `init --bare ${bare}`)
  git(dir, `remote add origin ${bare}`)
  git(dir, 'push -u origin main')
  // 慢速 git 操作：pre-push hook 休眠 1s
  writeFileSync(join(dir, '.git/hooks/pre-push'), '#!/bin/sh\nsleep 1\n', { mode: 0o755 })
  let ticks = 0
  const probe = setInterval(() => ticks++, 50)
  await versionBump({
    release: '2.0.0',
    files: ['package.json'],
    cwd: dir,
    commit: true,
    tag: true,
    push: true,
    confirm: false,
  })
  clearInterval(probe)
  expect(git(bare, 'log -1 --pretty=%s main')).toBe('chore: release v2.0.0')
  expect(git(bare, 'tag -l')).toBe('v2.0.0')
  expect(ticks).toBeGreaterThan(5)
})

it('失败步骤拒绝且错误可读', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'bumpp-vb-fail-'))
  dirs.push(dir)
  writeFileSync(join(dir, 'package.json'), '{\n  "version": "1.0.0"\n}\n')
  await expect(
    versionBump({ release: '2.0.0', files: ['package.json'], cwd: dir, commit: true }),
  ).rejects.toThrowError(/not a git repository/)
})
