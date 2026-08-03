import { execSync } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, expect, it } from 'vitest'
import { generateChangelog } from './index.js'

let dirs: string[] = []

const git = (cwd: string, args: string): string =>
  execSync(`git ${args}`, { cwd, encoding: 'utf8' }).trim()

const initRepo = (): string => {
  const dir = mkdtempSync(join(tmpdir(), 'bumpp-changelog-'))
  dirs.push(dir)
  git(dir, 'init -b main')
  git(dir, 'config user.email test@example.com')
  git(dir, 'config user.name Test')
  git(dir, 'config commit.gpgsign false')
  writeFileSync(join(dir, 'f.txt'), 'init\n')
  git(dir, 'add .')
  git(dir, 'commit -m "chore: init"')
  git(dir, 'tag v1.0.0')
  git(dir, 'remote add origin git@github.com:owner/repo.git')
  writeFileSync(join(dir, 'a.txt'), 'a\n')
  git(dir, 'add .')
  git(dir, 'commit -m "feat(ui): add x (#12)"')
  return dir
}

afterEach(() => {
  dirs.forEach((dir) => rmSync(dir, { recursive: true, force: true }))
  dirs = []
})

it('generateChangelog 端到端：写盘 + 提交 + 返回结构', () => {
  const dir = initRepo()
  const result = generateChangelog({ from: 'v1.0.0', to: '1.1.0' }, dir)
  expect(result.markdown).toMatch(/^## v1\.1\.0/)
  expect(result.markdown).toContain('**ui:** Add x ([#12](https://github.com/owner/repo/pull/12))')
  expect(result.changelogMD).toBe(readFileSync(join(dir, 'CHANGELOG.md'), 'utf8'))
  expect(result.changelogMD.startsWith('# Changelog\n\n\n## v1.1.0')).toBe(true)
  expect(git(dir, 'log -1 --pretty=%s')).toBe('chore: update CHANGELOG.md')
})

it('generateChangelog overrides 透传 changelog 段生效', () => {
  const dir = initRepo()
  const result = generateChangelog(
    {
      from: 'v1.0.0',
      to: '1.1.0',
      overrides: { changelog: { commitMessage: 'docs: 更新 {{output}}', hideAuthorEmail: false } },
    },
    dir,
  )
  expect(git(dir, 'log -1 --pretty=%s')).toBe('docs: 更新 CHANGELOG.md')
  expect(result.markdown).toContain('- Test <test@example.com>')
  expect(existsSync(join(dir, 'CHANGELOG.md'))).toBe(true)
})
