import { execSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, expect, it } from 'vitest'
import { resolveConfig } from './config'

let dir: string
let prevCwd: string

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), 'vbumpp-config-'))
  prevCwd = process.cwd()
  // changelogen 的 loadChangelogConfig 需要 git 仓库（git tag --points-at HEAD）
  const git = (args: string) => execSync(`git ${args}`, { cwd: dir })
  git('init -b main')
  git('config user.email test@example.com')
  git('config user.name Test')
  git('config commit.gpgsign false')
  git('commit --allow-empty -m "chore: init"')
  process.chdir(dir)
})

afterEach(() => {
  process.chdir(prevCwd)
  rmSync(dir, { recursive: true, force: true })
})

it('-r 递归收集经 core 插件链模式表（node manifests + Cargo.toml，ADR-0003 opt-in）', async () => {
  const config = await resolveConfig({ bumpp: { recursive: true } })
  expect(config.bumpp.files).toContain('**/package.json')
  // basename 取磁盘惯例名（Cargo.toml 大写开头），Linux 大小写敏感 FS 上才能命中
  expect(config.bumpp.files).toContain('**/Cargo.toml')
  expect(config.bumpp.recursive).toBe(false)
})
