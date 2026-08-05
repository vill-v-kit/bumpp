/**
 * npm-publish.mjs 的 CLI 契约测试（COL-53）。
 * Seam：CLI 契约本身——spawn 真实脚本进程（--dry-run 不上传），经
 * PUBLISH_GUARD_NPM_URL 把 publish-guard 查询与 pnpm publish 的 registry
 * 检查都导向本地 stub，验证「守卫过滤 → 放行上架」全链路的构成。
 */
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { createServer } from 'node:http'
import { execFile } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const script = fileURLToPath(new URL('./npm-publish.mjs', import.meta.url))

// 当前仓库的 11 个可上架包（website 与根 monorepo 均 private，永不出现在放行集）；
// 新增发版包时此清单需同步——恰好全覆盖是 COL-53 的验收项
const PUBLISHABLE = [
  '@vill-v/bumpp-core',
  '@vill-v/bumpp-core-darwin-arm64',
  '@vill-v/bumpp-core-linux-arm64-gnu',
  '@vill-v/bumpp-core-linux-x64-gnu',
  '@vill-v/bumpp-core-win32-arm64-msvc',
  '@vill-v/bumpp-core-win32-x64-msvc',
  '@vill-v/bumpp',
  '@vill-v/bumpp-gitcode',
  '@vill-v/bumpp-gitee',
  '@vill-v/bumpp-github',
  '@vill-v/bumpp-gitlab',
]

let server
let base
// 每个测试替换此 handler 来模拟 registry 行为
let handler = (_req, res) => res.writeHead(500).end()

beforeAll(async () => {
  server = createServer((req, res) => handler(req, res))
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
  base = `http://127.0.0.1:${server.address().port}`
})

afterAll(() => server.close())

function runPublish() {
  return new Promise((resolve) => {
    execFile(
      'node',
      [script, '--dry-run'],
      { env: { ...process.env, PUBLISH_GUARD_NPM_URL: base } },
      (error, stdout, stderr) => {
        resolve({ code: error ? error.code : 0, stdout, stderr })
      },
    )
  })
}

// pnpm publish --dry-run 输出的「📦 <name>@<version> → …」提取为包名集合
function dryRunPackages(stdout) {
  return [...stdout.matchAll(/📦 (\S+?)@/g)].map((m) => m[1]).sort()
}

describe('守卫过滤 → 放行上架', () => {
  it('全部未上架（404）→ 恰好 11 个包进入 publish', { timeout: 60000 }, async () => {
    handler = (_req, res) => res.writeHead(404).end()
    const r = await runPublish()
    expect(r.code).toBe(0)
    expect(dryRunPackages(r.stdout)).toEqual([...PUBLISHABLE].sort())
  })

  it('部分已上架 → publish 只含未上架的，SKIP 行进 stderr', { timeout: 60000 }, async () => {
    const published = ['@vill-v/bumpp', '@vill-v/bumpp-core-darwin-arm64']
    handler = (req, res) => {
      const name = decodeURIComponent(req.url.split('/')[1])
      res.writeHead(published.includes(name) ? 200 : 404).end()
    }
    const r = await runPublish()
    expect(r.code).toBe(0)
    expect(dryRunPackages(r.stdout)).toEqual(PUBLISHABLE.filter((n) => !published.includes(n)).sort())
    for (const p of published) {
      expect(r.stderr).toContain(`SKIP ${p}@`)
    }
  })

  it('全部已上架（200）→ 不调用 pnpm publish 直接收工', { timeout: 60000 }, async () => {
    handler = (_req, res) => res.writeHead(200).end('{}')
    const r = await runPublish()
    expect(r.code).toBe(0)
    expect(r.stdout).not.toContain('📦')
    expect(r.stdout + r.stderr).toMatch(/nothing to do|全部已上架/)
  })
})

describe('查询失败原子性', () => {
  it('任一包查询失败（500）→ exit 2 且不发起 publish', { timeout: 60000 }, async () => {
    handler = (req, res) => {
      const name = decodeURIComponent(req.url.split('/')[1])
      res.writeHead(name === '@vill-v/bumpp-gitlab' ? 500 : 404).end()
    }
    const r = await runPublish()
    expect(r.code).toBe(2)
    expect(r.stdout).not.toContain('📦')
    expect(r.stderr).toContain('ERROR')
  })
})
