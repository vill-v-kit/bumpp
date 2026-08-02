import { execSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, expect, it } from 'vitest'
import {
  getCurrentGitBranch,
  getGitDiff,
  getLastGitTag,
  gitCommit,
  gitPush,
  gitTag,
  resolveRepoConfig,
} from './index.js'

let dirs: string[] = []

const git = (cwd: string, args: string): string =>
  execSync(`git ${args}`, { cwd, encoding: 'utf8' }).trim()

const initRepo = (): string => {
  const dir = mkdtempSync(join(tmpdir(), 'bumpp-git-'))
  dirs.push(dir)
  git(dir, 'init -b main')
  git(dir, 'config user.email test@example.com')
  git(dir, 'config user.name Test')
  git(dir, 'config commit.gpgsign false')
  git(dir, 'config tag.gpgsign false')
  writeFileSync(join(dir, 'package.json'), '{\n  "version": "1.0.0"\n}\n')
  git(dir, 'add .')
  git(dir, 'commit -m init')
  return dir
}

afterEach(() => {
  dirs.forEach((dir) => rmSync(dir, { recursive: true, force: true }))
  dirs = []
})

it('git commit 真实发生，%s 模板替换，返回事件与信息', () => {
  const dir = initRepo()
  writeFileSync(join(dir, 'a.txt'), 'a')
  git(dir, 'add .')
  const outcome = gitCommit(dir, {
    updatedFiles: ['a.txt'],
    all: false,
    noVerify: false,
    sign: false,
    message: 'chore: release v%s',
    newVersion: '2.0.0',
  })
  expect(outcome.event).toBe('git commit')
  expect(outcome.commitMessage).toBe('chore: release v2.0.0')
  expect(git(dir, 'log -1 --pretty=%s')).toBe('chore: release v2.0.0')
})

it('noVerify 跳过 pre-commit hook，否则提交失败', () => {
  const dir = initRepo()
  writeFileSync(join(dir, '.git/hooks/pre-commit'), '#!/bin/sh\nexit 1\n', { mode: 0o755 })
  const spec = {
    updatedFiles: [] as string[],
    all: false,
    sign: false,
    message: 'release v%s',
    newVersion: '2.0.0',
  }
  expect(() => gitCommit(dir, { ...spec, noVerify: false })).toThrowError(/git commit/)
  const outcome = gitCommit(dir, { ...spec, noVerify: true })
  expect(outcome.commitMessage).toBe('release v2.0.0')
})

it('git tag 附注与 %s 模板', () => {
  const dir = initRepo()
  const outcome = gitTag(dir, {
    name: 'v%s',
    message: 'chore: release v%s',
    sign: false,
    newVersion: '2.0.0',
  })
  expect(outcome.event).toBe('git tag')
  expect(outcome.tagName).toBe('v2.0.0')
  expect(git(dir, 'tag -l')).toBe('v2.0.0')
  expect(git(dir, "for-each-ref refs/tags/v2.0.0 --format='%(contents)'")).toContain(
    'chore: release v2.0.0',
  )
})

it('git push 推送提交与 tag 到远端', () => {
  const dir = initRepo()
  const bare = mkdtempSync(join(tmpdir(), 'bumpp-bare-'))
  dirs.push(bare)
  git(dir, `init --bare ${bare}`)
  git(dir, `remote add origin ${bare}`)
  git(dir, 'push -u origin main')
  writeFileSync(join(dir, 'a.txt'), 'a')
  git(dir, 'add .')
  gitCommit(dir, {
    updatedFiles: ['a.txt'],
    all: false,
    noVerify: false,
    sign: false,
    message: 'release v%s',
    newVersion: '2.0.0',
  })
  gitTag(dir, { name: 'v%s', message: 'release v%s', sign: false, newVersion: '2.0.0' })
  const outcome = gitPush(dir, true)
  expect(outcome.event).toBe('git push')
  expect(git(bare, 'log -1 --pretty=%s main')).toBe('release v2.0.0')
  expect(git(bare, 'tag -l')).toBe('v2.0.0')
})

it('git 命令失败时错误含 stderr', () => {
  const dir = mkdtempSync(join(tmpdir(), 'bumpp-notrepo-'))
  dirs.push(dir)
  expect(() =>
    gitCommit(dir, {
      updatedFiles: [],
      all: false,
      noVerify: false,
      sign: false,
      message: 'release v%s',
      newVersion: '2.0.0',
    }),
  ).toThrowError(/not a git repository/)
})

it('getLastGitTag 返回真实 tag 名，无 tag / 非仓库返回 null', () => {
  const dir = initRepo()
  expect(getLastGitTag(dir)).toBeNull()
  git(dir, 'tag v1.0.0')
  expect(getLastGitTag(dir)).toBe('v1.0.0')
  const notRepo = mkdtempSync(join(tmpdir(), 'bumpp-notrepo-'))
  dirs.push(notRepo)
  expect(getLastGitTag(notRepo)).toBeNull()
})

it('getCurrentGitBranch 返回当前分支名', () => {
  const dir = initRepo()
  expect(getCurrentGitBranch(dir)).toBe('main')
})

it('getGitDiff 返回范围内提交（新→旧，含 author 与 body）', () => {
  const dir = initRepo()
  git(dir, 'tag v1.0.0')
  writeFileSync(join(dir, 'a.txt'), 'a')
  git(dir, 'add .')
  git(dir, 'commit -m "feat: add a"')
  writeFileSync(join(dir, 'b.txt'), 'b')
  git(dir, 'add .')
  git(dir, 'commit -m "fix: b" -m "BREAKING CHANGE: b breaks"')
  const commits = getGitDiff('v1.0.0', undefined, dir)
  expect(commits).toHaveLength(2)
  expect(commits[0].message).toBe('fix: b')
  expect(commits[0].shortHash).toBe(git(dir, 'log -1 --pretty=%h'))
  expect(commits[0].author).toEqual({ name: 'Test', email: 'test@example.com' })
  expect(commits[0].body).toContain('BREAKING CHANGE: b breaks')
  expect(commits[1].message).toBe('feat: add a')
})

it('resolveRepoConfig：package.json repository 优先，无源返回 null', () => {
  const dir = initRepo()
  expect(resolveRepoConfig(dir)).toBeNull()
  writeFileSync(
    join(dir, 'package.json'),
    '{ "repository": "git@github.com:owner/repo.git" }',
  )
  expect(resolveRepoConfig(dir)).toEqual({
    provider: 'github',
    domain: 'github.com',
    repo: 'owner/repo',
  })
})
