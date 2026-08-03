// golden fixture 一次性生成脚本（dev-only，ADR-0012）：
// 以真 changelogen 0.6.2 在合成 git 仓库产出 markdown，施加三处申报偏差
// 等效变换后固化。重生成：node crates/bumpp-core/tests/fixtures/changelog-gen.mjs
// 变换清单：
//   ① '#### ⚠️ Breaking Changes' → '#### 🚨 破坏性改动'（types.BreakingChange.title）
//   ② '### ❤️ Contributors' → '### ❤️ 贡献者'（硬编码中文节头）
//   ③ 贡献者行剥除 ungh.cc 解析结果 ` ([@user](https://github.com/user))`（网络杀除）
// 另：生成时 hideAuthorEmail: true（本实现默认翻转）；chore(deps) 过滤同原 JS。
import { execSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { generateMarkDown, getGitDiff, parseCommits } from '../../../../node_modules/.pnpm/changelogen@0.6.2/node_modules/changelogen/dist/index.mjs'

const OUT = new URL('./changelog/', import.meta.url).pathname

const git = (cwd, args) => execSync(`git ${args}`, { cwd, encoding: 'utf8' })

// 内建默认（ADR-0013）：原 JS getDefaultsChangeLogConfig 同形
const TYPES = {
  feat: { title: '🚀 特性' },
  perf: { title: '🔥 性能优化' },
  fix: { title: '🩹 修复' },
  refactor: { title: '💅 重构' },
  examples: { title: '🏀 示例' },
  docs: { title: '📖 文档' },
  chore: { title: '🏡 框架' },
  build: { title: '📦 打包' },
  test: { title: '✅ 测试' },
  BreakingChange: { title: '🚨 破坏性改动' },
  style: { title: '🎨 样式' },
}

const AUTHORS = {
  alice: ['Alice Dev', 'alice@example.com'],
  bob: ['bob smith', 'bob@example.com'],
  carol: ['carol', 'carol@example.com'],
  dave: ['dave', 'dave@example.com'],
  bot: ['dependabot[bot]', 'bot@example.com'],
}

function initRepo() {
  const dir = mkdtempSync(join(tmpdir(), 'changelog-golden-'))
  git(dir, 'init -b main')
  git(dir, 'config commit.gpgsign false')
  git(dir, 'config tag.gpgsign false')
  return dir
}

function commit(dir, author, message, extra = []) {
  const [name, email] = AUTHORS[author]
  const env = `GIT_AUTHOR_NAME="${name}" GIT_AUTHOR_EMAIL="${email}" GIT_COMMITTER_NAME="${name}" GIT_COMMITTER_EMAIL="${email}"`
  execSync(`touch ${join(dir, 'f.txt')} && echo "${Date.now()}${Math.random()}" >> ${join(dir, 'f.txt')}`, { cwd: dir })
  git(dir, 'add .')
  const msgArgs = ['-m', ...[message].flat(), ...extra].map((m) => `"${m}"`).join(' ')
  execSync(`${env} git commit ${msgArgs}`, { cwd: dir })
}

function applyTransforms(md) {
  return md
    .replace('#### ⚠️ Breaking Changes', '#### 🚨 破坏性改动')
    .replace('### ❤️ Contributors', '### ❤️ 贡献者')
    .replace(/ \(\[@([^\]]+)\]\(https:\/\/github\.com\/[^)]+\)\)/g, '')
}

async function capture(name, { from, newVersion, to }, commitsFn) {
  const dir = initRepo()
  commit(dir, 'alice', 'chore: init')
  git(dir, `tag ${from}`)
  commitsFn(dir)
  const rawCommits = await getGitDiff(from, 'HEAD', dir)
  const config = {
    cwd: dir,
    types: TYPES,
    repo: { provider: 'github', domain: 'github.com', repo: 'owner/repo' },
    from,
    to,
    newVersion,
    output: 'CHANGELOG.md',
    scopeMap: {},
    excludeAuthors: [],
    noAuthors: false,
    hideAuthorEmail: true,
    templates: { tagBody: 'v{{newVersion}}' },
  }
  const parsed = parseCommits(rawCommits, config).filter(
    (c) => config.types[c.type] && !(c.type === 'chore' && c.scope === 'deps' && !c.isBreaking),
  )
  const markdown = applyTransforms(await generateMarkDown(parsed, config))
  mkdirSync(join(OUT, name), { recursive: true })
  writeFileSync(
    join(OUT, name, 'input.json'),
    JSON.stringify({ rawCommits, from, to, newVersion, repo: config.repo }, null, 2),
  )
  writeFileSync(join(OUT, name, 'expected.md'), markdown)
  writeFileSync(
    join(OUT, name, 'NOTES.md'),
    [
      `# fixture ${name}`,
      '',
      '- 出处：changelogen@0.6.2（generateMarkDown）在合成 git 仓库的真实产出',
      `- 生成：tests/fixtures/changelog-gen.mjs（dev-only）`,
      '- 变换：① Breaking 节标题中文化 ② 贡献者节头中文化 ③ 剥除 ungh.cc @username 链接',
      '- 生成配置：hideAuthorEmail: true（本实现默认翻转）；chore(deps) 过滤同原 JS',
      '',
    ].join('\n'),
  )
  rmSync(dir, { recursive: true, force: true })
  console.log(`fixture ${name} written`)
}

await capture('full', { from: 'v1.0.0', to: 'v1.2.0', newVersion: '1.2.0' }, (dir) => {
  commit(dir, 'alice', 'feat(ui): add dashboard (#123)')
  commit(dir, 'bob', 'fix: repair crash #45')
  commit(dir, 'alice', 'feat!: drop legacy api')
  commit(dir, 'bot', 'chore(deps): bump serde from 1 to 2')
  commit(dir, 'alice', 'chore(deps)!: bump tokio')
  commit(dir, 'carol', 'docs: update guide')
  commit(dir, 'dave', 'random non-conventional message')
  commit(dir, 'alice', 'feat: add :tada: celebrations')
  commit(dir, 'alice', 'chore: housekeeping')
})

await capture('minimal', { from: 'v0.1.0', to: 'v0.2.0', newVersion: '0.2.0' }, (dir) => {
  commit(dir, 'alice', 'feat: initial feature')
})
