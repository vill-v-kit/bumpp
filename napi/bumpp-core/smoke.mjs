// napi 链路冒烟（COL-40）：全部 changelog 系导出 + 既有导出可调用且返回结构正确
import { execSync } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
  generateChangelog,
  getCurrentGitBranch,
  getGitDiff,
  getLastGitTag,
  loadBumpConfig,
  resolveRepoConfig,
  versionBumpInfo,
  versionFileManifestGlobs,
} from './index.js'

const dir = mkdtempSync(join(tmpdir(), 'bumpp-smoke-'))
const git = (args) => execSync(`git ${args}`, { cwd: dir, encoding: 'utf8' }).trim()
git('init -b main')
git('config user.email t@e.com')
git('config user.name T')
git('config commit.gpgsign false')
writeFileSync(
  join(dir, 'package.json'),
  JSON.stringify({ version: '1.0.0', repository: 'https://github.com/owner/repo.git' }),
)
git('add .')
git('commit -m "chore: init"')
git('tag v1.0.0')
writeFileSync(join(dir, 'f.txt'), 'x')
git('add .')
git('commit -m "feat: add x (#1)"')

const checks = [
  ['versionBumpInfo 导出存在', typeof versionBumpInfo === 'function'],
  ['loadBumpConfig 内建默认', loadBumpConfig().commit === true],
  ['getLastGitTag 真实 tag 名', getLastGitTag(dir) === 'v1.0.0'],
  // 须在 generateChangelog 前——后者会新增一条 changelog 提交
  ['getGitDiff 范围提交', getGitDiff('v1.0.0', undefined, dir).length === 1],
  [
    'generateChangelog 端到端',
    generateChangelog({ from: 'v1.0.0', to: '1.1.0' }, dir).markdown.startsWith('## v1.1.0'),
  ],
  ['getCurrentGitBranch', getCurrentGitBranch(dir) === 'main'],
  ['resolveRepoConfig 结构', resolveRepoConfig(dir).repo === 'owner/repo'],
  [
    'versionFileManifestGlobs 插件底座链',
    versionFileManifestGlobs().includes('**/Cargo.toml'),
  ],
  [
    'changelogMD 写盘一致',
    readFileSync(join(dir, 'CHANGELOG.md'), 'utf8').includes('Add x'),
  ],
]

let failed = 0
for (const [name, ok] of checks) {
  console.log(ok ? '✓' : '✗', name)
  if (!ok) failed++
}
rmSync(dir, { recursive: true, force: true })
process.exit(failed)
